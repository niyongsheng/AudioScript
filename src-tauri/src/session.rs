use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};

use crate::asr::{AsrEngine, TARGET_SAMPLE_RATE};
use crate::audio::{self, CaptureHandle, SILENCE_RMS};
use crate::config::{
    target_lang_for, AsrConfig, EngineType, MtModelTier, SessionConfig, TranslateConfig,
    TranslateEngine, ZhScript,
};
use crate::models::catalog;
use crate::paths;
use crate::transcript::{TranscriptSegment, TranscriptStore, TranslationState};
use crate::translate::queue::{Job, JobQueue};
use crate::translate::Translator;

mod lifecycle;

use lifecycle::{Failure, Lifecycle};

const TRANSLATE_QUEUE_CAPACITY: usize = 32;

fn create_translator(
    app: &AppHandle,
    cfg: &TranslateConfig,
    mt_tier: MtModelTier,
) -> Result<Option<Box<dyn Translator>>, String> {
    let engine: Box<dyn Translator> = match cfg.engine {
        TranslateEngine::LocalNmt => {
            let roots = paths::search_roots(app);
            let resolved = paths::locate_mt_root(&roots, mt_tier)?;
            tracing::info!(
                tier = mt_tier.as_str(),
                via = resolved.via.as_str(),
                path = %resolved.path.display(),
                "翻译模型目录"
            );
            let missing = paths::missing_mt_files(&resolved.path, catalog::mt_entry(mt_tier))?;
            if !missing.is_empty() {
                return Err(format!(
                    "所选翻译模型「{}」未安装或文件不完整：{}\n缺少：{}\n→ 打开「模型」页下载，或切换回基础模型",
                    catalog::mt_entry(mt_tier).title,
                    resolved.path.display(),
                    missing.join(", ")
                ));
            }
            Box::new(crate::translate::local_nmt::LocalNmt::new(
                resolved.path,
                mt_tier,
            ))
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
        TranslateEngine::Off => return Ok(None),
        TranslateEngine::RemoteApi => {
            return Err(format!(
                "翻译引擎 {} 尚未接入。可选值：local-nmt（本地离线）、mock（开发用）、off（仅显示原文）",
                cfg.engine.as_str()
            ));
        }
    };

    Ok(Some(engine))
}

pub const EVENT_SEGMENTS: &str = "transcript://segments";
pub const EVENT_TASK_STATE: &str = "transcript://state";
pub const EVENT_AUDIO: &str = "transcript://audio";
pub const EVENT_LEVEL: &str = "transcript://level";
pub const EVENT_ERROR: &str = "transcript://error";

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

impl From<lifecycle::Status> for TaskStatus {
    fn from(status: lifecycle::Status) -> Self {
        match status {
            lifecycle::Status::Ready => Self::Ready,
            lifecycle::Status::Capturing => Self::Capturing,
            lifecycle::Status::Error => Self::Error,
        }
    }
}

struct TranslationSession {
    queue: Arc<JobQueue>,
    translator: Arc<Mutex<Box<dyn Translator>>>,
    done: std::sync::mpsc::Receiver<()>,
}

impl Drop for TranslationSession {
    fn drop(&mut self) {
        self.queue.close();
    }
}

struct RunningSession {
    capture: CaptureHandle,
    engine: Arc<Mutex<Box<dyn AsrEngine>>>,
    device: String,
    device_sample_rate: u32,

    translation: Option<TranslationSession>,
    events: Sender<UiEvent>,
}

enum UiEvent {
    Status(TaskStatus),
    Failed { epoch: u64, message: String },
    Level(f64),
    Audio(AudioStatus),
    Segments(Vec<TranscriptSegment>),
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
                    UiEvent::Status(status) => {
                        if app.state::<SessionState>().status() == status {
                            emit_status(&app, status);
                        }
                    }
                    UiEvent::Failed { epoch, message } => fail_session(&app, epoch, message),
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
                }

                let ms = t0.elapsed().as_millis();
                if ms >= 300 {
                    tracing::warn!(ms, "事件广播耗时异常");
                }
            }
            tracing::debug!("事件线程退出");
        })
        .map_err(|e| format!("启动事件线程失败：{e}"))?;

    let _ = EVENT_SENDER.set(tx);
    Ok(())
}

fn events_sender() -> Result<Sender<UiEvent>, String> {
    EVENT_SENDER
        .get()
        .cloned()
        .ok_or_else(|| "事件线程未初始化".to_string())
}

enum TranslateCommand {
    Start {
        queue: Arc<JobQueue>,
        translator: Arc<Mutex<Box<dyn Translator>>>,
        store: Arc<Mutex<TranscriptStore>>,
        events: Sender<UiEvent>,
        done: Sender<()>,
    },
}

static TRANSLATE_TX: std::sync::OnceLock<Sender<TranslateCommand>> = std::sync::OnceLock::new();

pub fn init_translate_worker() -> Result<(), String> {
    let (tx, rx) = std::sync::mpsc::channel::<TranslateCommand>();
    std::thread::Builder::new()
        .name("translate-worker".into())
        .spawn(move || run_translate_worker(rx))
        .map_err(|e| format!("启动翻译线程失败：{e}"))?;
    let _ = TRANSLATE_TX.set(tx);
    Ok(())
}

fn run_translate_worker(rx: std::sync::mpsc::Receiver<TranslateCommand>) {
    while let Ok(TranslateCommand::Start {
        queue,
        translator,
        store,
        events,
        done,
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
                        tracing::info!("[译文] {}", seg.translation.as_deref().unwrap_or(""))
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
        let _ = done.send(());
    }
    tracing::debug!("翻译线程退出");
}

#[typeshare::typeshare]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub status: TaskStatus,
    pub device: Option<String>,
    pub device_sample_rate: Option<u32>,
    pub error: Option<String>,
}

#[derive(Default)]
pub struct SessionState {
    store: Arc<Mutex<TranscriptStore>>,
    broadcast: Mutex<()>,
    lifecycle: Mutex<Lifecycle<RunningSession>>,
    started_at: Mutex<Option<Instant>>,
    first_text_logged: AtomicBool,
    audio_started_at: Arc<Mutex<Option<Instant>>>,
    teardown_active: AtomicBool,
}

impl SessionState {
    fn clear_with(&self, emit: impl FnOnce(Vec<TranscriptSegment>)) -> Result<(), String> {
        let _broadcast = self
            .broadcast
            .lock()
            .expect("session broadcast mutex poisoned");
        let lifecycle = self
            .lifecycle
            .lock()
            .expect("session lifecycle mutex poisoned");
        lifecycle.ensure_clearable(self.teardown_active.load(Ordering::SeqCst))?;
        {
            let mut store = self.store.lock().expect("transcript store mutex poisoned");
            *store = TranscriptStore::new();
        }
        emit(Vec::new());
        Ok(())
    }

    fn broadcast_segments(
        &self,
        queued: Vec<TranscriptSegment>,
        emit: impl FnOnce(Vec<TranscriptSegment>),
    ) {
        let _broadcast = self
            .broadcast
            .lock()
            .expect("session broadcast mutex poisoned");
        let segments = {
            let store = self.store.lock().expect("transcript store mutex poisoned");
            coalesce(queued)
                .into_iter()
                .filter_map(|segment| {
                    store
                        .get(&segment.id)
                        .filter(|current| current.revision >= segment.revision)
                        .cloned()
                })
                .collect::<Vec<_>>()
        };
        if !segments.is_empty() {
            if segments
                .iter()
                .any(|segment| !segment.text.trim().is_empty())
            {
                self.note_first_text();
            }
            emit(segments);
        }
    }

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
        self.lifecycle
            .lock()
            .expect("session lifecycle mutex poisoned")
            .status()
            .into()
    }

    pub fn snapshot(&self) -> Vec<TranscriptSegment> {
        self.store
            .lock()
            .expect("transcript store mutex poisoned")
            .snapshot()
            .to_vec()
    }

    pub fn is_running(&self) -> bool {
        self.lifecycle
            .lock()
            .expect("session lifecycle mutex poisoned")
            .running()
            .is_some()
    }

    pub fn info(&self) -> SessionInfo {
        let guard = self
            .lifecycle
            .lock()
            .expect("session lifecycle mutex poisoned");
        let status = guard.status().into();
        let error = guard.error().map(str::to_owned);
        match guard.running() {
            Some(r) => SessionInfo {
                status,
                device: Some(r.device.clone()),
                device_sample_rate: Some(r.device_sample_rate),
                error,
            },
            None => SessionInfo {
                status,
                device: None,
                device_sample_rate: None,
                error,
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

struct StartAttempt<'a> {
    state: &'a SessionState,
    epoch: u64,
}

impl Drop for StartAttempt<'_> {
    fn drop(&mut self) {
        self.state
            .lifecycle
            .lock()
            .expect("session lifecycle mutex poisoned")
            .abort_start(self.epoch);
    }
}

pub fn start(
    app: &AppHandle,
    cfg: SessionConfig,
    source: audio::AudioSourceKind,
    device: Option<String>,
) -> Result<(), String> {
    let mut cfg = cfg;
    cfg.translate.target_lang = target_lang_for(&cfg.translate.source_lang).to_owned();

    tracing::info!("启动：收到命令");
    let state = app.state::<SessionState>();

    if state.is_running() {
        return Err("已在采集中".into());
    }

    let mut waited_ms = 0u64;
    while state.teardown_active.load(Ordering::SeqCst) && waited_ms < 5000 {
        std::thread::sleep(Duration::from_millis(100));
        waited_ms += 100;
    }
    if state.teardown_active.load(Ordering::SeqCst) {
        return Err("上一次会话仍在收尾（音频设备尚未释放），请稍后重试".into());
    }
    if waited_ms > 0 {
        tracing::info!(ms = waited_ms, "启动前等待上一次收尾完成");
    }

    let epoch = {
        let mut lifecycle = state
            .lifecycle
            .lock()
            .expect("session lifecycle mutex poisoned");
        if state.teardown_active.load(Ordering::SeqCst) {
            return Err("上一次会话仍在收尾，请稍后重试".into());
        }
        lifecycle.begin()?
    };
    let _attempt = StartAttempt {
        state: &state,
        epoch,
    };

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

    if !matches!(cfg.asr.engine_type, EngineType::LocalStreaming) {
        return Err(format!(
            "引擎 {} 尚未接入（当前仅支持 local-streaming）",
            cfg.asr.engine_type.as_str()
        ));
    }

    let prefs = crate::prefs::load(app);
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
            return Err(format!("自定义 ASR 模型目录不存在：{}", dir.display()));
        }
        let missing = paths::missing_asr_files(&dir, asr_entry)?;
        if !missing.is_empty() {
            return Err(format!(
                "自定义 ASR 模型目录不完整：{}\n缺少：{}",
                dir.display(),
                missing.join(", ")
            ));
        }
        tracing::info!(path = %dir.display(), "使用自定义 ASR 模型目录");
        dir
    } else {
        let roots = paths::search_roots(app);
        let resolved = paths::locate_asr(&roots, prefs.asr_model_tier)?;
        tracing::info!(
            tier = prefs.asr_model_tier.as_str(),
            via = resolved.via.as_str(),
            path = %resolved.path.display(),
            "会话使用 ASR 模型"
        );
        let missing = paths::missing_asr_files(&resolved.path, asr_entry)?;
        if !missing.is_empty() {
            return Err(format!(
                "所选语音识别模型「{}」未安装或文件不完整：{}\n缺少：{}\n→ 打开「模型」页下载，或切换回内置模型",
                asr_entry.title,
                resolved.path.display(),
                missing.join(", ")
            ));
        }
        resolved.path
    };

    let mut engine = crate::asr::streaming::StreamingAdapter::new();
    let engine_cfg = AsrConfig {
        model_dir: Some(asr_dir.to_string_lossy().into_owned()),
        ..cfg.asr.clone()
    };
    engine
        .init(&engine_cfg)
        .map_err(|e| format!("引擎初始化失败：{e}"))?;
    tracing::info!(
        caps = ?engine.capabilities(),
        ms = t0.elapsed().as_millis(),
        "ASR 引擎已就绪"
    );

    let engine: Arc<Mutex<Box<dyn AsrEngine>>> = Arc::new(Mutex::new(Box::new(engine)));

    let events = events_sender()?;
    let translation = match create_translator(app, &cfg.translate, prefs.mt_model_tier)? {
        Some(mut translator) => {
            translator
                .init(&cfg.translate)
                .map_err(|e| format!("翻译引擎初始化失败：{e}"))?;
            tracing::info!(
                engine = cfg.translate.engine.as_str(),
                caps = ?translator.capabilities(),
                ms = t0.elapsed().as_millis(),
                "翻译引擎已就绪"
            );

            let (done_tx, done_rx) = std::sync::mpsc::channel();
            let translation = TranslationSession {
                queue: Arc::new(JobQueue::new(TRANSLATE_QUEUE_CAPACITY)),
                translator: Arc::new(Mutex::new(translator)),
                done: done_rx,
            };
            TRANSLATE_TX
                .get()
                .ok_or("翻译线程未初始化")?
                .send(TranslateCommand::Start {
                    queue: Arc::clone(&translation.queue),
                    translator: Arc::clone(&translation.translator),
                    store: Arc::clone(&state.store),
                    events: events.clone(),
                    done: done_tx,
                })
                .map_err(|_| "翻译线程已退出".to_string())?;
            Some(translation)
        }
        None => None,
    };

    let store = Arc::clone(&state.store);
    let engine_for_sink = Arc::clone(&engine);
    let queue_for_sink = translation.as_ref().map(|t| Arc::clone(&t.queue));
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

    tracing::info!("启动采集");
    let translate_source_is_zh = cfg.translate.source_lang.starts_with("zh");
    let zh_script = cfg.translate.zh_script;
    let audio_clock = Arc::clone(&state.audio_started_at);
    let events_for_failure = events.clone();
    let capture = audio::start(source, device.as_deref(), move |chunk: &[f32]| {
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
            eng.feed_pcm(chunk)
                .map_err(|e| format!("语音识别中断：{e}"))?
        };
        if deltas.is_empty() {
            return Ok(());
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
                    for seg in &changed {
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

                    let marked = schedule_translation(&mut s, &changed, queue_for_sink.as_deref());
                    pending.extend(changed);
                    marked
                }
                Err(e) => {
                    return Err(format!("段落状态机拒绝了引擎输出：{e}"));
                }
            }
        };

        pending.extend(to_translate);

        if last_emit.elapsed() >= EMIT_INTERVAL && !pending.is_empty() {
            let batch = coalesce(std::mem::take(&mut pending));
            let _ = events_for_sink.send(UiEvent::Segments(batch));
            last_emit = Instant::now();
        }
        Ok(())
    }, move |message| {
        let _ = events_for_failure.send(UiEvent::Failed { epoch, message });
    })
    .map_err(|e| format!("启动采集失败：{e}"))?;

    let device_name = capture.device_name().to_string();
    let device_sample_rate = capture.device_sample_rate();

    tracing::info!(
        device = %device_name,
        device_sample_rate,
        target_sample_rate = TARGET_SAMPLE_RATE,
        setup_ms = t0.elapsed().as_millis(),
        "转写会话已开始"
    );

    let session = RunningSession {
        capture,
        engine,
        device: device_name,
        device_sample_rate,
        translation,
        events: events.clone(),
    };
    let published = {
        let mut lifecycle = state
            .lifecycle
            .lock()
            .expect("session lifecycle mutex poisoned");
        match lifecycle.publish(epoch, session) {
            Ok(()) => {
                let _ = events.send(UiEvent::Status(TaskStatus::Capturing));
                Ok(())
            }
            Err(session) => {
                state.teardown_active.store(true, Ordering::SeqCst);
                Err((
                    session,
                    lifecycle.error().unwrap_or("启动已取消").to_owned(),
                ))
            }
        }
    };
    if let Err((session, error)) = published {
        schedule_teardown(app, session);
        return Err(error);
    }

    crate::telemetry::track(
        app,
        "session_started",
        Some(serde_json::json!({
            "audio_source": source.as_str(),
            "translate_engine": cfg.translate.engine.as_str(),
            "source_lang": cfg.translate.source_lang,
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
    let session = {
        let mut lifecycle = state
            .lifecycle
            .lock()
            .expect("session lifecycle mutex poisoned");
        let session = lifecycle.stop();
        if session.is_some() {
            state.teardown_active.store(true, Ordering::SeqCst);
        }
        session
    };
    let Some(session) = session else {
        tracing::info!("停止：没有运行中的会话，已取消尚未完成的启动");
        return Ok(());
    };

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
    let _ = session.events.send(UiEvent::Status(TaskStatus::Ready));
    schedule_teardown(app, session);
    Ok(())
}

fn fail_session(app: &AppHandle, epoch: u64, message: String) {
    let state = app.state::<SessionState>();
    let session = {
        let mut lifecycle = state
            .lifecycle
            .lock()
            .expect("session lifecycle mutex poisoned");
        match lifecycle.fail(epoch, message.clone()) {
            Failure::Stale => return,
            Failure::Accepted(session) => {
                if session.is_some() {
                    state.teardown_active.store(true, Ordering::SeqCst);
                }
                session
            }
        }
    };
    tracing::error!(epoch, "会话中断：{message}");
    emit_status(app, TaskStatus::Error);
    if let Err(error) = app.emit(EVENT_ERROR, message) {
        tracing::debug!("广播会话错误失败：{error}");
    }
    if let Some(session) = session {
        schedule_teardown(app, session);
    }
}

fn schedule_teardown(app: &AppHandle, session: RunningSession) {
    session.capture.request_stop();
    if let Some(translation) = &session.translation {
        let jobs = translation.queue.close_and_drain();
        let changed = cancel_translation_jobs(
            &mut app
                .state::<SessionState>()
                .store
                .lock()
                .expect("transcript store mutex poisoned"),
            jobs,
            "会话已结束，取消积压翻译",
        );
        if !changed.is_empty() {
            let _ = session.events.send(UiEvent::Segments(changed));
        }
    }
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
        .map_err(|e| format!("启动收尾线程失败：{e}"))?;
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
        translation,
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

    let state = app.state::<SessionState>();
    let changed = if let Some(translation) = translation {
        translation.queue.close();
        if translation.done.recv().is_err() {
            tracing::error!("翻译工作线程异常退出");
        }
        let mut translator = translation
            .translator
            .lock()
            .expect("translator mutex poisoned");
        let changed = translate_final_tail(&state.store, changed, Some(translator.as_mut()));
        translator.shutdown();
        changed
    } else {
        translate_final_tail(&state.store, changed, None)
    };
    tracing::info!(ms = t0.elapsed().as_millis(), "收尾完成");

    if !changed.is_empty() {
        let _ = events.send(UiEvent::Segments(changed));
    }
}

pub fn clear(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<SessionState>();
    state.clear_with(|segments| broadcast_segments(app, segments))
}

fn translate_final_tail(
    store: &Mutex<TranscriptStore>,
    changed: Vec<TranscriptSegment>,
    translator: Option<&mut dyn Translator>,
) -> Vec<TranscriptSegment> {
    let Some(translator) = translator else {
        return changed;
    };
    let ids = changed
        .iter()
        .filter(|segment| {
            segment.state.is_frozen() && segment.translation_state == TranslationState::Idle
        })
        .map(|segment| segment.id.clone())
        .collect::<Vec<_>>();
    let pending = store
        .lock()
        .expect("transcript store mutex poisoned")
        .mark_pending_translation(&ids);
    let mut updates = changed;
    for segment in pending {
        let result = translator
            .translate(&segment.text)
            .map_err(|error| error.to_string());
        let translated = store
            .lock()
            .expect("transcript store mutex poisoned")
            .set_translation(&segment.id, result);
        if let Ok(segment) = translated {
            updates.push(segment);
        }
    }
    coalesce(updates)
}

fn cancel_translation_jobs(
    store: &mut TranscriptStore,
    jobs: impl IntoIterator<Item = Job>,
    reason: &str,
) -> Vec<TranscriptSegment> {
    jobs.into_iter()
        .filter_map(|job| {
            if store.get(&job.segment_id)?.translation_state != TranslationState::Pending {
                return None;
            }
            store
                .set_translation(&job.segment_id, Err(reason.to_owned()))
                .ok()
        })
        .collect()
}

fn schedule_translation(
    store: &mut TranscriptStore,
    changed: &[TranscriptSegment],
    queue: Option<&JobQueue>,
) -> Vec<TranscriptSegment> {
    let Some(queue) = queue else {
        return Vec::new();
    };
    let ids: Vec<String> = changed
        .iter()
        .filter(|s| s.state.is_frozen() && s.translation_state == TranslationState::Idle)
        .map(|s| s.id.clone())
        .collect();
    let mut updates = Vec::new();
    for segment in store.mark_pending_translation(&ids) {
        let dropped = queue.push(Job {
            segment_id: segment.id.clone(),
            text: segment.text.clone(),
        });
        updates.push(segment);
        if let Some(dropped) = dropped {
            tracing::warn!(id = %dropped.segment_id, "翻译任务已丢弃（队列已满或会话已结束）");
            updates.extend(cancel_translation_jobs(
                store,
                [dropped],
                "翻译任务已丢弃（队列已满或会话已结束）",
            ));
        }
    }
    updates
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
    app.state::<SessionState>()
        .broadcast_segments(segments, |segments| broadcast_segments(app, segments));
}

fn broadcast_segments(app: &AppHandle, segments: Vec<TranscriptSegment>) {
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

    fn finalized(id: &str) -> TranscriptSegment {
        let mut segment = seg(id, "只有原文", 1);
        segment.state = SegmentState::Final;
        segment
    }

    #[test]
    fn translation_off_keeps_snapshot_and_export_without_fake_translation() {
        let mut store = TranscriptStore::new();
        let changed = store.apply(vec![finalized("off")]).unwrap();
        assert!(schedule_translation(&mut store, &changed, None).is_empty());
        let segment = store.get("off").unwrap();
        assert_eq!(segment.translation_state, TranslationState::Idle);
        assert_eq!(segment.translation, None);
        let markdown = crate::transcript::to_markdown(store.snapshot(), "测试时间", None);
        assert!(markdown.contains("只有原文"));
        assert!(!markdown.contains("⟦译⟧"));
        assert!(!markdown.contains("> "));
    }

    #[test]
    fn overflow_and_stop_cancel_pending_translations() {
        let mut store = TranscriptStore::new();
        let changed = store
            .apply(vec![finalized("old"), finalized("latest")])
            .unwrap();
        let queue = JobQueue::new(1);
        let updates = schedule_translation(&mut store, &changed, Some(&queue));
        assert_eq!(
            store.get("old").unwrap().translation_state,
            TranslationState::Failed
        );
        assert_eq!(
            store.get("latest").unwrap().translation_state,
            TranslationState::Pending
        );
        assert!(updates
            .iter()
            .any(|s| s.id == "old" && s.translation_state == TranslationState::Failed));

        let cancelled = cancel_translation_jobs(&mut store, queue.close_and_drain(), "已停止");
        assert_eq!(cancelled.len(), 1);
        assert!(store
            .snapshot()
            .iter()
            .all(|s| s.translation_state == TranslationState::Failed));

        let changed = store.apply(vec![finalized("after-stop")]).unwrap();
        schedule_translation(&mut store, &changed, Some(&queue));
        assert_eq!(
            store.get("after-stop").unwrap().translation_state,
            TranslationState::Failed
        );
    }

    #[test]
    fn failed_capture_start_releases_worker_for_the_next_session() {
        let (commands, receiver) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || run_translate_worker(receiver));
        let (events, _) = std::sync::mpsc::channel();
        let store = Arc::new(Mutex::new(TranscriptStore::new()));
        let start_translation = || {
            let mut translator = crate::translate::mock::MockTranslator::new(Duration::ZERO, 0);
            translator.init(&TranslateConfig::default()).unwrap();
            let (done_tx, done_rx) = std::sync::mpsc::channel();
            let session = TranslationSession {
                queue: Arc::new(JobQueue::new(4)),
                translator: Arc::new(Mutex::new(Box::new(translator))),
                done: done_rx,
            };
            commands
                .send(TranslateCommand::Start {
                    queue: Arc::clone(&session.queue),
                    translator: Arc::clone(&session.translator),
                    store: Arc::clone(&store),
                    events: events.clone(),
                    done: done_tx,
                })
                .unwrap();
            session
        };

        let failed_start = start_translation();
        drop(failed_start);
        let retry = start_translation();
        {
            let mut store = store.lock().unwrap();
            let changed = store.apply(vec![finalized("retry")]).unwrap();
            schedule_translation(&mut store, &changed, Some(&retry.queue));
        }
        retry.queue.close();
        retry
            .done
            .recv_timeout(Duration::from_secs(2))
            .expect("首次采集失败不能阻塞后续翻译");
        assert_eq!(
            store
                .lock()
                .unwrap()
                .get("retry")
                .unwrap()
                .translation_state,
            TranslationState::Done
        );
        drop(commands);
        worker.join().unwrap();
    }

    #[test]
    fn clear_waits_for_initialization_and_teardown_even_after_stop() {
        let state = SessionState::default();
        state
            .store
            .lock()
            .unwrap()
            .apply(vec![finalized("saved")])
            .unwrap();
        let epoch = state.lifecycle.lock().unwrap().begin().unwrap();
        assert!(state.clear_with(|_| panic!("启动期间不应清空")).is_err());
        state.lifecycle.lock().unwrap().stop();
        assert!(state
            .clear_with(|_| panic!("取消后初始化仍未退出"))
            .is_err());
        state.lifecycle.lock().unwrap().abort_start(epoch);
        state.teardown_active.store(true, Ordering::SeqCst);
        assert!(state.clear_with(|_| panic!("收尾期间不应清空")).is_err());
        assert_eq!(state.snapshot().len(), 1);

        state.teardown_active.store(false, Ordering::SeqCst);
        state
            .clear_with(|segments| assert!(segments.is_empty()))
            .unwrap();
        assert!(state.snapshot().is_empty());
    }

    #[test]
    fn queued_batches_use_current_revision_and_cannot_resurrect_cleared_segments() {
        let state = SessionState::default();
        let old = seg("queued", "旧草稿", 1);
        {
            let mut store = state.store.lock().unwrap();
            store.apply(vec![old.clone()]).unwrap();
            let mut latest = finalized("queued");
            latest.revision = 2;
            store.apply(vec![latest]).unwrap();
            store
                .set_translation("queued", Ok("最新译文".into()))
                .unwrap();
        }
        state.broadcast_segments(vec![old.clone(), old.clone()], |segments| {
            assert_eq!(segments.len(), 1);
            assert_eq!(segments[0].revision, 3);
            assert_eq!(segments[0].translation.as_deref(), Some("最新译文"));
        });
        let mut emissions = Vec::new();
        state
            .clear_with(|segments| emissions.push(segments))
            .unwrap();
        state.broadcast_segments(vec![old], |segments| emissions.push(segments));
        assert_eq!(emissions.len(), 1, "清空后的旧批次必须被丢弃");
        assert!(emissions[0].is_empty());
    }

    #[test]
    fn clear_is_serialized_with_a_broadcast_already_in_progress() {
        let state = Arc::new(SessionState::default());
        let segment = finalized("in-flight");
        state
            .store
            .lock()
            .unwrap()
            .apply(vec![segment.clone()])
            .unwrap();
        let emissions = Arc::new(Mutex::new(Vec::new()));
        let (broadcasting, started) = std::sync::mpsc::channel();
        let (resume, resume_receiver) = std::sync::mpsc::channel();
        let broadcast_state = Arc::clone(&state);
        let broadcast_emissions = Arc::clone(&emissions);
        let broadcaster = std::thread::spawn(move || {
            broadcast_state.broadcast_segments(vec![segment], |segments| {
                broadcasting.send(()).unwrap();
                resume_receiver.recv().unwrap();
                broadcast_emissions.lock().unwrap().push(segments);
            });
        });
        started.recv().unwrap();

        let (cleared, clear_done) = std::sync::mpsc::channel();
        let clear_state = Arc::clone(&state);
        let clear_emissions = Arc::clone(&emissions);
        let clearer = std::thread::spawn(move || {
            clear_state
                .clear_with(|segments| {
                    clear_emissions.lock().unwrap().push(segments);
                })
                .unwrap();
            cleared.send(()).unwrap();
        });
        let premature_clear = clear_done.recv_timeout(Duration::from_millis(50));
        resume.send(()).unwrap();
        broadcaster.join().unwrap();
        clearer.join().unwrap();
        assert!(premature_clear.is_err(), "清空广播必须等待在途广播完成");
        let emissions = emissions.lock().unwrap();
        assert_eq!(emissions.len(), 2);
        assert_eq!(emissions[0][0].id, "in-flight");
        assert!(emissions[1].is_empty());
    }

    #[test]
    fn stop_flush_translates_final_tail_before_export() {
        let store = Mutex::new(TranscriptStore::new());
        let changed = store
            .lock()
            .unwrap()
            .apply(vec![finalized("tail")])
            .unwrap();
        let mut translator = crate::translate::mock::MockTranslator::new(Duration::ZERO, 0);
        translator.init(&TranslateConfig::default()).unwrap();
        let translated = translate_final_tail(&store, changed, Some(&mut translator));
        assert_eq!(translated.len(), 1);
        assert_eq!(translated[0].translation_state, TranslationState::Done);
        assert!(translated[0].revision > 1);
        let store = store.lock().unwrap();
        let markdown = crate::transcript::to_markdown(store.snapshot(), "测试时间", None);
        assert!(markdown.contains("> ⟦译⟧只有原文"));
    }

    #[test]
    fn stop_flush_with_translation_off_preserves_only_source_text() {
        let store = Mutex::new(TranscriptStore::new());
        let changed = store
            .lock()
            .unwrap()
            .apply(vec![finalized("tail-off")])
            .unwrap();
        let translated = translate_final_tail(&store, changed, None);
        assert_eq!(translated[0].translation_state, TranslationState::Idle);
        assert_eq!(translated[0].translation, None);
        assert_eq!(store.lock().unwrap().get("tail-off").unwrap().revision, 1);
    }

    #[test]
    fn failed_tail_translation_terminates_pending_and_keeps_original() {
        let store = Mutex::new(TranscriptStore::new());
        let changed = store
            .lock()
            .unwrap()
            .apply(vec![finalized("tail-error")])
            .unwrap();
        let mut translator = crate::translate::mock::MockTranslator::new(Duration::ZERO, 1);
        translator.init(&TranslateConfig::default()).unwrap();
        let translated = translate_final_tail(&store, changed, Some(&mut translator));
        assert_eq!(translated[0].translation_state, TranslationState::Failed);
        assert_eq!(translated[0].text, "只有原文");
        assert_eq!(translated[0].translation, None);
    }
}
