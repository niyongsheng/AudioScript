use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, Emitter};
use typeshare::typeshare;

use crate::audio;
use crate::i18n::{tr, Key};

#[cfg(target_os = "macos")]
mod macos;

pub const PROBE_WINDOW: Duration = Duration::from_millis(5500);

static PROBING: Mutex<()> = Mutex::new(());

#[typeshare]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MicPermission {
    Granted,
    Denied,
    Restricted,
    NotDetermined,
    NotApplicable,
    Unknown,
}

#[typeshare]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SystemAudioVerdict {
    Audible,
    SilentWhilePlaying,
    TapRejected,
    NoAudioPlaying,
    UnsupportedOs,
    Inconclusive,
}

#[typeshare]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionSnapshot {
    pub microphone: MicPermission,
    pub microphone_promptable: bool,
}

#[typeshare]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MicRequestResult {
    pub state: MicPermission,
    pub timed_out: bool,
}

#[typeshare]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemAudioProbe {
    pub verdict: SystemAudioVerdict,
    pub rms: f64,
    pub peak: f32,
    pub samples: u32,
    pub active_processes: Option<u32>,
    pub detail: Option<String>,
}

impl SystemAudioProbe {
    fn inconclusive(active_processes: Option<u32>, detail: impl Into<String>) -> Self {
        Self {
            verdict: SystemAudioVerdict::Inconclusive,
            rms: 0.0,
            peak: 0.0,
            samples: 0,
            active_processes,
            detail: Some(detail.into()),
        }
    }
}

pub fn snapshot() -> PermissionSnapshot {
    PermissionSnapshot {
        microphone: mic_state(),
        microphone_promptable: mic_promptable(),
    }
}

pub fn request_mic() -> Result<MicRequestResult, String> {
    request_mic_impl()
}

pub fn probe_system_audio(app: &AppHandle) -> SystemAudioProbe {
    let _guard = match PROBING.try_lock() {
        Ok(g) => g,
        Err(_) => return SystemAudioProbe::inconclusive(None, tr(Key::PermissionProbing)),
    };

    let active = match audio::any_output_active() {
        Ok(n) => Some(n),
        Err(e) => {
            if let audio::AudioError::Open(msg) = &e {
                if msg.to_lowercase().contains("unsupported os") {
                    return SystemAudioProbe {
                        verdict: SystemAudioVerdict::UnsupportedOs,
                        rms: 0.0,
                        peak: 0.0,
                        samples: 0,
                        active_processes: None,
                        detail: Some(msg.clone()),
                    };
                }
            }
            None
        }
    };

    let mut attempts = 0;
    let probe = loop {
        attempts += 1;
        let level_app = app.clone();
        match audio::probe_loopback(PROBE_WINDOW, move |chunk| emit_level(&level_app, chunk)) {
            Ok(p) => {
                let prompt_swallowed_the_window =
                    p.samples == 0 && attempts < 2 && audio_capture_promptable();
                if prompt_swallowed_the_window {
                    tracing::info!("首次创建 tap 可能刚触发系统授权弹窗，重测一次");
                    continue;
                }
                break p;
            }
            Err(e) => return SystemAudioProbe::inconclusive(active, e.to_string()),
        }
    };

    let verdict = classify_system_audio(
        probe.peak,
        probe.samples.min(u32::MAX as u64) as u32,
        probe.denied,
        active,
    );

    let detail = probe.terminal.clone().or_else(|| {
        (verdict == SystemAudioVerdict::Inconclusive)
            .then(|| inconclusive_reason(probe.samples, audio_capture_promptable()))
    });

    SystemAudioProbe {
        verdict,
        rms: probe.rms,
        peak: probe.peak,
        samples: probe.samples.min(u32::MAX as u64) as u32,
        active_processes: active,
        detail,
    }
}

fn emit_level(app: &AppHandle, chunk: &[f32]) {
    if chunk.is_empty() {
        return;
    }
    let mean_sq = chunk.iter().map(|s| (*s as f64) * (*s as f64)).sum::<f64>() / chunk.len() as f64;
    if let Err(e) = app.emit(crate::session::EVENT_LEVEL, mean_sq.sqrt()) {
        tracing::debug!("广播检测电平失败（可能没有窗口在监听）：{e}");
    }
}

fn inconclusive_reason(samples: u64, promptable: bool) -> String {
    if samples > 0 {
        return tr(Key::PermissionNoIo).to_string();
    }
    if promptable {
        tr(Key::PermissionPromptUnhandled).to_string()
    } else {
        tr(Key::PermissionUnbundled).to_string()
    }
}

#[cfg(not(target_os = "macos"))]
fn audio_capture_promptable() -> bool {
    false
}

#[cfg(target_os = "macos")]
fn audio_capture_promptable() -> bool {
    macos::has_usage_description("NSAudioCaptureUsageDescription")
}

pub fn classify_system_audio(
    peak: f32,
    samples: u32,
    denied: bool,
    active_processes: Option<u32>,
) -> SystemAudioVerdict {
    if denied {
        return SystemAudioVerdict::TapRejected;
    }
    if peak > 0.0 {
        return SystemAudioVerdict::Audible;
    }
    if samples == 0 {
        return SystemAudioVerdict::Inconclusive;
    }
    match active_processes {
        Some(n) if n >= 1 => SystemAudioVerdict::SilentWhilePlaying,
        Some(0) => SystemAudioVerdict::NoAudioPlaying,
        _ => SystemAudioVerdict::Inconclusive,
    }
}

pub fn certain_denial(source: audio::AudioSourceKind) -> Option<&'static str> {
    let needs_mic = matches!(
        source,
        audio::AudioSourceKind::Mic | audio::AudioSourceKind::Mix
    );
    if !needs_mic {
        return None;
    }
    match mic_state() {
        MicPermission::Denied | MicPermission::Restricted => Some("microphone"),
        _ => None,
    }
}

pub fn settings_url(pane: &str) -> Option<&'static str> {
    match pane {
        "microphone" => {
            Some("x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone")
        }
        "systemAudio" => {
            Some("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
        }
        "speech" => {
            Some("x-apple.systempreferences:com.apple.preference.security?Privacy_SpeechRecognition")
        }
        _ => None,
    }
}

#[cfg(not(target_os = "macos"))]
fn mic_state() -> MicPermission {
    MicPermission::NotApplicable
}

#[cfg(not(target_os = "macos"))]
fn mic_promptable() -> bool {
    false
}

#[cfg(not(target_os = "macos"))]
fn request_mic_impl() -> Result<MicRequestResult, String> {
    Ok(MicRequestResult {
        state: MicPermission::NotApplicable,
        timed_out: false,
    })
}

#[cfg(target_os = "macos")]
fn mic_state() -> MicPermission {
    macos::status()
}

#[cfg(target_os = "macos")]
fn mic_promptable() -> bool {
    mic_state() == MicPermission::NotDetermined
        && macos::has_usage_description("NSMicrophoneUsageDescription")
}

#[cfg(target_os = "macos")]
fn request_mic_impl() -> Result<MicRequestResult, String> {
    macos::request()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verdict(peak: f32, samples: u32, denied: bool, active: Option<u32>) -> SystemAudioVerdict {
        classify_system_audio(peak, samples, denied, active)
    }

    #[test]
    fn denied_evidence_wins_over_everything() {
        assert_eq!(
            verdict(0.5, 100, true, Some(3)),
            SystemAudioVerdict::TapRejected
        );
    }

    #[test]
    fn any_nonzero_peak_is_audible() {
        assert_eq!(
            verdict(1e-7, 100, false, Some(1)),
            SystemAudioVerdict::Audible
        );
        assert_eq!(verdict(1.0, 100, false, None), SystemAudioVerdict::Audible);
    }

    #[test]
    fn zero_peak_while_playing_is_the_denial_signature() {
        assert_eq!(
            verdict(0.0, 100, false, Some(2)),
            SystemAudioVerdict::SilentWhilePlaying
        );
    }

    #[test]
    fn zero_peak_with_nothing_playing_is_not_a_failure() {
        assert_eq!(
            verdict(0.0, 100, false, Some(0)),
            SystemAudioVerdict::NoAudioPlaying
        );
    }

    #[test]
    fn zero_samples_is_inconclusive() {
        assert_eq!(
            verdict(0.0, 0, false, Some(0)),
            SystemAudioVerdict::Inconclusive
        );
    }

    #[test]
    fn unknown_activity_is_inconclusive() {
        assert_eq!(
            verdict(0.0, 100, false, None),
            SystemAudioVerdict::Inconclusive
        );
    }

    #[test]
    fn settings_urls_are_stable() {
        assert!(settings_url("microphone")
            .unwrap()
            .contains("Privacy_Microphone"));
        assert!(settings_url("systemAudio")
            .unwrap()
            .contains("Privacy_ScreenCapture"));
        assert!(settings_url("speech")
            .unwrap()
            .contains("Privacy_SpeechRecognition"));
        assert!(settings_url("nope").is_none());
    }

    #[test]
    fn probe_window_covers_the_library_detector() {
        assert!(PROBE_WINDOW >= Duration::from_secs(5));
    }

    #[test]
    fn zero_frames_without_a_usage_description_blames_the_unbundled_build() {
        assert!(inconclusive_reason(0, false).contains("build:app"));
    }

    #[test]
    fn zero_frames_with_a_usage_description_points_at_the_prompt() {
        assert!(inconclusive_reason(0, true).contains("授权弹窗"));
    }

    #[test]
    fn frames_but_no_output_points_at_playback() {
        assert!(inconclusive_reason(88_000, true).contains("播放"));
        assert!(inconclusive_reason(88_000, false).contains("播放"));
    }
}
