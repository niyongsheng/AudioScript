use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use flexaudio::{DeviceInfo, OutputFormat, SourceKind, StreamConfig};
use typeshare::typeshare;

use crate::asr::TARGET_SAMPLE_RATE;

pub const CHUNK_SAMPLES: usize = 1600;

pub const SILENCE_RMS: f64 = 1e-4;

const POLL_IDLE: Duration = Duration::from_millis(5);

pub const ENV_DEVICE: &str = "AUDIOSCRIPT_DEVICE";

pub const ENV_SOURCE: &str = "AUDIOSCRIPT_SOURCE";

pub fn source_from_env() -> AudioSourceKind {
    std::env::var(ENV_SOURCE)
        .ok()
        .and_then(|v| AudioSourceKind::parse(&v))
        .unwrap_or(AudioSourceKind::Mic)
}

#[typeshare]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AudioSourceKind {
    #[default]
    Mic,
    Loopback,
    Mix,
}

impl AudioSourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mic => "mic",
            Self::Loopback => "loopback",
            Self::Mix => "mix",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "mic" => Some(Self::Mic),
            "loopback" => Some(Self::Loopback),
            "mix" => Some(Self::Mix),
            _ => None,
        }
    }

    fn to_flexaudio(self) -> SourceKind {
        match self {
            Self::Mic => SourceKind::Mic,
            Self::Loopback => SourceKind::SystemLoopback,
            Self::Mix => SourceKind::Mix,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("未找到名为 {0} 的音频设备")]
    DeviceNotFound(String),

    #[error("打开采集失败：{0}")]
    Open(String),

    #[error("启动采集失败：{0}")]
    Start(String),

    #[error("设备枚举失败：{0}")]
    Enumerate(String),

    #[error("{message}")]
    Permission {
        permission: &'static str,
        message: String,
    },
}

pub struct CaptureHandle {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    device_sample_rate: u32,
    device_name: String,
}

impl CaptureHandle {
    pub fn device_sample_rate(&self) -> u32 {
        self.device_sample_rate
    }

    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

impl Drop for CaptureHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

#[typeshare]
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputDevice {
    pub id: String,
    pub name: String,
    pub source_kind: AudioSourceKind,
    pub sample_rate: u32,
    pub channels: u16,
    pub is_default: bool,
    pub is_loopback: bool,
}

pub fn list_devices(kind: AudioSourceKind) -> Result<Vec<InputDevice>, AudioError> {
    Ok(devices_for(kind)?
        .into_iter()
        .map(to_input_device)
        .collect())
}

fn devices_for(kind: AudioSourceKind) -> Result<Vec<DeviceInfo>, AudioError> {
    let want = kind.to_flexaudio();

    #[cfg(target_os = "macos")]
    if want == SourceKind::SystemLoopback {
        return flexaudio_os_macos::list_output_devices()
            .map_err(|e| AudioError::Enumerate(e.to_string()));
    }

    flexaudio::devices()
        .map(|all| all.into_iter().filter(|d| d.source_kind == want).collect())
        .map_err(|e| AudioError::Enumerate(e.to_string()))
}

fn to_input_device(d: DeviceInfo) -> InputDevice {
    let source_kind = match d.source_kind {
        SourceKind::Mic => AudioSourceKind::Mic,
        SourceKind::SystemLoopback | SourceKind::ProcessLoopback => AudioSourceKind::Loopback,
        SourceKind::Mix => AudioSourceKind::Mix,
    };
    InputDevice {
        id: d.id,
        name: d.name,
        source_kind,
        sample_rate: d.sample_rate,
        channels: d.channels,
        is_default: d.is_default,
        is_loopback: d.is_loopback,
    }
}

fn resolve_device_name(kind: AudioSourceKind, wanted: Option<&str>) -> String {
    let Ok(devices) = devices_for(kind) else {
        return wanted.unwrap_or("(系统默认)").to_string();
    };
    let want = kind.to_flexaudio();

    let found = match wanted {
        Some(id) => devices.iter().find(|d| d.id == id),
        None => devices
            .iter()
            .find(|d| d.source_kind == want && d.is_default)
            .or_else(|| devices.iter().find(|d| d.source_kind == want)),
    };

    found
        .map(|d| d.name.clone())
        .unwrap_or_else(|| wanted.unwrap_or("(系统默认)").to_string())
}

pub fn start<S, F>(
    kind: AudioSourceKind,
    device_id: Option<&str>,
    sink: S,
    failed: F,
) -> Result<CaptureHandle, AudioError>
where
    S: FnMut(&[f32]) -> Result<(), String> + Send + 'static,
    F: FnMut(String) + Send + 'static,
{
    let config = StreamConfig {
        kind: kind.to_flexaudio(),
        device_id: device_id.map(str::to_owned),
        exclude_self: true,
        output: OutputFormat {
            sample_rate: TARGET_SAMPLE_RATE,
            channels: 1,
        },
        ..Default::default()
    };

    let stream = open_started(&config)?;
    let native_rate = stream.native_format().0;

    let device_name = resolve_device_name(kind, device_id);

    tracing::info!(
        source = kind.as_str(),
        device = %device_name,
        native_rate,
        target_rate = TARGET_SAMPLE_RATE,
        "采集已启动"
    );

    let stop = Arc::new(AtomicBool::new(false));

    let thread = spawn_worker(stream, Arc::clone(&stop), sink, failed, None)?;

    Ok(CaptureHandle {
        stop,
        thread: Some(thread),
        device_sample_rate: native_rate,
        device_name,
    })
}

fn open_started(config: &StreamConfig) -> Result<flexaudio::Stream, AudioError> {
    match open_and_start(config) {
        Ok(stream) => Ok(stream),
        #[cfg(target_os = "windows")]
        Err(e) if config.exclude_self && e.kind() == flexaudio::ErrorKind::UnsupportedOsVersion => {
            tracing::warn!("当前 Windows 版本不支持进程回环，改为采集整设备音频");
            let mut fallback = config.clone();
            fallback.exclude_self = false;
            open_and_start(&fallback).map_err(classify_open_error)
        }
        Err(e) => Err(classify_open_error(e)),
    }
}

fn open_and_start(config: &StreamConfig) -> Result<flexaudio::Stream, flexaudio::Error> {
    let mut stream = flexaudio::open(config.clone())?;
    stream.start()?;
    Ok(stream)
}

fn spawn_worker<S, F>(
    mut stream: flexaudio::Stream,
    stop: Arc<AtomicBool>,
    mut sink: S,
    mut failed: F,
    probe: Option<Arc<Mutex<ProbeAccum>>>,
) -> Result<std::thread::JoinHandle<()>, AudioError>
where
    S: FnMut(&[f32]) -> Result<(), String> + Send + 'static,
    F: FnMut(String) + Send + 'static,
{
    std::thread::Builder::new()
        .name("audio-worker".into())
        .spawn(move || {
            let mut buf: Vec<f32> = Vec::with_capacity(CHUNK_SAMPLES * 2);
            let mut failure = None;

            'capture: while !stop.load(Ordering::SeqCst) {
                while let Some(event) = stream.poll_event() {
                    if let Some(acc) = &probe {
                        acc.lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .note_event(event);
                    }
                }

                match stream.poll_chunk() {
                    Some(chunk) => {
                        if let Some(acc) = &probe {
                            acc.lock()
                                .unwrap_or_else(|e| e.into_inner())
                                .observe(&chunk.data);
                        }
                        for &s in &chunk.data {
                            buf.push(s);
                            if buf.len() == CHUNK_SAMPLES {
                                if let Err(error) = sink(&buf) {
                                    failure = Some(error);
                                    break 'capture;
                                }
                                buf.clear();
                            }
                        }
                    }
                    None => {
                        if let Some(e) = stream.terminal_error() {
                            if let Some(acc) = &probe {
                                acc.lock()
                                    .unwrap_or_else(|e| e.into_inner())
                                    .note_terminal(&e);
                            }
                            tracing::error!("采集终结性错误：{e}");
                            failure = Some(format!("音频采集中断：{e}"));
                            break;
                        }
                        std::thread::sleep(POLL_IDLE);
                    }
                }
            }

            if failure.is_none() && !buf.is_empty() {
                failure = sink(&buf).err();
            }
            if let Some(error) = failure {
                failed(error);
            }
            stream.stop();
            tracing::info!("采集工作线程退出");
        })
        .map_err(|e| AudioError::Start(e.to_string()))
}

fn classify_open_error(e: flexaudio::Error) -> AudioError {
    if let Some(permission) = e.permission() {
        return AudioError::Permission {
            permission: permission.as_str(),
            message: e.to_string(),
        };
    }

    let msg = e.to_string();
    if e.kind() == flexaudio::ErrorKind::DeviceNotFound || msg.to_lowercase().contains("not found")
    {
        return AudioError::DeviceNotFound(msg);
    }
    AudioError::Open(msg)
}

#[derive(Debug, Default)]
pub struct ProbeAccum {
    pub samples: u64,
    sum_sq: f64,
    pub peak: f32,
    pub terminal: Option<String>,
    pub denied_events: u32,
    pub silence_while_active: u32,
}

impl ProbeAccum {
    fn observe(&mut self, data: &[f32]) {
        self.samples += data.len() as u64;
        for &s in data {
            self.sum_sq += (s as f64) * (s as f64);
            let a = s.abs();
            if a > self.peak {
                self.peak = a;
            }
        }
    }

    fn note_event(&mut self, event: flexaudio::Event) {
        match event {
            flexaudio::Event::PermissionDenied { .. } => self.denied_events += 1,
            flexaudio::Event::SilenceWhileSourceActive { .. } => self.silence_while_active += 1,
            _ => {}
        }
    }

    fn note_terminal(&mut self, e: &flexaudio::Error) {
        self.terminal = Some(e.to_string());
        if e.permission() == Some(flexaudio::Permission::SystemAudio) {
            self.denied_events += 1;
        }
    }

    pub fn rms(&self) -> f64 {
        rms(self.sum_sq, self.samples as usize)
    }
}

pub fn rms(sum_sq: f64, samples: usize) -> f64 {
    if samples == 0 {
        return 0.0;
    }
    (sum_sq / samples as f64).sqrt()
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoopbackProbe {
    pub rms: f64,
    pub peak: f32,
    pub samples: u64,
    pub terminal: Option<String>,
    pub denied: bool,
}

pub fn probe_loopback<S>(window: Duration, sink: S) -> Result<LoopbackProbe, AudioError>
where
    S: FnMut(&[f32]) + Send + 'static,
{
    let config = StreamConfig {
        kind: SourceKind::SystemLoopback,
        device_id: None,
        exclude_self: true,
        output: OutputFormat {
            sample_rate: TARGET_SAMPLE_RATE,
            channels: 1,
        },
        ..Default::default()
    };

    let stream = open_started(&config)?;

    let acc = Arc::new(Mutex::new(ProbeAccum::default()));
    let stop = Arc::new(AtomicBool::new(false));

    let mut sink = sink;
    let thread = spawn_worker(
        stream,
        Arc::clone(&stop),
        move |chunk| {
            sink(chunk);
            Ok(())
        },
        |_| {},
        Some(Arc::clone(&acc)),
    )?;

    std::thread::sleep(window);
    stop.store(true, Ordering::SeqCst);
    let _ = thread.join();

    let acc = Arc::try_unwrap(acc)
        .map_err(|_| AudioError::Open("检测累加器仍被占用".into()))?
        .into_inner()
        .unwrap_or_else(|e| e.into_inner());

    Ok(LoopbackProbe {
        rms: acc.rms(),
        peak: acc.peak,
        samples: acc.samples,
        terminal: acc.terminal.clone(),
        denied: acc.denied_events > 0,
    })
}

pub fn any_output_active() -> Result<u32, AudioError> {
    let list = flexaudio::processes().map_err(classify_open_error)?;
    Ok(list
        .iter()
        .filter(|p| p.is_output_active == Some(true))
        .count() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_kind_names_are_stable() {
        assert_eq!(AudioSourceKind::Mic.as_str(), "mic");
        assert_eq!(AudioSourceKind::Loopback.as_str(), "loopback");
        assert_eq!(AudioSourceKind::Mix.as_str(), "mix");
    }

    #[test]
    fn source_kind_maps_to_flexaudio() {
        assert_eq!(AudioSourceKind::Mic.to_flexaudio(), SourceKind::Mic);
        assert_eq!(
            AudioSourceKind::Loopback.to_flexaudio(),
            SourceKind::SystemLoopback
        );
        assert_eq!(AudioSourceKind::Mix.to_flexaudio(), SourceKind::Mix);
    }

    #[test]
    fn microphone_permission_points_at_the_microphone_pane() {
        let e = classify_open_error(flexaudio::Error::PermissionDenied {
            permission: flexaudio::Permission::Microphone,
            detail: String::new(),
        });
        let AudioError::Permission {
            permission,
            message,
        } = e
        else {
            panic!("应归类为权限错误");
        };
        assert_eq!(permission, "microphone");
        assert!(message.contains("Microphone"), "{message}");
        assert!(!message.contains("System Audio Recording"), "{message}");
    }

    #[test]
    fn system_audio_permission_points_at_the_recording_pane() {
        let e = classify_open_error(flexaudio::Error::PermissionDenied {
            permission: flexaudio::Permission::SystemAudio,
            detail: String::new(),
        });
        let AudioError::Permission {
            permission,
            message,
        } = e
        else {
            panic!("应归类为权限错误");
        };
        assert_eq!(permission, "systemAudio");
        assert!(message.contains("System Audio Recording"), "{message}");
    }

    #[test]
    fn permission_survives_context_wrapping() {
        let e = flexaudio::Error::PermissionDenied {
            permission: flexaudio::Permission::SystemAudio,
            detail: String::new(),
        }
        .with_diagnostic_detail("os status -50".into());
        assert!(matches!(
            classify_open_error(e),
            AudioError::Permission { .. }
        ));
    }

    #[test]
    fn device_not_found_is_its_own_error() {
        let e = classify_open_error(flexaudio::Error::Backend("device not found: foo".into()));
        assert!(matches!(e, AudioError::DeviceNotFound(_)));
    }

    #[test]
    fn probe_accum_measures_rms_and_peak() {
        let mut acc = ProbeAccum::default();
        acc.observe(&[0.5, -0.5, 0.5, -0.5]);
        assert_eq!(acc.samples, 4);
        assert!((acc.rms() - 0.5).abs() < 1e-9);
        assert!((acc.peak - 0.5).abs() < 1e-9);

        let mut silent = ProbeAccum::default();
        silent.observe(&[0.0; 8]);
        assert_eq!(silent.samples, 8);
        assert_eq!(silent.rms(), 0.0);
        assert_eq!(silent.peak, 0.0);
    }

    #[test]
    fn empty_probe_accum_reports_zero_rms() {
        assert_eq!(ProbeAccum::default().rms(), 0.0);
    }
}
