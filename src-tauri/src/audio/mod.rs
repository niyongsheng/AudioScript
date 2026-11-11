use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::config::{normalize_device, sanitize_apps, AppRef};
use crate::i18n::{tf, tr, Key};
use flexaudio::{DeviceInfo, OutputFormat, SourceKind, StreamConfig};
use typeshare::typeshare;

use crate::asr::TARGET_SAMPLE_RATE;

#[cfg(target_os = "windows")]
mod windows_mic;

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
    App,
}

impl AudioSourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mic => "mic",
            Self::Loopback => "loopback",
            Self::Mix => "mix",
            Self::App => "app",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "mic" => Some(Self::Mic),
            "loopback" => Some(Self::Loopback),
            "mix" => Some(Self::Mix),
            "app" => Some(Self::App),
            _ => None,
        }
    }

    fn to_flexaudio(self) -> SourceKind {
        match self {
            Self::Mic => SourceKind::Mic,
            Self::Loopback => SourceKind::SystemLoopback,
            Self::Mix => SourceKind::Mix,
            Self::App => SourceKind::ProcessLoopback,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("{}", tf(Key::AudioDeviceNotFound, &[.0.as_str()]))]
    DeviceNotFound(String),

    #[error("{}", tf(Key::AudioOpenFailed, &[.0.as_str()]))]
    Open(String),

    #[error("{}", tf(Key::AudioStartFailed, &[.0.as_str()]))]
    Start(String),

    #[error("{}", tf(Key::AudioEnumerateFailed, &[.0.as_str()]))]
    Enumerate(String),

    #[error("{}", tf(Key::AudioProcessUnsupported, &[.0.as_str()]))]
    ProcessUnsupported(String),

    #[error("{}", tf(Key::AudioProcessNotFound, &[.0.as_str()]))]
    ProcessNotFound(String),

    #[error("{}", tr(Key::AudioAppNotSelected))]
    AppNotSelected,

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

    pub fn is_finished(&self) -> bool {
        self.thread.as_ref().is_none_or(|t| t.is_finished())
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
    if kind == AudioSourceKind::App {
        return Ok(Vec::new());
    }
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

#[typeshare]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioProcessInfo {
    pub key: String,
    pub pid: u32,
    pub name: String,
    pub executable: Option<String>,
    pub bundle_id: Option<String>,
    pub is_output_active: Option<bool>,
}

static PROCESS_ENUM_LOCK: Mutex<()> = Mutex::new(());

fn enumerate_processes() -> Result<Vec<flexaudio::ProcessInfo>, flexaudio::Error> {
    let _guard = PROCESS_ENUM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    flexaudio::processes()
}

pub fn list_processes() -> Result<Vec<AudioProcessInfo>, AudioError> {
    let list = enumerate_processes().map_err(classify_process_error)?;
    #[cfg(target_os = "macos")]
    let own_name = own_app_name();
    let mut out = Vec::new();
    for p in list {
        if !is_app_process(&p) {
            continue;
        }
        let name = process_display_name(&p);
        #[cfg(target_os = "macos")]
        if own_name.as_deref() == Some(name.as_str()) {
            continue;
        }
        let Some(key) = process_key_with_name(&p, &name) else {
            continue;
        };
        push_process(
            &mut out,
            AudioProcessInfo {
                key,
                pid: p.pid,
                name,
                executable: p.executable,
                bundle_id: p.bundle_id,
                is_output_active: p.is_output_active,
            },
        );
    }
    Ok(out)
}

fn push_process(out: &mut Vec<AudioProcessInfo>, info: AudioProcessInfo) {
    if let Some(existing) = out.iter_mut().find(|e| e.name == info.name) {
        if info.is_output_active == Some(true) && existing.is_output_active != Some(true) {
            *existing = info;
        }
        return;
    }
    out.push(info);
}

fn is_app_process(p: &flexaudio::ProcessInfo) -> bool {
    #[cfg(target_os = "macos")]
    {
        let Some(app) =
            objc2_app_kit::NSRunningApplication::runningApplicationWithProcessIdentifier(
                p.pid as i32,
            )
        else {
            return false;
        };
        if app.activationPolicy() == objc2_app_kit::NSApplicationActivationPolicy::Regular {
            return true;
        }
        let Some(name) = app.localizedName().map(|name| name.to_string()) else {
            return false;
        };
        return regular_app_names()
            .iter()
            .any(|host| name.starts_with(host.as_str()));
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = p;
        true
    }
}

fn classify_process_error(e: flexaudio::Error) -> AudioError {
    if let Some(permission) = e.permission() {
        return AudioError::Permission {
            permission: permission.as_str(),
            message: e.to_string(),
        };
    }
    AudioError::ProcessUnsupported(e.to_string())
}

fn process_display_name(p: &flexaudio::ProcessInfo) -> String {
    #[cfg(target_os = "macos")]
    if let Some(raw) = localized_process_name(p.pid) {
        let hosts = regular_app_names();
        if let Some(host) = hosts
            .iter()
            .filter(|host| raw.starts_with(host.as_str()))
            .max_by_key(|host| host.len())
        {
            return host.clone();
        }
        return raw;
    }
    p.name.clone()
}

#[cfg(target_os = "macos")]
fn regular_app_names() -> Vec<String> {
    let workspace = objc2_app_kit::NSWorkspace::sharedWorkspace();
    workspace
        .runningApplications()
        .iter()
        .filter(|app| {
            app.activationPolicy() == objc2_app_kit::NSApplicationActivationPolicy::Regular
        })
        .filter_map(|app| app.localizedName().map(|name| name.to_string()))
        .collect()
}

#[cfg(target_os = "macos")]
fn own_app_name() -> Option<String> {
    let app = objc2_app_kit::NSRunningApplication::currentApplication();
    app.localizedName().map(|name| name.to_string())
}

#[cfg(target_os = "macos")]
fn localized_process_name(pid: u32) -> Option<String> {
    let app =
        objc2_app_kit::NSRunningApplication::runningApplicationWithProcessIdentifier(pid as i32)?;
    let name = app.localizedName()?;
    let name = name.to_string();
    let trimmed = name.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

fn process_key(p: &flexaudio::ProcessInfo) -> Option<String> {
    process_key_with_name(p, &process_display_name(p))
}

fn process_key_with_name(p: &flexaudio::ProcessInfo, name: &str) -> Option<String> {
    #[cfg(not(target_os = "macos"))]
    let _ = name;

    #[cfg(target_os = "macos")]
    if let Some(bundle) = p
        .bundle_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if bundle.starts_with("com.apple.WebKit.") || bundle.contains(".helper") {
            return Some(format!("bundle:{bundle}|name:{name}"));
        }
        return Some(format!("bundle:{bundle}"));
    }

    let exe = p
        .executable
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())?;
    #[cfg(target_os = "windows")]
    {
        return Some(format!("exe:{}", exe.to_ascii_lowercase()));
    }
    #[cfg(not(target_os = "windows"))]
    {
        Some(format!("exe:{exe}"))
    }
}

fn pick_process<'a>(
    list: &'a [flexaudio::ProcessInfo],
    key: &str,
) -> Option<&'a flexaudio::ProcessInfo> {
    let mut found: Option<&flexaudio::ProcessInfo> = None;
    for p in list {
        if process_key(p).as_deref() != Some(key) {
            continue;
        }
        let active = p.is_output_active == Some(true);
        match found {
            None => found = Some(p),
            Some(best) if active && best.is_output_active != Some(true) => found = Some(p),
            _ => {}
        }
    }
    found
}

fn pick_process_by_name<'a>(
    list: &'a [flexaudio::ProcessInfo],
    name: &str,
) -> Option<&'a flexaudio::ProcessInfo> {
    let mut found: Option<&flexaudio::ProcessInfo> = None;
    for p in list {
        if process_display_name(p) != name {
            continue;
        }
        let active = p.is_output_active == Some(true);
        match found {
            None => found = Some(p),
            Some(best) if active && best.is_output_active != Some(true) => found = Some(p),
            _ => {}
        }
    }
    found
}

fn resolve_apps(apps: &[AppRef]) -> Result<Vec<(AppRef, u32)>, AudioError> {
    let list = enumerate_processes().map_err(classify_process_error)?;
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for app in apps {
        let pid = pick_process(&list, &app.key)
            .or_else(|| pick_process_by_name(&list, &app.name))
            .map(|p| p.pid)
            .ok_or_else(|| AudioError::ProcessNotFound(app.name.clone()))?;
        if seen.insert(pid) {
            out.push((app.clone(), pid));
        }
    }
    Ok(out)
}

fn resolve_device_name(kind: AudioSourceKind, wanted: Option<&str>) -> String {
    let Ok(devices) = devices_for(kind) else {
        return wanted.unwrap_or(tr(Key::AudioDefaultDevice)).to_string();
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
        .unwrap_or_else(|| wanted.unwrap_or(tr(Key::AudioDefaultDevice)).to_string())
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceSelection {
    pub device: Option<String>,
    pub mix_mic: Option<String>,
    pub mix_system: Option<String>,
    pub apps: Vec<AppRef>,
}

impl DeviceSelection {
    pub fn single(device: Option<String>) -> Self {
        Self {
            device: normalize_device(device),
            ..Self::default()
        }
    }

    pub fn mix(mic: Option<String>, system: Option<String>) -> Self {
        Self {
            device: None,
            mix_mic: normalize_device(mic),
            mix_system: normalize_device(system),
            ..Self::default()
        }
    }

    pub fn with_apps(mut self, apps: Vec<AppRef>) -> Self {
        self.apps = sanitize_apps(apps);
        self
    }
}

fn display_name(kind: AudioSourceKind, selection: &DeviceSelection) -> String {
    if kind == AudioSourceKind::App {
        if selection.apps.is_empty() {
            return tr(Key::AudioAppDefault).to_string();
        }
        return selection
            .apps
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>()
            .join(" + ");
    }
    if kind == AudioSourceKind::Mix {
        return format!(
            "{} + {}",
            resolve_device_name(AudioSourceKind::Mic, selection.mix_mic.as_deref()),
            resolve_device_name(AudioSourceKind::Loopback, selection.mix_system.as_deref())
        );
    }
    resolve_device_name(kind, selection.device.as_deref())
}

pub fn start<S>(
    kind: AudioSourceKind,
    selection: &DeviceSelection,
    sink: S,
) -> Result<CaptureHandle, AudioError>
where
    S: FnMut(&[f32]) + Send + 'static,
{
    #[cfg(target_os = "windows")]
    if let Some(probe) = windows_mic::probe(kind, selection) {
        let handle = windows_mic::start(kind, selection, probe, sink)?;
        tracing::info!(
            source = kind.as_str(),
            device = %handle.device_name(),
            native_rate = handle.device_sample_rate(),
            target_rate = TARGET_SAMPLE_RATE,
            "采集已启动"
        );
        return Ok(handle);
    }

    if kind == AudioSourceKind::App {
        return start_apps(selection, sink);
    }

    let config = StreamConfig {
        kind: kind.to_flexaudio(),
        device_id: selection.device.clone(),
        mix_mic_device_id: selection.mix_mic.clone(),
        mix_system_device_id: selection.mix_system.clone(),
        exclude_self: true,
        output: OutputFormat {
            sample_rate: TARGET_SAMPLE_RATE,
            channels: 1,
        },
        ..Default::default()
    };

    let stream = open_started(&config)?;
    let native_rate = stream.native_format().0;

    let device_name = display_name(kind, selection);

    tracing::info!(
        source = kind.as_str(),
        device = %device_name,
        native_rate,
        target_rate = TARGET_SAMPLE_RATE,
        "采集已启动"
    );

    let stop = Arc::new(AtomicBool::new(false));

    let thread = spawn_worker(stream, Arc::clone(&stop), sink, None)?;

    Ok(CaptureHandle {
        stop,
        thread: Some(thread),
        device_sample_rate: native_rate,
        device_name,
    })
}

fn start_apps<S>(selection: &DeviceSelection, sink: S) -> Result<CaptureHandle, AudioError>
where
    S: FnMut(&[f32]) + Send + 'static,
{
    if selection.apps.is_empty() {
        return Err(AudioError::AppNotSelected);
    }
    let resolved = resolve_apps(&selection.apps)?;
    let mut streams: Vec<flexaudio::Stream> = Vec::new();
    let mut names = Vec::new();
    let mut native_rate = 0;
    for (app, pid) in resolved {
        match open_started(&app_stream_config(pid)) {
            Ok(stream) => {
                if native_rate == 0 {
                    native_rate = stream.native_format().0;
                }
                tracing::info!(app = %app.name, key = %app.key, pid, "目标应用已解析");
                names.push(app.name);
                streams.push(stream);
            }
            Err(AudioError::DeviceNotFound(_)) => {
                for stream in &mut streams {
                    stream.stop();
                }
                return Err(AudioError::ProcessNotFound(app.name));
            }
            Err(e) => {
                for stream in &mut streams {
                    stream.stop();
                }
                return Err(e);
            }
        }
    }
    if streams.is_empty() {
        return Err(AudioError::AppNotSelected);
    }

    let device_name = names.join(" + ");
    tracing::info!(
        source = AudioSourceKind::App.as_str(),
        device = %device_name,
        streams = streams.len(),
        native_rate,
        target_rate = TARGET_SAMPLE_RATE,
        "采集已启动"
    );

    let stop = Arc::new(AtomicBool::new(false));
    let thread = spawn_mixer(streams, Arc::clone(&stop), sink)?;

    Ok(CaptureHandle {
        stop,
        thread: Some(thread),
        device_sample_rate: native_rate,
        device_name,
    })
}

fn app_stream_config(pid: u32) -> StreamConfig {
    StreamConfig {
        kind: SourceKind::ProcessLoopback,
        target_pid: Some(pid),
        exclude_self: true,
        output: OutputFormat {
            sample_rate: TARGET_SAMPLE_RATE,
            channels: 1,
        },
        ..Default::default()
    }
}

fn open_started(config: &StreamConfig) -> Result<flexaudio::Stream, AudioError> {
    match open_and_start(config) {
        Ok(stream) => Ok(stream),
        #[cfg(target_os = "windows")]
        Err(e)
            if config.exclude_self
                && config.kind != SourceKind::ProcessLoopback
                && e.kind() == flexaudio::ErrorKind::UnsupportedOsVersion =>
        {
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

fn spawn_worker<S>(
    mut stream: flexaudio::Stream,
    stop: Arc<AtomicBool>,
    mut sink: S,
    probe: Option<Arc<Mutex<ProbeAccum>>>,
) -> Result<std::thread::JoinHandle<()>, AudioError>
where
    S: FnMut(&[f32]) + Send + 'static,
{
    std::thread::Builder::new()
        .name("audio-worker".into())
        .spawn(move || {
            let mut buf: Vec<f32> = Vec::with_capacity(CHUNK_SAMPLES * 2);

            while !stop.load(Ordering::SeqCst) {
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
                                sink(&buf);
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
                            break;
                        }
                        std::thread::sleep(POLL_IDLE);
                    }
                }
            }

            stream.stop();
            tracing::info!("采集工作线程退出");
        })
        .map_err(|e| AudioError::Start(e.to_string()))
}

const MIX_TICK: Duration = Duration::from_millis(20);

const MIX_QUEUE_CHUNKS: usize = 10;

const MIX_FRAMES: usize = (TARGET_SAMPLE_RATE as usize) / 50;

struct MixInput {
    stream: flexaudio::Stream,
    queue: VecDeque<f32>,
    dead: bool,
}

fn spawn_mixer<S>(
    streams: Vec<flexaudio::Stream>,
    stop: Arc<AtomicBool>,
    mut sink: S,
) -> Result<std::thread::JoinHandle<()>, AudioError>
where
    S: FnMut(&[f32]) + Send + 'static,
{
    std::thread::Builder::new()
        .name("audio-mixer".into())
        .spawn(move || {
            let mut inputs: Vec<MixInput> = streams
                .into_iter()
                .map(|stream| MixInput {
                    stream,
                    queue: VecDeque::new(),
                    dead: false,
                })
                .collect();
            let mut out: Vec<f32> = Vec::with_capacity(CHUNK_SAMPLES);
            let mut next_tick = Instant::now();

            while !stop.load(Ordering::SeqCst) {
                let mut alive = false;
                for input in &mut inputs {
                    if input.dead {
                        continue;
                    }
                    while let Some(chunk) = input.stream.poll_chunk() {
                        input.queue.extend(chunk.data);
                    }
                    let limit = MIX_QUEUE_CHUNKS * MIX_FRAMES;
                    if input.queue.len() > limit {
                        let excess = input.queue.len() - limit;
                        input.queue.drain(..excess);
                    }
                    if let Some(e) = input.stream.terminal_error() {
                        tracing::error!("多路采集中的一路终止：{e}");
                        input.dead = true;
                        input.queue.clear();
                        continue;
                    }
                    alive = true;
                }
                if !alive {
                    break;
                }

                for _ in 0..MIX_FRAMES {
                    let mut sample = 0.0f32;
                    for input in &mut inputs {
                        if let Some(s) = input.queue.pop_front() {
                            sample += s;
                        }
                    }
                    out.push(sample.clamp(-1.0, 1.0));
                    if out.len() == CHUNK_SAMPLES {
                        sink(&out);
                        out.clear();
                    }
                }

                next_tick += MIX_TICK;
                let now = Instant::now();
                if next_tick > now {
                    std::thread::sleep(next_tick - now);
                } else {
                    next_tick = now;
                }
            }

            for input in &mut inputs {
                input.stream.stop();
            }
            tracing::info!("多路采集工作线程退出");
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

    let thread = spawn_worker(stream, Arc::clone(&stop), sink, Some(Arc::clone(&acc)))?;

    std::thread::sleep(window);
    stop.store(true, Ordering::SeqCst);
    let _ = thread.join();

    let acc = Arc::try_unwrap(acc)
        .map_err(|_| AudioError::Open(tr(Key::AudioAccumulatorBusy).into()))?
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
    let list = enumerate_processes().map_err(classify_open_error)?;
    Ok(list
        .iter()
        .filter(|p| p.is_output_active == Some(true))
        .count() as u32)
}

#[cfg(any(target_os = "windows", test))]
fn mix_mic_and_system(
    mic: &mut VecDeque<f32>,
    system: &mut VecDeque<f32>,
    frames: usize,
) -> Vec<f32> {
    let mut out = Vec::with_capacity(frames);
    for _ in 0..frames {
        let m = mic.pop_front().unwrap_or(0.0);
        let s = system.pop_front().unwrap_or(0.0);
        out.push((m + s).clamp(-1.0, 1.0));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_kind_names_are_stable() {
        assert_eq!(AudioSourceKind::Mic.as_str(), "mic");
        assert_eq!(AudioSourceKind::Loopback.as_str(), "loopback");
        assert_eq!(AudioSourceKind::Mix.as_str(), "mix");
        assert_eq!(AudioSourceKind::App.as_str(), "app");
    }

    #[test]
    fn source_kind_maps_to_flexaudio() {
        assert_eq!(AudioSourceKind::Mic.to_flexaudio(), SourceKind::Mic);
        assert_eq!(
            AudioSourceKind::Loopback.to_flexaudio(),
            SourceKind::SystemLoopback
        );
        assert_eq!(AudioSourceKind::Mix.to_flexaudio(), SourceKind::Mix);
        assert_eq!(
            AudioSourceKind::App.to_flexaudio(),
            SourceKind::ProcessLoopback
        );
    }

    #[test]
    fn device_selection_normalizes_empty_ids() {
        assert_eq!(
            DeviceSelection::single(Some(String::new())),
            DeviceSelection::default()
        );
        let mix = DeviceSelection::mix(Some("mic-1".into()), Some(String::new()));
        assert_eq!(mix.device, None);
        assert_eq!(mix.mix_mic.as_deref(), Some("mic-1"));
        assert_eq!(mix.mix_system, None);
    }

    #[test]
    fn device_selection_drops_invalid_app_key() {
        let app = AppRef {
            key: "pid:42".into(),
            name: "App".into(),
        };
        assert!(
            DeviceSelection::single(None)
                .with_apps(vec![app])
                .apps
                .is_empty(),
            "非法身份必须被丢弃，不能拿去匹配"
        );
    }

    #[test]
    fn process_key_is_stable() {
        let p = flexaudio::ProcessInfo {
            pid: 42,
            name: "App".into(),
            executable: Some("app.exe".into()),
            bundle_id: Some("com.example.app".into()),
            is_output_active: None,
        };
        #[cfg(target_os = "macos")]
        assert_eq!(process_key(&p).as_deref(), Some("bundle:com.example.app"));
        #[cfg(not(target_os = "macos"))]
        assert_eq!(process_key(&p).as_deref(), Some("exe:app.exe"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn webkit_helper_keys_include_host_name() {
        let p = flexaudio::ProcessInfo {
            pid: 42,
            name: "com.apple.WebKit.GPU".into(),
            executable: Some("com.apple.WebKit.GPU".into()),
            bundle_id: Some("com.apple.WebKit.GPU".into()),
            is_output_active: Some(true),
        };
        assert_eq!(
            process_key_with_name(&p, "Safari浏览器").as_deref(),
            Some("bundle:com.apple.WebKit.GPU|name:Safari浏览器")
        );
    }

    #[test]
    fn duplicate_app_names_prefer_active_entry() {
        let idle = AudioProcessInfo {
            key: "bundle:com.example.app".into(),
            pid: 1,
            name: "App".into(),
            executable: None,
            bundle_id: Some("com.example.app".into()),
            is_output_active: Some(false),
        };
        let active = AudioProcessInfo {
            key: "bundle:com.example.app.helper".into(),
            pid: 2,
            is_output_active: Some(true),
            ..idle.clone()
        };
        let mut out = Vec::new();
        push_process(&mut out, idle);
        push_process(&mut out, active);
        assert_eq!(out.len(), 1, "同名应用只能保留一条");
        assert_eq!(out[0].key, "bundle:com.example.app.helper");
    }

    #[test]
    fn pick_process_prefers_active_process() {
        let idle = flexaudio::ProcessInfo {
            pid: 1,
            name: "App".into(),
            executable: Some("app.exe".into()),
            bundle_id: None,
            is_output_active: Some(false),
        };
        let active = flexaudio::ProcessInfo {
            pid: 2,
            is_output_active: Some(true),
            ..idle.clone()
        };
        let list = vec![idle, active];
        assert_eq!(pick_process(&list, "exe:app.exe").map(|p| p.pid), Some(2));
        assert!(pick_process(&list, "exe:missing").is_none());
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

    #[test]
    fn mix_pads_the_silent_side_with_zeros() {
        let mut mic = VecDeque::from(vec![0.25, 0.25]);
        let mut system = VecDeque::new();
        let mixed = mix_mic_and_system(&mut mic, &mut system, 3);
        assert_eq!(mixed, vec![0.25, 0.25, 0.0]);
        assert!(mic.is_empty() && system.is_empty());
    }

    #[test]
    fn mix_clamps_overlapping_speech() {
        let mut mic = VecDeque::from(vec![0.8, -0.8]);
        let mut system = VecDeque::from(vec![0.8, -0.8]);
        let mixed = mix_mic_and_system(&mut mic, &mut system, 2);
        assert_eq!(mixed, vec![1.0, -1.0]);
    }
}
