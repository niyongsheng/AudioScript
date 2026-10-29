use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};

use crate::asr::{AsrEngine, TARGET_SAMPLE_RATE};
use crate::audio::{self, CaptureHandle, SILENCE_RMS};
use crate::config::{
    effective_target_lang, AsrConfig, EngineType, MtModelTier, SessionConfig, TranslateConfig,
    TranslateEngine, ZhScript,
};
use crate::i18n::{tf, tr, Key};
use crate::models::catalog;
use crate::paths;
use crate::transcript::{TranscriptSegment, TranscriptStore, TranslationState};
use crate::translate::queue::{Job, JobQueue};
use crate::translate::Translator;

const TRANSLATE_QUEUE_CAPACITY: usize = 32;

fn create_translator(
    roots: &paths::ModelSearchRoots,
    cfg: &TranslateConfig,
) -> Result<Box<dyn Translator>, String> {
    let engine: Box<dyn Translator> = match cfg.engine {
        TranslateEngine::LocalNmt => {
            let entry = catalog::mt_entry(cfg.mt_model_tier);
            let resolved = paths::locate_mt_root(roots, entry)?;
            tracing::info!(
                via = resolved.via.as_str(),
                path = %resolved.path.display(),
                model = %entry.id,
                "翻译模型目录"
            );
            let missing = paths::missing_mt_files(&resolved.path, entry)?;
            if !missing.is_empty() {
                return Err(tf(
                    Key::TranslateSessionModelMissing,
                    &[
                        &entry.localized_title(),
                        &resolved.path.display().to_string(),
                        &missing.join(", "),
                    ],
                ));
            }
            match entry.mt_format() {
                Some(catalog::MtFormat::Marian) => {
                    Box::new(crate::translate::local_nmt::LocalNmt::new(resolved.path))
                }
                Some(catalog::MtFormat::Nllb) => {
                    Box::new(crate::translate::nllb::NllbTranslator::new(resolved.path))
                }
                None => return Err(tf(Key::TranslateEngineNotWired, &[&entry.id])),
            }
        }
        TranslateEngine::MacTranslation => {
            #[cfg(target_os = "macos")]
            {
                Box::new(crate::translate::mac_translation::MacTranslator::new())
            }
            #[cfg(not(target_os = "macos"))]
            {
                return Err(tf(Key::TranslateEngineNotWired, &[cfg.engine.as_str()]));
            }
        }
        TranslateEngine::Mock => {
            let fail_every = std::env::var(crate::translate::mock::ENV_FAIL_EVERY)
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            Box::new(crate::translate::mock::MockTranslator::new(
                crate::translate::mock::DEFAULT_DELAY,
                fail_every,
            ))
        }
        TranslateEngine::Off => Box::new(crate::translate::mock::MockTranslator::new(
            std::time::Duration::ZERO,
            0,
        )),
        TranslateEngine::RemoteApi => {
            return Err(tf(Key::TranslateEngineNotWired, &[cfg.engine.as_str()]));
        }
    };

    Ok(engine)
}

#[derive(Debug, PartialEq)]
struct TranslatorKey {
    engine: TranslateEngine,
    mt_tier: MtModelTier,
    source_lang: String,
    target_lang: String,
    zh_script: ZhScript,
    model_dir: Option<PathBuf>,
}

struct CachedTranslator {
    key: TranslatorKey,
    translator: Arc<Mutex<Box<dyn Translator>>>,
}

static TRANSLATOR_CACHE: Mutex<Option<CachedTranslator>> = Mutex::new(None);

fn translator_key(
    roots: &paths::ModelSearchRoots,
    cfg: &TranslateConfig,
) -> Result<TranslatorKey, String> {
    let model_dir = if cfg.engine == TranslateEngine::LocalNmt {
        Some(paths::locate_mt_root(roots, catalog::mt_entry(cfg.mt_model_tier))?.path)
    } else {
        None
    };
    Ok(TranslatorKey {
        engine: cfg.engine,
        mt_tier: cfg.mt_model_tier,
        source_lang: cfg.source_lang.clone(),
        target_lang: cfg.target_lang.clone(),
        zh_script: cfg.zh_script,
        model_dir,
    })
}

fn cached_translator(key: &TranslatorKey) -> Option<Arc<Mutex<Box<dyn Translator>>>> {
    let slot = TRANSLATOR_CACHE
        .lock()
        .expect("translator cache mutex poisoned");
    slot.as_ref()
        .filter(|cached| &cached.key == key)
        .map(|cached| Arc::clone(&cached.translator))
}

fn evict_translator() {
    let cached = TRANSLATOR_CACHE
        .lock()
        .expect("translator cache mutex poisoned")
        .take();
    if let Some(cached) = cached {
        cached
            .translator
            .lock()
            .expect("translator mutex poisoned")
            .shutdown();
    }
}

fn translator_for(
    roots: &paths::ModelSearchRoots,
    cfg: &TranslateConfig,
) -> Result<Arc<Mutex<Box<dyn Translator>>>, String> {
    let key = translator_key(roots, cfg)?;
    if let Some(translator) = cached_translator(&key) {
        tracing::info!(engine = cfg.engine.as_str(), "翻译引擎缓存命中");
        return Ok(translator);
    }

    evict_translator();
    let mut translator = create_translator(roots, cfg)?;
    translator
        .init(cfg)
        .map_err(|e| tf(Key::SessionTranslateInitFailed, &[&e.to_string()]))?;
    let translator = Arc::new(Mutex::new(translator));
    *TRANSLATOR_CACHE
        .lock()
        .expect("translator cache mutex poisoned") = Some(CachedTranslator {
        key,
        translator: Arc::clone(&translator),
    });
    Ok(translator)
}

fn is_han(c: char) -> bool {
    matches!(c, '\u{3400}'..='\u{4DBF}' | '\u{4E00}'..='\u{9FFF}' | '\u{F900}'..='\u{FAFF}')
}

fn is_kana(c: char) -> bool {
    matches!(c, '\u{3040}'..='\u{30FF}')
}

fn is_hangul(c: char) -> bool {
    matches!(c, '\u{1100}'..='\u{11FF}' | '\u{3130}'..='\u{318F}' | '\u{AC00}'..='\u{D7AF}')
}

fn language_mismatch(text: &str, source_lang: &str) -> bool {
    let mut han = 0usize;
    let mut kana = 0usize;
    let mut hangul = 0usize;
    let mut latin = 0usize;
    for c in text.chars() {
        if is_han(c) {
            han += 1;
        } else if is_kana(c) {
            kana += 1;
        } else if is_hangul(c) {
            hangul += 1;
        } else if c.is_ascii_alphabetic() {
            latin += 1;
        }
    }
    let cjk = han + kana + hangul;
    let total = cjk + latin;
    if total < 2 {
        return false;
    }
    if source_lang.starts_with("zh") {
        latin >= 3 && latin * 10 >= total * 9
    } else if source_lang.starts_with("en") {
        cjk >= 2 && cjk * 4 >= total
    } else if source_lang.starts_with("ja") {
        kana == 0 && (hangul > 0 || latin >= 3)
    } else if source_lang.starts_with("ko") {
        hangul == 0 && (kana > 0 || latin >= 3)
    } else {
        false
    }
}

pub const EVENT_SEGMENTS: &str = "transcript://segments";
pub const EVENT_TASK_STATE: &str = "transcript://state";
pub const EVENT_AUDIO: &str = "transcript://audio";
pub const EVENT_LEVEL: &str = "transcript://level";
pub const EVENT_LANGUAGE_MISMATCH: &str = "transcript://language-mismatch";

const FIRST_TEXT_BUDGET_MS: u128 = 200;

#[typeshare::typeshare]
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioStatus {
    pub silent: bool,
    pub rms: f64,
    pub segments: u32,
}

const EMIT_INTERVAL: Duration = Duration::from_millis(100);

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

const LEVEL_INTERVAL: Duration = Duration::from_millis(200);

#[typeshare::typeshare]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskStatus {
    #[default]
    Ready,
    Capturing,
    Error,
}

struct RunningSession {
    capture: CaptureHandle,
    engine: Arc<Mutex<Box<dyn AsrEngine>>>,
    device: String,
    device_sample_rate: u32,

    queue: Arc<JobQueue>,
    events: Sender<UiEvent>,
}

enum UiEvent {
    Status(TaskStatus),
    Level(f64),
    Audio(AudioStatus),
    Segments(Vec<TranscriptSegment>),
    LanguageMismatch(bool),
}

static EVENT_SENDER: std::sync::OnceLock<Sender<UiEvent>> = std::sync::OnceLock::new();

pub fn init_emitter(app: &AppHandle) -> Result<(), String> {
    let (tx, rx) = std::sync::mpsc::channel::<UiEvent>();
    let app = app.clone();

    std::thread::Builder::new()
        .name("session-events".into())
        .spawn(move || {
            loop {
                let event = match rx.recv_timeout(Duration::from_secs(5)) {
                    Ok(event) => event,
                    Err(RecvTimeoutError::Timeout) => continue,
                    Err(RecvTimeoutError::Disconnected) => break,
                };

                let t0 = Instant::now();
                match event {
                    UiEvent::Status(status) => emit_status(&app, status),
                    UiEvent::Segments(segments) => emit_segments(&app, segments),
                    UiEvent::Level(level) => {
                        if let Err(e) = app.emit(EVENT_LEVEL, level) {
                            tracing::debug!("广播电平失败（可能没有窗口）：{e}");
                        }
                    }
                    UiEvent::Audio(status) => {
                        if let Err(e) = app.emit(EVENT_AUDIO, status) {
                            tracing::debug!("广播音频状态失败（可能没有窗口）：{e}");
                        }
                    }
                    UiEvent::LanguageMismatch(mismatch) => {
                        if let Err(e) = app.emit(EVENT_LANGUAGE_MISMATCH, mismatch) {
                            tracing::debug!("广播语言不匹配提示失败（可能没有窗口）：{e}");
                        }
                    }
                }

                let ms = t0.elapsed().as_millis();
                if ms >= 300 {
                    tracing::warn!(ms, "事件广播耗时异常");
                }
            }
            tracing::debug!("事件线程退出");
        })
        .map_err(|e| tf(Key::SessionEventThreadStartFailed, &[&e.to_string()]))?;

    let _ = EVENT_SENDER.set(tx);
    Ok(())
}

fn events_sender() -> Result<Sender<UiEvent>, String> {
    EVENT_SENDER
        .get()
        .cloned()
        .ok_or_else(|| tr(Key::SessionEventThreadMissing).to_string())
}

enum TranslateCommand {
    Start {
        queue: Arc<JobQueue>,
        translator: Arc<Mutex<Box<dyn Translator>>>,
        store: Arc<Mutex<TranscriptStore>>,
        events: Sender<UiEvent>,
    },
}

static TRANSLATE_TX: std::sync::OnceLock<Sender<TranslateCommand>> = std::sync::OnceLock::new();

pub fn init_translate_worker() -> Result<(), String> {
    let (tx, rx) = std::sync::mpsc::channel::<TranslateCommand>();
    std::thread::Builder::new()
        .name("translate-worker".into())
        .spawn(move || {
            while let Ok(TranslateCommand::Start {
                queue,
                translator,
                store,
                events,
            }) = rx.recv()
            {
                while let Some(job) = queue.pop_blocking() {
                    let result = translator
                        .lock()
                        .expect("translator mutex poisoned")
                        .translate(&job.text)
                        .map_err(|e| e.to_string());

                    let updated = {
                        let mut s = store.lock().expect("transcript store mutex poisoned");
                        s.set_translation(&job.segment_id, result).ok()
                    };

                    if let Some(seg) = updated {
                        match seg.translation_state {
                            TranslationState::Done => {
                                tracing::info!(
                                    "[译文] {}",
                                    seg.translation.as_deref().unwrap_or("")
                                )
                            }
                            TranslationState::Failed => {
                                tracing::warn!(id = %seg.id, "[译文] 失败，该段只显示原文")
                            }
                            _ => {}
                        }
                        let _ = events.send(UiEvent::Segments(vec![seg]));
                    }
                }
                tracing::info!("翻译工作线程空闲");
            }
            tracing::debug!("翻译线程退出");
        })
        .map_err(|e| tf(Key::SessionTranslateThreadStartFailed, &[&e.to_string()]))?;
    let _ = TRANSLATE_TX.set(tx);
    Ok(())
}

#[typeshare::typeshare]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub status: TaskStatus,
    pub device: Option<String>,
    pub device_sample_rate: Option<u32>,
}

#[derive(Default)]
pub struct SessionState {
    store: Arc<Mutex<TranscriptStore>>,
    running: Mutex<Option<RunningSession>>,
    status: Mutex<TaskStatus>,
    started_at: Mutex<Option<Instant>>,
    first_text_logged: AtomicBool,
    audio_started_at: Arc<Mutex<Option<Instant>>>,
    teardown_active: AtomicBool,
}

impl SessionState {
    fn note_first_text(&self) {
        if self.first_text_logged.swap(true, Ordering::SeqCst) {
            return;
        }
        let t0 = *self
            .started_at
            .lock()
            .expect("session start mutex poisoned");
        if let Some(t0) = t0 {
            let ms = t0.elapsed().as_millis();
            if ms <= FIRST_TEXT_BUDGET_MS {
                tracing::info!(ms, budget = FIRST_TEXT_BUDGET_MS, "首次出字：达标");
            } else {
                tracing::warn!(
                    ms,
                    budget = FIRST_TEXT_BUDGET_MS,
                    "首次出字：**超出预算**（含引擎加载——模型是在点开始时才加载的）"
                );
            }
        }
    }
}

impl SessionState {
    pub fn status(&self) -> TaskStatus {
        *self.status.lock().expect("session status mutex poisoned")
    }

    fn set_status(&self, s: TaskStatus) {
        *self.status.lock().expect("session status mutex poisoned") = s;
    }

    pub fn snapshot(&self) -> Vec<TranscriptSegment> {
        self.store
            .lock()
            .expect("transcript store mutex poisoned")
            .snapshot()
            .to_vec()
    }

    pub fn is_running(&self) -> bool {
        self.running
            .lock()
            .expect("session running mutex poisoned")
            .is_some()
    }

    pub(crate) fn wait_until_idle(&self, timeout: Duration) -> bool {
        let limit = timeout.as_millis() as u64;
        let mut waited_ms = 0u64;
        while self.teardown_active.load(Ordering::SeqCst) && waited_ms < limit {
            std::thread::sleep(Duration::from_millis(100));
            waited_ms += 100;
        }
        let idle = !self.teardown_active.load(Ordering::SeqCst);
        if waited_ms > 0 {
            tracing::info!(ms = waited_ms, "等待上一次会话收尾完成");
        }
        idle
    }

    pub fn info(&self) -> SessionInfo {
        let guard = self.running.lock().expect("session running mutex poisoned");
        let status = self.status();
        match guard.as_ref() {
            Some(r) => SessionInfo {
                status,
                device: Some(r.device.clone()),
                device_sample_rate: Some(r.device_sample_rate),
            },
            None => SessionInfo {
                status,
                device: None,
                device_sample_rate: None,
            },
        }
    }
}

impl std::fmt::Debug for SessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionState")
            .field("status", &self.status())
            .field("running", &self.is_running())
            .field("segments", &self.snapshot().len())
            .finish()
    }
}

pub fn start(
    app: &AppHandle,
    cfg: SessionConfig,
    source: audio::AudioSourceKind,
    devices: audio::DeviceSelection,
) -> Result<(), String> {
    let mut cfg = cfg;
    let prefs = crate::prefs::load(app);
    cfg.translate.mt_model_tier = prefs.mt_model_tier;
    let target_lang = effective_target_lang(
        cfg.translate.engine,
        cfg.translate.mt_model_tier,
        &cfg.translate.source_lang,
        &cfg.translate.target_lang,
    )
    .to_owned();
    cfg.translate.target_lang = target_lang;

    tracing::info!("启动：收到命令");
    let state = app.state::<SessionState>();

    if state.is_running() {
        return Err(tr(Key::SessionAlreadyCapturing).into());
    }

    if !state.wait_until_idle(Duration::from_secs(5)) {
        return Err(tr(Key::SessionTeardownBusy).into());
    }

    let t0 = Instant::now();
    *state
        .started_at
        .lock()
        .expect("session start mutex poisoned") = Some(t0);
    state.first_text_logged.store(false, Ordering::SeqCst);
    *state
        .audio_started_at
        .lock()
        .expect("audio start mutex poisoned") = None;

    let roots = paths::search_roots(app);

    let (mut engine, engine_cfg): (Box<dyn AsrEngine>, AsrConfig) = match cfg.asr.engine_type {
        EngineType::MacNative => {
            let engine_cfg = AsrConfig {
                model_dir: None,
                source_lang: cfg.translate.source_lang.clone(),
                ..cfg.asr.clone()
            };
            (
                Box::new(crate::asr::mac_native::MacNativeAdapter::new()),
                engine_cfg,
            )
        }
        EngineType::LocalStreaming => {
            let asr_entry = catalog::asr_entry(prefs.asr_model_tier);
            let asr_dir = if let Some(custom) = cfg
                .asr
                .model_dir
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                let dir = dunce::simplified(std::path::Path::new(custom)).to_path_buf();
                if !dir.is_dir() {
                    return Err(tf(
                        Key::SessionCustomAsrMissing,
                        &[&dir.display().to_string()],
                    ));
                }
                let missing = paths::missing_asr_files(&dir, asr_entry)?;
                if !missing.is_empty() {
                    return Err(tf(
                        Key::SessionCustomAsrIncomplete,
                        &[&dir.display().to_string(), &missing.join(", ")],
                    ));
                }
                tracing::info!(path = %dir.display(), "使用自定义 ASR 模型目录");
                dir
            } else {
                let resolved = paths::locate_asr(&roots, prefs.asr_model_tier)?;
                tracing::info!(
                    tier = prefs.asr_model_tier.as_str(),
                    via = resolved.via.as_str(),
                    path = %resolved.path.display(),
                    "会话使用 ASR 模型"
                );
                let missing = paths::missing_asr_files(&resolved.path, asr_entry)?;
                if !missing.is_empty() {
                    return Err(tf(
                        Key::SessionAsrModelIncomplete,
                        &[
                            &asr_entry.localized_title(),
                            &resolved.path.display().to_string(),
                            &missing.join(", "),
                        ],
                    ));
                }
                resolved.path
            };

            let engine: Box<dyn AsrEngine> = match asr_entry.asr_format() {
                Some(catalog::AsrFormat::Transducer) => {
                    Box::new(crate::asr::streaming::StreamingAdapter::new())
                }
                Some(catalog::AsrFormat::Qwen3Asr) => {
                    let vad = paths::vad_model_path(&roots)
                        .ok_or_else(|| tf(Key::AsrVadModelMissing, &[paths::VAD_MODEL_FILE]))?;
                    Box::new(crate::asr::offline::Qwen3OfflineAdapter::new(
                        vad, asr_entry,
                    ))
                }
                None => return Err(tf(Key::AsrInvalidConfig, &[&asr_entry.id])),
            };
            let engine_cfg = AsrConfig {
                model_dir: Some(asr_dir.to_string_lossy().into_owned()),
                ..cfg.asr.clone()
            };
            (engine, engine_cfg)
        }
        other => {
            return Err(tf(Key::SessionEngineNotWired, &[other.as_str()]));
        }
    };
    engine
        .init(&engine_cfg)
        .map_err(|e| tf(Key::SessionAsrInitFailed, &[&e.to_string()]))?;
    tracing::info!(
        engine = cfg.asr.engine_type.as_str(),
        caps = ?engine.capabilities(),
        ms = t0.elapsed().as_millis(),
        "ASR 引擎已就绪"
    );

    let engine: Arc<Mutex<Box<dyn AsrEngine>>> = Arc::new(Mutex::new(engine));

    let translator = translator_for(&roots, &cfg.translate)?;

    let queue = Arc::new(JobQueue::new(TRANSLATE_QUEUE_CAPACITY));
    let events = events_sender()?;

    let translate_tx = TRANSLATE_TX
        .get()
        .ok_or_else(|| tr(Key::SessionTranslateThreadMissing))?
        .clone();
    translate_tx
        .send(TranslateCommand::Start {
            queue: Arc::clone(&queue),
            translator: Arc::clone(&translator),
            store: Arc::clone(&state.store),
            events: events.clone(),
        })
        .map_err(|_| tr(Key::SessionTranslateThreadExited).to_string())?;

    let caps = translator
        .lock()
        .expect("translator mutex poisoned")
        .capabilities();
    tracing::info!(
        engine = cfg.translate.engine.as_str(),
        ?caps,
        ms = t0.elapsed().as_millis(),
        "翻译引擎已就绪"
    );

    let store = Arc::clone(&state.store);
    let engine_for_sink = Arc::clone(&engine);
    let queue_for_sink = Arc::clone(&queue);
    let events_for_sink = events.clone();

    let mut last_emit = Instant::now()
        .checked_sub(EMIT_INTERVAL)
        .unwrap_or_else(Instant::now);
    let mut pending: Vec<TranscriptSegment> = Vec::new();

    let mut last_beat = Instant::now();
    let mut energy = 0.0f64;
    let mut energy_n = 0usize;
    let mut was_silent: Option<bool> = None;
    let mut last_level = Instant::now();
    let mut level_sq = 0.0f64;
    let mut level_n = 0usize;
    let mut level_emit_failed = false;
    let mut language_mismatch_reported = false;

    tracing::info!("启动采集");
    let translate_source_is_zh = cfg.translate.source_lang.starts_with("zh");
    let translate_source_lang = cfg.translate.source_lang.clone();
    let translate_enabled = cfg.translate.engine != TranslateEngine::Off;
    let language_check_enabled = cfg.translate.engine == TranslateEngine::LocalNmt;
    let zh_script = cfg.translate.zh_script;
    let audio_clock = Arc::clone(&state.audio_started_at);
    let capture = audio::start(source, &devices, move |chunk: &[f32]| {
        {
            let mut clock = audio_clock.lock().expect("audio start mutex poisoned");
            if clock.is_none() {
                *clock = Some(Instant::now());
            }
        }

        let sum_sq = chunk.iter().map(|&s| (s as f64) * (s as f64)).sum::<f64>();
        energy += sum_sq;
        energy_n += chunk.len();
        level_sq += sum_sq;
        level_n += chunk.len();

        if last_level.elapsed() >= LEVEL_INTERVAL {
            let level = audio::rms(level_sq, level_n);
            if events_for_sink.send(UiEvent::Level(level)).is_err() && !level_emit_failed {
                level_emit_failed = true;
                tracing::error!("事件线程已退出，后续电平不再广播");
            }
            last_level = Instant::now();
            level_sq = 0.0;
            level_n = 0;
        }

        if last_beat.elapsed() >= HEARTBEAT_INTERVAL {
            let rms = audio::rms(energy, energy_n);
            let count = store
                .lock()
                .expect("transcript store mutex poisoned")
                .segment_count();
            let silent = rms < SILENCE_RMS;

            if silent {
                let hint = if count > 0 {
                    "已转出过文字，多半只是暂时没有音频（暂停或静音段落），不一定是故障"
                } else if matches!(
                    source,
                    audio::AudioSourceKind::Loopback | audio::AudioSourceKind::Mix
                ) {
                    "还没转出任何文字。确认播放器在出声；若确实在播，检查「屏幕与系统音频录制」权限"
                } else {
                    "还没转出任何文字，麦克风可能未授权、被占用，或所选设备是虚拟回环"
                };

                if count > 0 {
                    if was_silent != Some(true) {
                        tracing::debug!(
                            rms = format!("{rms:.6}"),
                            segments = count,
                            source = source.as_str(),
                            "音频电平接近零：{hint}"
                        );
                    }
                } else {
                    tracing::warn!(
                        rms = format!("{rms:.6}"),
                        segments = count,
                        source = source.as_str(),
                        "**音频电平接近零**：{hint}"
                    );
                }
            } else {
                tracing::info!(
                    rms = format!("{rms:.4}"),
                    segments = count,
                    source = source.as_str(),
                    "心跳"
                );
            }

            if was_silent != Some(silent) {
                was_silent = Some(silent);
                let _ = events_for_sink.send(UiEvent::Audio(AudioStatus {
                    silent,
                    rms,
                    segments: count as u32,
                }));
            }

            last_beat = Instant::now();
            energy = 0.0;
            energy_n = 0;
        }

        let mut deltas = {
            let mut eng = engine_for_sink.lock().expect("engine mutex poisoned");
            match eng.feed_pcm(chunk) {
                Ok(d) => d,
                Err(e) => {
                    tracing::error!("引擎处理音频失败：{e}");
                    return;
                }
            }
        };
        if deltas.is_empty() {
            return;
        }
        if translate_source_is_zh && zh_script == ZhScript::Traditional {
            for d in &mut deltas {
                d.text = crate::translate::script::to_traditional(&d.text);
            }
        }

        let to_translate = {
            let mut s = store.lock().expect("transcript store mutex poisoned");
            match s.apply(deltas) {
                Ok(changed) => {
                    let mut mismatch = false;
                    let mut ids = Vec::new();
                    for seg in &changed {
                        let mismatched = language_check_enabled
                            && seg.state.is_frozen()
                            && language_mismatch(&seg.text, &translate_source_lang);
                        mismatch |= mismatched;
                        if translate_enabled
                            && seg.state.is_frozen()
                            && seg.translation_state == TranslationState::Idle
                            && !mismatched
                        {
                            ids.push(seg.id.clone());
                        }
                        if seg.state != crate::transcript::SegmentState::Partial {
                            let audio_ms: i128 = audio_clock
                                .lock()
                                .expect("audio start mutex poisoned")
                                .map(|a| a.elapsed().as_millis() as i128)
                                .unwrap_or(0);
                            tracing::info!(
                                start_ms = seg.start_ms,
                                end_ms = seg.end_ms,
                                gap_ms = audio_ms - seg.end_ms as i128,
                                "[定稿] {}",
                                seg.text
                            );
                        }
                    }

                    if mismatch && !language_mismatch_reported {
                        language_mismatch_reported = true;
                        tracing::warn!(
                            source_lang = %translate_source_lang,
                            "识别文本与源语言设置不一致，跳过翻译"
                        );
                        let _ = events_for_sink.send(UiEvent::LanguageMismatch(true));
                    }

                    let marked = s.mark_pending_translation(&ids);
                    pending.extend(changed);
                    marked
                }
                Err(e) => {
                    tracing::error!("段落状态机拒绝了引擎输出：{e}");
                    return;
                }
            }
        };

        for seg in to_translate {
            let dropped = queue_for_sink.push(Job {
                segment_id: seg.id.clone(),
                text: seg.text.clone(),
            });
            if let Some(d) = dropped {
                tracing::warn!(
                    dropped_ms = d.segment_id,
                    "翻译队列溢出，丢弃最旧的任务（翻译跟不上实时）"
                );
            }
            pending.push(seg);
        }

        if last_emit.elapsed() >= EMIT_INTERVAL && !pending.is_empty() {
            let batch = coalesce(std::mem::take(&mut pending));
            let _ = events_for_sink.send(UiEvent::Segments(batch));
            last_emit = Instant::now();
        }
    })
    .map_err(|e| tf(Key::SessionStartCaptureFailed, &[&e.to_string()]))?;

    let device_name = capture.device_name().to_string();
    let device_sample_rate = capture.device_sample_rate();

    tracing::info!(
        device = %device_name,
        device_sample_rate,
        target_sample_rate = TARGET_SAMPLE_RATE,
        setup_ms = t0.elapsed().as_millis(),
        "转写会话已开始"
    );

    let _ = events.send(UiEvent::Status(TaskStatus::Capturing));

    *state
        .running
        .lock()
        .expect("session running mutex poisoned") = Some(RunningSession {
        capture,
        engine,
        device: device_name,
        device_sample_rate,
        queue,
        events,
    });
    state.set_status(TaskStatus::Capturing);

    crate::telemetry::track(
        app,
        "session_started",
        Some(serde_json::json!({
            "audio_source": source.as_str(),
            "translate_engine": cfg.translate.engine.as_str(),
            "source_lang": cfg.translate.source_lang,
            "target_lang": cfg.translate.target_lang,
            "zh_script": cfg.translate.zh_script.as_str(),
            "asr_model_tier": prefs.asr_model_tier.as_str(),
            "custom_asr_model": if cfg.asr.model_dir.as_deref().is_some_and(|s| !s.trim().is_empty()) { 1 } else { 0 },
        })),
    );

    Ok(())
}

pub fn stop(app: &AppHandle) -> Result<(), String> {
    tracing::info!("停止：收到命令");
    let state = app.state::<SessionState>();

    let Some(session) = state
        .running
        .lock()
        .expect("session running mutex poisoned")
        .take()
    else {
        state.set_status(TaskStatus::Ready);
        tracing::info!("停止：没有运行中的会话，直接返回");
        return Ok(());
    };
    tracing::info!("停止：已摘除运行会话");

    let audio_ms = {
        let clock = state
            .audio_started_at
            .lock()
            .expect("audio start mutex poisoned");
        (*clock)
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0)
    };
    crate::telemetry::track(
        app,
        "session_stopped",
        Some(serde_json::json!({
            "duration_bucket": crate::telemetry::duration_bucket(audio_ms),
            "segments": state.snapshot().len() as u64,
        })),
    );

    session.capture.request_stop();
    session.queue.close();

    state.set_status(TaskStatus::Ready);
    let _ = session.events.send(UiEvent::Status(TaskStatus::Ready));
    tracing::info!("停止：状态已就绪，收尾转入后台");

    state.teardown_active.store(true, Ordering::SeqCst);

    let app_for_teardown = app.clone();
    let task: TeardownTask = Box::new(move || finish_teardown(&app_for_teardown, session));

    match TEARDOWN_TX.get() {
        Some(tx) => {
            if let Err(err) = tx.send(task) {
                tracing::error!("收尾线程已退出，改为在命令线程收尾");
                (err.0)();
            }
        }
        None => {
            tracing::error!("收尾线程未初始化，改为在命令线程收尾");
            task();
        }
    }

    Ok(())
}

type TeardownTask = Box<dyn FnOnce() + Send>;

static TEARDOWN_TX: std::sync::OnceLock<std::sync::mpsc::Sender<TeardownTask>> =
    std::sync::OnceLock::new();

pub fn init_teardown_executor() -> Result<(), String> {
    let (tx, rx) = std::sync::mpsc::channel::<TeardownTask>();
    std::thread::Builder::new()
        .name("session-teardown".into())
        .spawn(move || {
            while let Ok(task) = rx.recv() {
                task();
            }
            tracing::debug!("收尾线程退出");
        })
        .map_err(|e| tf(Key::SessionTeardownThreadStartFailed, &[&e.to_string()]))?;
    let _ = TEARDOWN_TX.set(tx);
    Ok(())
}

fn finish_teardown(app: &AppHandle, session: RunningSession) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        teardown(app, session);
    }));
    if outcome.is_err() {
        tracing::error!("收尾线程异常退出");
    }
    app.state::<SessionState>()
        .teardown_active
        .store(false, Ordering::SeqCst);
}

fn teardown(app: &AppHandle, session: RunningSession) {
    let RunningSession {
        capture,
        engine,
        queue,
        events,
        ..
    } = session;
    let t0 = Instant::now();

    tracing::info!("收尾：等待采集线程退出");
    drop(capture);
    tracing::info!(ms = t0.elapsed().as_millis(), "收尾：采集线程已退出");

    tracing::info!("收尾：冲刷引擎");
    let tail = {
        let mut eng = engine.lock().expect("engine mutex poisoned");
        let flushed = eng.flush();
        eng.shutdown();
        flushed
    };
    let changed = match tail {
        Ok(tail) if !tail.is_empty() => {
            let state = app.state::<SessionState>();
            let applied = {
                let mut s = state.store.lock().expect("transcript store mutex poisoned");
                s.apply(tail)
            };
            match applied {
                Ok(changed) => changed,
                Err(e) => {
                    tracing::error!("收尾段落被状态机拒绝：{e}");
                    Vec::new()
                }
            }
        }
        Ok(_) => Vec::new(),
        Err(e) => {
            tracing::error!("冲刷引擎失败：{e}");
            Vec::new()
        }
    };
    tracing::info!(ms = t0.elapsed().as_millis(), "收尾：引擎已关闭");

    let dropped = queue.clear();
    if dropped > 0 {
        tracing::info!(dropped, "停止时丢弃了积压的翻译任务");
    }
    queue.close();

    tracing::info!(
        ms = t0.elapsed().as_millis(),
        "收尾：翻译队列已关闭，翻译引擎保留在缓存"
    );

    if !changed.is_empty() {
        let _ = events.send(UiEvent::Segments(changed));
    }
}

pub fn clear(app: &AppHandle) {
    let state = app.state::<SessionState>();
    {
        let mut store = state.store.lock().expect("transcript store mutex poisoned");
        *store = TranscriptStore::new();
    }
    emit_segments(app, Vec::new());
}

fn coalesce(batch: Vec<TranscriptSegment>) -> Vec<TranscriptSegment> {
    let mut latest: HashMap<String, TranscriptSegment> = HashMap::new();
    let mut order: Vec<String> = Vec::new();

    for seg in batch {
        if !latest.contains_key(&seg.id) {
            order.push(seg.id.clone());
        }
        let keep = latest
            .get(&seg.id)
            .is_none_or(|e| seg.revision >= e.revision);
        if keep {
            latest.insert(seg.id.clone(), seg);
        }
    }

    order
        .into_iter()
        .filter_map(|id| latest.remove(&id))
        .collect()
}

fn emit_segments(app: &AppHandle, segments: Vec<TranscriptSegment>) {
    if segments.iter().any(|s| !s.text.trim().is_empty()) {
        app.state::<SessionState>().note_first_text();
    }

    if let Err(e) = app.emit(EVENT_SEGMENTS, segments) {
        tracing::debug!("广播段落失败（可能没有窗口在监听）：{e}");
    }
}

fn emit_status(app: &AppHandle, status: TaskStatus) {
    crate::tray::sync_task_state(app, status);
    if let Err(e) = app.emit(EVENT_TASK_STATE, status) {
        tracing::debug!("广播任务状态失败（可能没有窗口在监听）：{e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcript::SegmentState;

    fn seg(id: &str, text: &str, rev: u32) -> TranscriptSegment {
        TranscriptSegment {
            id: id.into(),
            start_ms: 0,
            end_ms: 100,
            text: text.into(),
            translation: None,
            translation_state: crate::transcript::TranslationState::Idle,
            state: SegmentState::Partial,
            revision: rev,
        }
    }

    #[test]
    fn coalesce_keeps_latest_revision_per_id() {
        let batch = vec![
            seg("a", "今", 1),
            seg("a", "今天", 2),
            seg("b", "另", 1),
            seg("a", "今天下午", 3),
        ];
        let out = coalesce(batch);

        assert_eq!(out.len(), 2, "应按 id 去重");
        assert_eq!(out[0].id, "a");
        assert_eq!(out[0].text, "今天下午", "应保留最新 revision");
        assert_eq!(out[1].id, "b", "顺序应按首次出现");
    }

    #[test]
    fn coalesce_does_not_let_an_older_revision_win() {
        let out = coalesce(vec![seg("a", "新", 5), seg("a", "旧", 2)]);
        assert_eq!(out[0].text, "新");
    }

    #[test]
    fn script_mismatch_is_detected_only_for_dominant_scripts() {
        assert!(language_mismatch("你好，世界。", "en"));
        assert!(!language_mismatch("hello world", "en"));
        assert!(language_mismatch("hello world", "zh"));
        assert!(!language_mismatch("你好，世界。", "zh"));
        assert!(language_mismatch("OpenAI 很好", "en"));
        assert!(!language_mismatch("OpenAI 很好", "zh"));
        assert!(!language_mismatch(
            "The meeting with 王伟 was productive",
            "en"
        ));
        assert!(!language_mismatch("Hi", "zh"));
    }

    #[test]
    fn script_mismatch_covers_multilingual_sources() {
        assert!(language_mismatch("hello world", "ja"));
        assert!(!language_mismatch("こんにちは", "ja"));
        assert!(language_mismatch("안녕하세요", "ja"));
        assert!(language_mismatch("hello world", "ko"));
        assert!(!language_mismatch("안녕하세요", "ko"));
        assert!(!language_mismatch("hello world", "fr"));
        assert!(!language_mismatch("bonjour tout le monde", "fr"));
    }
}
