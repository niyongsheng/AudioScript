use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use crate::audio::{self, AudioError, AudioSourceKind, CaptureHandle};
use crate::commands;
use crate::session::SessionState;

pub const EVENT_DEVICE_LEVEL: &str = "transcript://device-level";

const LEVEL_CHUNKS: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub source: AudioSourceKind,
    pub device: Option<String>,
}

impl Target {
    pub fn new(source: AudioSourceKind, device: Option<&str>) -> Self {
        Self {
            source,
            device: crate::config::normalize_device(device.map(str::to_owned)),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceLevel {
    source: AudioSourceKind,
    device: Option<String>,
    rms: f64,
}

struct Running {
    target: Target,
    capture: CaptureHandle,
}

struct Inner {
    desired: Vec<Target>,
    running: Vec<Running>,
    visible: bool,
}

impl Inner {
    fn take_running(&mut self) -> Vec<Running> {
        std::mem::take(&mut self.running)
    }
}

fn pending_targets(inner: &Inner) -> Vec<Target> {
    inner
        .desired
        .iter()
        .filter(|t| !inner.running.iter().any(|r| &r.target == *t))
        .cloned()
        .collect()
}

pub struct MonitorState {
    inner: Mutex<Inner>,
}

impl Default for MonitorState {
    fn default() -> Self {
        Self {
            inner: Mutex::new(Inner {
                desired: Vec::new(),
                running: Vec::new(),
                visible: true,
            }),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MonitorError {
    #[error(transparent)]
    Audio(#[from] AudioError),
    #[error("{0}")]
    Busy(String),
}

#[derive(Default)]
struct LevelWindow {
    sum_sq: f64,
    samples: usize,
    chunks: u32,
}

impl LevelWindow {
    fn observe(&mut self, chunk: &[f32]) -> Option<f64> {
        self.sum_sq += chunk.iter().map(|s| (*s as f64) * (*s as f64)).sum::<f64>();
        self.samples += chunk.len();
        self.chunks += 1;
        if self.chunks < LEVEL_CHUNKS {
            return None;
        }
        let rms = audio::rms(self.sum_sq, self.samples);
        self.sum_sq = 0.0;
        self.samples = 0;
        self.chunks = 0;
        Some(rms)
    }
}

fn normalize_targets(targets: &[Target]) -> Vec<Target> {
    let mut out: Vec<Target> = Vec::with_capacity(targets.len());
    for target in targets {
        let normalized = Target::new(target.source, target.device.as_deref());
        if !out.contains(&normalized) {
            out.push(normalized);
        }
    }
    out
}

fn drop_running(running: Vec<Running>) {
    if running.is_empty() {
        return;
    }
    let count = running.len();
    drop(running);
    tracing::info!(count, "音频预览已停止");
}

pub fn start(app: &AppHandle, targets: &[Target]) -> Result<(), MonitorError> {
    let monitor = app.state::<MonitorState>();
    monitor
        .inner
        .lock()
        .expect("monitor mutex poisoned")
        .desired = normalize_targets(targets);
    reconcile(app)
}

pub fn stop(app: &AppHandle) {
    let monitor = app.state::<MonitorState>();
    let running = {
        let mut inner = monitor.inner.lock().expect("monitor mutex poisoned");
        inner.desired.clear();
        inner.take_running()
    };
    drop_running(running);
}

pub fn suspend(app: &AppHandle) {
    let monitor = app.state::<MonitorState>();
    let running = {
        let mut inner = monitor.inner.lock().expect("monitor mutex poisoned");
        inner.take_running()
    };
    drop_running(running);
}

pub fn resume(app: &AppHandle) {
    if let Err(e) = reconcile(app) {
        tracing::warn!("恢复音频预览失败：{e}");
    }
}

pub fn on_window_hidden(app: &AppHandle) {
    let monitor = app.state::<MonitorState>();
    let running = {
        let mut inner = monitor.inner.lock().expect("monitor mutex poisoned");
        inner.visible = false;
        inner.take_running()
    };
    drop_running(running);
}

pub fn on_window_shown(app: &AppHandle) {
    let monitor = app.state::<MonitorState>();
    monitor
        .inner
        .lock()
        .expect("monitor mutex poisoned")
        .visible = true;
    let handle = app.clone();
    commands::enqueue_session(move || resume(&handle));
}

fn reconcile(app: &AppHandle) -> Result<(), MonitorError> {
    let monitor = app.state::<MonitorState>();
    let session = app.state::<SessionState>();
    let session_running = session.is_running();

    let (pending, stale) = {
        let mut inner = monitor.inner.lock().expect("monitor mutex poisoned");
        if inner.desired.is_empty() || !inner.visible || session_running {
            let running = inner.take_running();
            drop(inner);
            drop_running(running);
            return Ok(());
        }

        let mut stale = Vec::new();
        let mut keep = Vec::with_capacity(inner.running.len());
        for running in std::mem::take(&mut inner.running) {
            if inner.desired.contains(&running.target) && !running.capture.is_finished() {
                keep.push(running);
            } else {
                stale.push(running);
            }
        }
        inner.running = keep;

        (pending_targets(&inner), stale)
    };
    drop_running(stale);
    if pending.is_empty() {
        return Ok(());
    }

    if !session.wait_until_idle(Duration::from_secs(5)) {
        return Err(MonitorError::Busy(
            crate::i18n::tr(crate::i18n::Key::SessionTeardownBusy).into(),
        ));
    }

    let pending = {
        let inner = monitor.inner.lock().expect("monitor mutex poisoned");
        if !inner.visible || session.is_running() {
            return Ok(());
        }
        pending_targets(&inner)
    };

    let mut failures: Vec<AudioError> = Vec::new();
    for target in pending {
        let level_app = app.clone();
        let level_target = target.clone();
        let mut window = LevelWindow::default();
        let selection = audio::DeviceSelection::single(target.device.clone());
        let started = audio::start(target.source, &selection, move |chunk| {
            if let Some(rms) = window.observe(chunk) {
                let level = DeviceLevel {
                    source: level_target.source,
                    device: level_target.device.clone(),
                    rms,
                };
                if let Err(e) = level_app.emit(EVENT_DEVICE_LEVEL, level) {
                    tracing::debug!("广播预览电平失败（可能没有窗口在监听）：{e}");
                }
            }
        });

        match started {
            Ok(capture) => {
                let mut inner = monitor.inner.lock().expect("monitor mutex poisoned");
                let accepted = inner.visible
                    && !session.is_running()
                    && inner.desired.contains(&target)
                    && !inner.running.iter().any(|r| r.target == target);
                if accepted {
                    tracing::info!(
                        source = target.source.as_str(),
                        device = target.device.as_deref().unwrap_or("(系统默认)"),
                        "音频预览已启动"
                    );
                    inner.running.push(Running { target, capture });
                }
            }
            Err(e) => {
                tracing::warn!(
                    source = target.source.as_str(),
                    device = target.device.as_deref().unwrap_or("(系统默认)"),
                    "启动音频预览失败：{e}"
                );
                failures.push(e);
            }
        }
    }

    let has_running = {
        let inner = monitor.inner.lock().expect("monitor mutex poisoned");
        !inner.running.is_empty()
    };
    if has_running || failures.is_empty() {
        Ok(())
    } else {
        Err(failures.remove(0).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_normalizes_empty_device() {
        assert_eq!(
            Target::new(AudioSourceKind::Mic, Some("")),
            Target {
                source: AudioSourceKind::Mic,
                device: None,
            }
        );
        assert_eq!(
            Target::new(AudioSourceKind::Loopback, Some("dev-1")).device,
            Some("dev-1".into())
        );
    }

    #[test]
    fn targets_dedupe_and_keep_order() {
        let list = vec![
            Target::new(AudioSourceKind::Mic, Some("dev-1")),
            Target::new(AudioSourceKind::Mic, Some("dev-2")),
            Target::new(AudioSourceKind::Mic, Some("dev-1")),
        ];
        assert_eq!(
            normalize_targets(&list),
            vec![
                Target {
                    source: AudioSourceKind::Mic,
                    device: Some("dev-1".into())
                },
                Target {
                    source: AudioSourceKind::Mic,
                    device: Some("dev-2".into())
                },
            ]
        );
        assert!(normalize_targets(&[]).is_empty());
        assert_eq!(
            normalize_targets(&[Target::new(AudioSourceKind::Mic, Some(""))]),
            vec![Target {
                source: AudioSourceKind::Mic,
                device: None
            }]
        );
    }

    #[test]
    fn targets_keep_the_same_device_id_per_source() {
        let list = vec![
            Target::new(AudioSourceKind::Mic, Some("dev-1")),
            Target::new(AudioSourceKind::Loopback, Some("dev-1")),
        ];
        assert_eq!(normalize_targets(&list).len(), 2);
    }

    #[test]
    fn level_window_emits_every_two_chunks() {
        let mut window = LevelWindow::default();
        assert_eq!(window.observe(&[0.5; 1600]), None);
        let rms = window
            .observe(&[0.5; 1600])
            .expect("第二个 chunk 应产出 RMS");
        assert!((rms - 0.5).abs() < 1e-9);
        assert_eq!(window.observe(&[0.0; 1600]), None);
        assert_eq!(window.observe(&[0.0; 1600]), Some(0.0));
    }
}
