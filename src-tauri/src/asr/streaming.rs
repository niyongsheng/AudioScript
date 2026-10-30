use std::path::{Path, PathBuf};

use sherpa_onnx::{OnlineRecognizer, OnlineRecognizerConfig, OnlineStream};

use super::segment::SegmentBuilder;
use super::{AsrEngine, AsrError, EngineCaps, TARGET_SAMPLE_RATE};
use crate::config::AsrConfig;
use crate::i18n::{tf, tr, Key};
use crate::models::catalog::CatalogEntry;
use crate::transcript::TranscriptSegment;

const TAIL_SILENCE_SECONDS: f32 = 0.3;

#[derive(Debug, Clone)]
pub struct ModelFiles {
    pub encoder: PathBuf,
    pub decoder: PathBuf,
    pub joiner: PathBuf,
    pub tokens: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Precision {
    Int8,
    Fp32,
}

impl Precision {
    fn parse(value: Option<&str>) -> Option<Self> {
        match value {
            Some("int8") => Some(Self::Int8),
            Some("fp32") => Some(Self::Fp32),
            _ => None,
        }
    }

    fn matches(self, name: &str) -> bool {
        match self {
            Precision::Int8 => name.contains(".int8."),
            Precision::Fp32 => !name.contains(".int8."),
        }
    }
}

fn discover_model_files(dir: &Path, entry: &CatalogEntry) -> Result<ModelFiles, AsrError> {
    let asr = entry.required.asr.as_ref();
    let encoder_pref =
        Precision::parse(asr.and_then(|a| a.encoder.as_deref())).unwrap_or(Precision::Int8);
    let decoder_pref =
        Precision::parse(asr.and_then(|a| a.decoder.as_deref())).unwrap_or(Precision::Fp32);
    let joiner_pref =
        Precision::parse(asr.and_then(|a| a.joiner.as_deref())).unwrap_or(Precision::Int8);

    let entries = std::fs::read_dir(dir).map_err(|e| {
        AsrError::ModelLoad(tf(
            Key::AsrReadModelDirFailed,
            &[&dir.display().to_string(), &e.to_string()],
        ))
    })?;

    let names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();

    let pick = |prefix: &str, prefer: Precision| -> Result<PathBuf, AsrError> {
        let candidates: Vec<&String> = names
            .iter()
            .filter(|n| n.starts_with(prefix) && n.ends_with(".onnx"))
            .collect();

        let chosen = candidates
            .iter()
            .find(|n| prefer.matches(n))
            .or_else(|| candidates.first())
            .ok_or_else(|| {
                AsrError::MissingModelFile(tf(
                    Key::AsrMissingPattern,
                    &[
                        &dir.display().to_string(),
                        prefix,
                        &if names.is_empty() {
                            tr(Key::AsrEmptyFolder).to_string()
                        } else {
                            names.join(", ")
                        },
                    ],
                ))
            })?;

        Ok(dir.join(chosen.as_str()))
    };

    let tokens = dir.join("tokens.txt");
    if !tokens.is_file() {
        return Err(AsrError::MissingModelFile(tf(
            Key::AsrMissingFile,
            &[&tokens.display().to_string(), &names.join(", ")],
        )));
    }

    Ok(ModelFiles {
        encoder: pick("encoder", encoder_pref)?,
        decoder: pick("decoder", decoder_pref)?,
        joiner: pick("joiner", joiner_pref)?,
        tokens,
    })
}

pub struct StreamingAdapter {
    entry: &'static CatalogEntry,
    recognizer: Option<OnlineRecognizer>,
    stream: Option<OnlineStream>,
    builder: SegmentBuilder,
    utterance: i64,
    samples_fed: u64,
    utterance_start_ms: u64,
    flushed: bool,
}

impl StreamingAdapter {
    pub fn new(entry: &'static CatalogEntry) -> Self {
        Self {
            entry,
            recognizer: None,
            stream: None,
            builder: SegmentBuilder::new(),
            utterance: 0,
            samples_fed: 0,
            utterance_start_ms: 0,
            flushed: false,
        }
    }

    fn now_ms(&self) -> u64 {
        self.samples_fed * 1000 / TARGET_SAMPLE_RATE as u64
    }

    fn strm(&self) -> Result<&OnlineStream, AsrError> {
        self.stream.as_ref().ok_or(AsrError::NotInitialized)
    }

    fn drain(&mut self, out: &mut Vec<TranscriptSegment>) -> Result<(), AsrError> {
        let rec = self.recognizer.as_ref().ok_or(AsrError::NotInitialized)?;
        let stream = self.stream.as_ref().ok_or(AsrError::NotInitialized)?;

        let mut pending: Option<(i64, String)> = None;

        while rec.is_ready(stream) {
            rec.decode(stream);

            if let Some(r) = rec.get_result(stream) {
                if !r.text.trim().is_empty() {
                    pending = Some((self.utterance, r.text));
                }
            }

            if rec.is_endpoint(stream) {
                if let Some((utt, text)) = pending.take() {
                    let text = crate::asr::latin_case::restore_sentence_case(&text);
                    let end = self.now_ms();
                    out.push(
                        self.builder
                            .build(utt, &text, self.utterance_start_ms, end, true),
                    );
                    self.builder.forget(utt);
                }
                rec.reset(stream);
                self.utterance += 1;
                self.utterance_start_ms = self.now_ms();
            }
        }

        if let Some((utt, text)) = pending {
            let text = crate::asr::latin_case::restore_sentence_case(&text);
            let end = self.now_ms();
            out.push(
                self.builder
                    .build(utt, &text, self.utterance_start_ms, end, false),
            );
        }

        Ok(())
    }
}

impl AsrEngine for StreamingAdapter {
    fn init(&mut self, cfg: &AsrConfig) -> Result<(), AsrError> {
        let dir = cfg
            .model_dir
            .as_ref()
            .ok_or_else(|| AsrError::InvalidConfig(tr(Key::AsrModelDirNotSet).into()))?;
        let files = discover_model_files(Path::new(dir), self.entry)?;

        let mut config = OnlineRecognizerConfig::default();

        config.model_config.transducer.encoder = Some(files.encoder.to_string_lossy().into_owned());
        config.model_config.transducer.decoder = Some(files.decoder.to_string_lossy().into_owned());
        config.model_config.transducer.joiner = Some(files.joiner.to_string_lossy().into_owned());
        config.model_config.tokens = Some(files.tokens.to_string_lossy().into_owned());
        config.model_config.provider = Some("cpu".into());

        config.decoding_method = Some("greedy_search".to_string());

        let (rule1, rule2, rule3) = cfg.endpoint.to_recognizer_rules();
        config.enable_endpoint = true;
        config.rule1_min_trailing_silence = rule1;
        config.rule2_min_trailing_silence = rule2;
        config.rule3_min_utterance_length = rule3;

        let recognizer = OnlineRecognizer::create(&config).ok_or_else(|| {
            AsrError::ModelLoad(tf(
                Key::AsrRecognizerNone,
                &[&Path::new(dir).display().to_string()],
            ))
        })?;

        let stream = recognizer.create_stream();
        if self
            .entry
            .required
            .asr
            .as_ref()
            .is_some_and(|asr| asr.stream_language)
        {
            stream.set_option("language", crate::config::base_lang(&cfg.source_lang));
        }

        self.recognizer = Some(recognizer);
        self.stream = Some(stream);
        self.utterance = 0;
        self.samples_fed = 0;
        self.utterance_start_ms = 0;
        self.flushed = false;
        Ok(())
    }

    fn capabilities(&self) -> EngineCaps {
        EngineCaps::STREAMING_TRANSDUCER
    }

    fn feed_pcm(&mut self, pcm: &[f32]) -> Result<Vec<TranscriptSegment>, AsrError> {
        if self.recognizer.is_none() {
            return Err(AsrError::NotInitialized);
        }
        let stream = self.strm()?;
        stream.accept_waveform(TARGET_SAMPLE_RATE as i32, pcm);
        self.samples_fed += pcm.len() as u64;

        let mut out = Vec::new();
        self.drain(&mut out)?;
        Ok(out)
    }

    fn flush(&mut self) -> Result<Vec<TranscriptSegment>, AsrError> {
        if self.recognizer.is_none() {
            return Err(AsrError::NotInitialized);
        }
        if self.flushed {
            return Ok(Vec::new());
        }
        self.flushed = true;

        let tail_len = (TARGET_SAMPLE_RATE as f32 * TAIL_SILENCE_SECONDS) as usize;
        let tail = vec![0.0f32; tail_len];

        {
            let stream = self.strm()?;
            stream.accept_waveform(TARGET_SAMPLE_RATE as i32, &tail);
            stream.input_finished();
        }
        self.samples_fed += tail_len as u64;

        let mut out = Vec::new();
        self.drain(&mut out)?;

        if let Some(last) = out.last_mut() {
            if last.state == crate::transcript::SegmentState::Partial {
                last.state = crate::transcript::SegmentState::Final;
            }
        }
        Ok(out)
    }

    fn shutdown(&mut self) {
        self.stream = None;
        self.recognizer = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AsrModelTier;

    fn builtin() -> &'static CatalogEntry {
        crate::models::catalog::asr_entry(AsrModelTier::Builtin)
    }

    #[test]
    fn discover_reports_missing_dir_clearly() {
        let err = discover_model_files(Path::new("/nonexistent/model/dir"), builtin()).unwrap_err();
        assert!(matches!(err, AsrError::ModelLoad(_)));
    }

    fn temp_model_dir(files: &[&str]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("asr-disc-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        for name in files {
            std::fs::write(dir.join(name), b"x").unwrap();
        }
        dir
    }

    #[test]
    fn discover_uses_mixed_precision_matching_upstream() {
        let dir = temp_model_dir(&[
            "encoder-epoch-99-avg-1.onnx",
            "encoder-epoch-99-avg-1.int8.onnx",
            "decoder-epoch-99-avg-1.onnx",
            "decoder-epoch-99-avg-1.int8.onnx",
            "joiner-epoch-99-avg-1.onnx",
            "joiner-epoch-99-avg-1.int8.onnx",
            "tokens.txt",
        ]);

        let f = discover_model_files(&dir, builtin()).unwrap();
        assert!(
            f.encoder.to_string_lossy().contains(".int8."),
            "encoder 应用 int8"
        );
        assert!(
            !f.decoder.to_string_lossy().contains(".int8."),
            "decoder 应保持 fp32"
        );
        assert!(
            f.joiner.to_string_lossy().contains(".int8."),
            "joiner 应用 int8"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn discover_follows_the_catalog_precision_declaration() {
        let dir = temp_model_dir(&[
            "encoder-x.onnx",
            "encoder-x.int8.onnx",
            "decoder-x.onnx",
            "decoder-x.int8.onnx",
            "joiner-x.onnx",
            "joiner-x.int8.onnx",
            "tokens.txt",
        ]);
        let nemotron = crate::models::catalog::asr_entry(AsrModelTier::Nemotron);
        let f = discover_model_files(&dir, nemotron).unwrap();
        assert!(
            f.encoder.to_string_lossy().contains(".int8."),
            "Nemotron 的 encoder 应选 int8"
        );
        assert!(
            f.decoder.to_string_lossy().contains(".int8."),
            "Nemotron 的 decoder 必须按 catalog 选 int8"
        );
        assert!(
            f.joiner.to_string_lossy().contains(".int8."),
            "Nemotron 的 joiner 应选 int8"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn discover_falls_back_when_preferred_precision_is_absent() {
        let dir = temp_model_dir(&[
            "encoder-epoch-99-avg-1.onnx",
            "decoder-epoch-99-avg-1.int8.onnx",
            "joiner-epoch-99-avg-1.onnx",
            "tokens.txt",
        ]);
        assert!(discover_model_files(&dir, builtin()).is_ok());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn discover_error_lists_actual_directory_contents() {
        let dir = temp_model_dir(&["encoder-epoch-99-avg-1.int8.onnx", "tokens.txt"]);
        let msg = discover_model_files(&dir, builtin())
            .unwrap_err()
            .to_string();
        assert!(msg.contains("decoder"), "{msg}");
        assert!(msg.contains("encoder-epoch-99-avg-1.int8.onnx"), "{msg}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn discover_ignores_chunk_variant_subdirectories() {
        let dir = temp_model_dir(&[
            "encoder-epoch-99-avg-1.int8.onnx",
            "decoder-epoch-99-avg-1.onnx",
            "joiner-epoch-99-avg-1.int8.onnx",
            "tokens.txt",
        ]);
        std::fs::create_dir_all(dir.join("64")).unwrap();
        std::fs::write(dir.join("64").join("encoder-x.onnx"), b"x").unwrap();

        let f = discover_model_files(&dir, builtin()).unwrap();
        assert!(!f.encoder.to_string_lossy().contains("/64/"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn feed_before_init_is_an_error_not_a_panic() {
        let mut a = StreamingAdapter::new(builtin());
        assert!(matches!(
            a.feed_pcm(&[0.0; 1600]),
            Err(AsrError::NotInitialized)
        ));
    }

    #[test]
    fn capabilities_declare_partial_support() {
        let a = StreamingAdapter::new(builtin());
        let c = a.capabilities();
        assert!(c.streaming && c.produces_partial && c.supports_endpoint);
        assert!(!c.requires_full_utterance);
    }
}
