use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use sherpa_onnx::{
    OfflineQwen3ASRModelConfig, OfflineRecognizer, OfflineRecognizerConfig, SileroVadModelConfig,
    VadModelConfig, VoiceActivityDetector,
};

use super::segment::SegmentBuilder;
use super::{AsrEngine, AsrError, EngineCaps, TARGET_SAMPLE_RATE};
use crate::config::AsrConfig;
use crate::i18n::{tf, tr, Key};
use crate::models::catalog::CatalogEntry;
use crate::transcript::TranscriptSegment;

const VAD_WINDOW: usize = 512;
const FLUSH_TIMEOUT: Duration = Duration::from_secs(60);

enum WorkerCommand {
    Start {
        recognizer: OfflineRecognizer,
        results: Sender<DecodeResult>,
    },
    Decode {
        utterance: i64,
        samples: Vec<f32>,
        start_ms: u64,
        end_ms: u64,
    },
    Flush,
    Stop,
}

enum DecodeResult {
    Segment {
        utterance: i64,
        text: String,
        start_ms: u64,
        end_ms: u64,
    },
    Error(String),
    Done,
}

static WORKER_TX: OnceLock<Sender<WorkerCommand>> = OnceLock::new();

pub fn init_worker() -> Result<(), String> {
    if WORKER_TX.get().is_some() {
        return Ok(());
    }

    let (tx, rx) = std::sync::mpsc::channel::<WorkerCommand>();
    std::thread::Builder::new()
        .name("asr-offline-worker".into())
        .spawn(move || worker_loop(rx))
        .map_err(|e| tf(Key::AsrOfflineWorkerStartFailed, &[&e.to_string()]))?;
    WORKER_TX
        .set(tx)
        .map_err(|_| tr(Key::AsrOfflineWorkerStartFailed).to_string())?;
    Ok(())
}

fn worker_loop(rx: Receiver<WorkerCommand>) {
    let mut recognizer: Option<OfflineRecognizer> = None;
    let mut results: Option<Sender<DecodeResult>> = None;

    while let Ok(command) = rx.recv() {
        match command {
            WorkerCommand::Start {
                recognizer: next,
                results: out,
            } => {
                recognizer = Some(next);
                results = Some(out);
            }
            WorkerCommand::Decode {
                utterance,
                samples,
                start_ms,
                end_ms,
            } => {
                let Some(out) = results.as_ref() else {
                    continue;
                };
                let Some(recognizer) = recognizer.as_ref() else {
                    let _ = out.send(DecodeResult::Error(tr(Key::AsrNotInitialized).to_string()));
                    continue;
                };

                let stream = recognizer.create_stream();
                stream.accept_waveform(TARGET_SAMPLE_RATE as i32, &samples);
                recognizer.decode(&stream);
                let Some(result) = stream.get_result() else {
                    continue;
                };
                let text = result.text.trim();
                if text.is_empty() {
                    continue;
                }
                let _ = out.send(DecodeResult::Segment {
                    utterance,
                    text: crate::asr::latin_case::restore_sentence_case(text),
                    start_ms,
                    end_ms,
                });
            }
            WorkerCommand::Flush => {
                if let Some(out) = results.as_ref() {
                    let _ = out.send(DecodeResult::Done);
                }
            }
            WorkerCommand::Stop => {
                recognizer = None;
                results = None;
            }
        }
    }
}

pub struct Qwen3OfflineAdapter {
    vad: Option<VoiceActivityDetector>,
    vad_path: PathBuf,
    entry: &'static CatalogEntry,
    commands: Option<Sender<WorkerCommand>>,
    results: Option<Receiver<DecodeResult>>,
    builder: SegmentBuilder,
    pending: Vec<f32>,
    samples_fed: u64,
    utterance: i64,
    flushed: bool,
}

impl Qwen3OfflineAdapter {
    pub fn new(vad_path: PathBuf, entry: &'static CatalogEntry) -> Self {
        Self {
            vad: None,
            vad_path,
            entry,
            commands: None,
            results: None,
            builder: SegmentBuilder::new(),
            pending: Vec::new(),
            samples_fed: 0,
            utterance: 0,
            flushed: false,
        }
    }

    fn require(path: &Path, exists: fn(&Path) -> bool) -> Result<(), AsrError> {
        if exists(path) {
            return Ok(());
        }
        Err(AsrError::MissingModelFile(tf(
            Key::AsrMissingFile,
            &[
                &path.display().to_string(),
                &listing(path.parent().unwrap_or_else(|| Path::new("."))),
            ],
        )))
    }

    fn feed_vad(&mut self, pcm: &[f32]) -> Result<(), AsrError> {
        self.samples_fed += pcm.len() as u64;
        self.pending.extend_from_slice(pcm);

        let vad = self.vad.as_ref().ok_or(AsrError::NotInitialized)?;
        let mut offset = 0;
        while self.pending.len() - offset >= VAD_WINDOW {
            vad.accept_waveform(&self.pending[offset..offset + VAD_WINDOW]);
            offset += VAD_WINDOW;
        }
        self.pending.drain(..offset);
        Ok(())
    }

    fn queue_segments(&mut self) -> Result<(), AsrError> {
        let mut found = Vec::new();
        {
            let vad = self.vad.as_ref().ok_or(AsrError::NotInitialized)?;
            while let Some(segment) = vad.front() {
                let samples = segment.samples().to_vec();
                let len = segment.n().max(0) as u64;
                drop(segment);
                vad.pop();
                found.push((samples, len));
            }
        }
        if found.is_empty() {
            return Ok(());
        }

        let end_ms = self.samples_fed * 1000 / TARGET_SAMPLE_RATE as u64;
        let commands = self.commands.clone().ok_or(AsrError::NotInitialized)?;
        for (samples, len) in found {
            let start_ms = end_ms.saturating_sub(len * 1000 / TARGET_SAMPLE_RATE as u64);
            commands
                .send(WorkerCommand::Decode {
                    utterance: self.utterance,
                    samples,
                    start_ms,
                    end_ms,
                })
                .map_err(|_| AsrError::ModelLoad(tr(Key::AsrOfflineWorkerMissing).into()))?;
            self.utterance += 1;
        }
        Ok(())
    }

    fn apply_results(
        &mut self,
        items: Vec<DecodeResult>,
    ) -> Result<Vec<TranscriptSegment>, AsrError> {
        let mut out = Vec::new();
        for item in items {
            match item {
                DecodeResult::Segment {
                    utterance,
                    text,
                    start_ms,
                    end_ms,
                } => {
                    out.push(self.builder.build(utterance, &text, start_ms, end_ms, true));
                    self.builder.forget(utterance);
                }
                DecodeResult::Error(e) => return Err(AsrError::ModelLoad(e)),
                DecodeResult::Done => {}
            }
        }
        Ok(out)
    }

    fn drain_results(&mut self) -> Result<Vec<TranscriptSegment>, AsrError> {
        let Some(rx) = self.results.take() else {
            return Ok(Vec::new());
        };
        let mut items = Vec::new();
        let mut disconnected = false;
        loop {
            match rx.try_recv() {
                Ok(item) => items.push(item),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    disconnected = true;
                    break;
                }
            }
        }
        if !disconnected {
            self.results = Some(rx);
        }
        self.apply_results(items)
    }

    fn wait_for_flush(&mut self) -> Result<Vec<TranscriptSegment>, AsrError> {
        let Some(rx) = self.results.take() else {
            return Ok(Vec::new());
        };
        let deadline = Instant::now() + FLUSH_TIMEOUT;
        let mut items = Vec::new();
        let mut done = false;
        while !done {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                self.results = Some(rx);
                return Err(AsrError::ModelLoad(tr(Key::AsrOfflineWaitTimeout).into()));
            }
            match rx.recv_timeout(remaining) {
                Ok(DecodeResult::Done) => done = true,
                Ok(item) => items.push(item),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    self.results = Some(rx);
                    return Err(AsrError::ModelLoad(tr(Key::AsrOfflineWaitTimeout).into()));
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    done = true;
                }
            }
        }
        self.results = Some(rx);
        self.apply_results(items)
    }
}

impl AsrEngine for Qwen3OfflineAdapter {
    fn init(&mut self, cfg: &AsrConfig) -> Result<(), AsrError> {
        let dir = cfg
            .model_dir
            .as_ref()
            .ok_or_else(|| AsrError::InvalidConfig(tr(Key::AsrModelDirNotSet).into()))?;
        let dir = Path::new(dir);
        let req = self
            .entry
            .required
            .asr
            .as_ref()
            .expect("目录校验保证 qwen3 档包含 required.asr");

        let conv_frontend = dir.join(
            req.conv_frontend
                .as_deref()
                .expect("目录校验保证 convFrontend"),
        );
        let encoder = dir.join(req.encoder.as_deref().expect("目录校验保证 encoder"));
        let decoder = dir.join(req.decoder.as_deref().expect("目录校验保证 decoder"));
        let tokenizer = dir.join(
            req.tokenizer_dir
                .as_deref()
                .expect("目录校验保证 tokenizerDir"),
        );

        Self::require(&conv_frontend, Path::is_file)?;
        Self::require(&encoder, Path::is_file)?;
        Self::require(&decoder, Path::is_file)?;
        Self::require(&tokenizer, Path::is_dir)?;
        Self::require(&self.vad_path, Path::is_file)?;

        let mut config = OfflineRecognizerConfig::default();
        config.model_config.qwen3_asr = OfflineQwen3ASRModelConfig {
            conv_frontend: Some(conv_frontend.to_string_lossy().into_owned()),
            encoder: Some(encoder.to_string_lossy().into_owned()),
            decoder: Some(decoder.to_string_lossy().into_owned()),
            tokenizer: Some(tokenizer.to_string_lossy().into_owned()),
            max_new_tokens: 256,
            ..Default::default()
        };
        config.model_config.provider = Some("cpu".into());
        config.model_config.num_threads = 2;

        let recognizer = OfflineRecognizer::create(&config).ok_or_else(|| {
            AsrError::ModelLoad(tf(
                Key::AsrOfflineRecognizerNone,
                &[&dir.display().to_string()],
            ))
        })?;

        let vad_config = VadModelConfig {
            silero_vad: SileroVadModelConfig {
                model: Some(self.vad_path.to_string_lossy().into_owned()),
                threshold: 0.5,
                min_silence_duration: cfg.endpoint.trailing_silence_ms as f32 / 1000.0,
                min_speech_duration: 0.25,
                window_size: VAD_WINDOW as i32,
                max_speech_duration: cfg.endpoint.max_utterance_ms as f32 / 1000.0,
            },
            sample_rate: TARGET_SAMPLE_RATE as i32,
            num_threads: 1,
            provider: Some("cpu".into()),
            ..Default::default()
        };
        let vad = VoiceActivityDetector::create(&vad_config, 60.0).ok_or_else(|| {
            AsrError::ModelLoad(tf(
                Key::AsrVadCreateNone,
                &[&self.vad_path.display().to_string()],
            ))
        })?;

        let commands = WORKER_TX
            .get()
            .cloned()
            .ok_or_else(|| AsrError::ModelLoad(tr(Key::AsrOfflineWorkerMissing).into()))?;
        let (results_tx, results_rx) = std::sync::mpsc::channel();
        commands
            .send(WorkerCommand::Start {
                recognizer,
                results: results_tx,
            })
            .map_err(|_| AsrError::ModelLoad(tr(Key::AsrOfflineWorkerMissing).into()))?;

        self.vad = Some(vad);
        self.commands = Some(commands);
        self.results = Some(results_rx);
        self.pending.clear();
        self.samples_fed = 0;
        self.utterance = 0;
        self.flushed = false;
        Ok(())
    }

    fn capabilities(&self) -> EngineCaps {
        EngineCaps::OFFLINE_QWEN3
    }

    fn feed_pcm(&mut self, pcm: &[f32]) -> Result<Vec<TranscriptSegment>, AsrError> {
        self.feed_vad(pcm)?;
        self.queue_segments()?;
        self.drain_results()
    }

    fn flush(&mut self) -> Result<Vec<TranscriptSegment>, AsrError> {
        if self.flushed {
            return Ok(Vec::new());
        }
        self.flushed = true;

        if !self.pending.is_empty() {
            let mut window = std::mem::take(&mut self.pending);
            window.resize(VAD_WINDOW, 0.0);
            if let Some(vad) = &self.vad {
                vad.accept_waveform(&window);
            }
        }
        if let Some(vad) = &self.vad {
            vad.flush();
        }
        self.queue_segments()?;

        if let Some(commands) = &self.commands {
            commands
                .send(WorkerCommand::Flush)
                .map_err(|_| AsrError::ModelLoad(tr(Key::AsrOfflineWorkerMissing).into()))?;
        } else {
            return Ok(Vec::new());
        }
        self.wait_for_flush()
    }

    fn shutdown(&mut self) {
        if let Some(commands) = &self.commands {
            let _ = commands.send(WorkerCommand::Stop);
        }
        self.vad = None;
        self.commands = None;
        self.results = None;
        self.pending.clear();
    }
}

fn listing(dir: &Path) -> String {
    crate::paths::file_names(dir)
        .map(|names| names.join(", "))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn qwen3_transcribes_a_wav() {
        let model_dir = std::env::var("QWEN3_MODEL_DIR").expect("QWEN3_MODEL_DIR");
        let vad_path = std::env::var("QWEN3_VAD_MODEL").expect("QWEN3_VAD_MODEL");
        let wav_path = std::env::var("QWEN3_WAV").expect("QWEN3_WAV");
        let wave = sherpa_onnx::Wave::read(&wav_path).expect("读取 wav");

        init_worker().unwrap();
        let entry = crate::models::catalog::entry("asr-qwen3").unwrap();
        let mut engine = Qwen3OfflineAdapter::new(PathBuf::from(vad_path), entry);
        engine
            .init(&AsrConfig {
                model_dir: Some(model_dir),
                ..Default::default()
            })
            .unwrap();

        let mut segments = Vec::new();
        for chunk in wave.samples().chunks(1600) {
            segments.extend(engine.feed_pcm(chunk).unwrap());
        }
        segments.extend(engine.flush().unwrap());

        let text = segments
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        println!("{text}");
        assert!(!text.is_empty(), "没有识别结果");
    }
}
