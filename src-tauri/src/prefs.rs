use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tauri::{AppHandle, Manager};

use crate::config::Preferences;
use crate::i18n::{tf, Key};

const FILE_NAME: &str = "preferences.json";

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| tf(Key::PrefsNoConfigDir, &[&e.to_string()]))?;
    Ok(dir.join(FILE_NAME))
}

static WRITE_LOCK: Mutex<()> = Mutex::new(());

pub fn update<F>(app: &AppHandle, f: F) -> Result<Preferences, String>
where
    F: FnOnce(&mut Preferences),
{
    let _guard = WRITE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let mut p = load(app);
    f(&mut p);
    let p = p.sanitized();
    save(app, &p)?;
    Ok(p)
}

pub fn parse(raw: &str) -> Result<Preferences, serde_json::Error> {
    Ok(serde_json::from_str::<Preferences>(raw)?.sanitized())
}

pub fn to_json(prefs: &Preferences) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(prefs)
}

pub fn load(app: &AppHandle) -> Preferences {
    match config_path(app) {
        Ok(path) => load_from(&path),
        Err(e) => {
            tracing::error!("{e}，使用默认偏好");
            Preferences::default()
        }
    }
}

pub fn load_from(path: &Path) -> Preferences {
    let raw = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::info!(path = %path.display(), "尚无偏好文件，使用默认值");
            return Preferences::default();
        }
        Err(e) => {
            tracing::error!(path = %path.display(), "偏好文件读取失败：{e}，使用默认值");
            return Preferences::default();
        }
    };

    match parse(&raw) {
        Ok(p) => {
            tracing::debug!(
                path = %path.display(),
                audio_source = %p.audio_source,
                engine = %p.translate_engine.as_str(),
                source_lang = %p.source_lang,
                "偏好已载入"
            );
            p
        }
        Err(e) => {
            tracing::error!(
                path = %path.display(),
                "偏好文件解析失败：{e}，使用默认值（原文件保留未动）"
            );
            Preferences::default()
        }
    }
}

pub fn save(app: &AppHandle, prefs: &Preferences) -> Result<(), String> {
    save_to(&config_path(app)?, prefs)
}

pub fn save_to(path: &Path, prefs: &Preferences) -> Result<(), String> {
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| tf(Key::PrefsNoParent, &[&path.display().to_string()]))?;

    std::fs::create_dir_all(dir).map_err(|e| tf(Key::PrefsCreateDirFailed, &[&e.to_string()]))?;

    let json = to_json(prefs).map_err(|e| tf(Key::PrefsSerializeFailed, &[&e.to_string()]))?;

    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, &json).map_err(|e| tf(Key::PrefsWriteTempFailed, &[&e.to_string()]))?;
    std::fs::rename(&tmp, path).map_err(|e| tf(Key::PrefsReplaceFailed, &[&e.to_string()]))?;

    tracing::debug!(path = %path.display(), "偏好已保存");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TranslateEngine;

    #[test]
    fn missing_file_is_not_an_error_path() {
        assert!(parse("").is_err(), "空串必须被当作坏文件而非静默成功");
    }

    #[test]
    fn empty_object_yields_all_defaults() {
        let p = parse("{}").unwrap();
        assert_eq!(p, Preferences::default());
    }

    #[test]
    fn partial_config_keeps_known_fields() {
        let p = parse(r#"{"sourceLang":"zh"}"#).unwrap();
        assert_eq!(p.source_lang, "zh");
        assert_eq!(p.audio_source, "loopback", "未提供的字段应回落默认");
    }

    #[test]
    fn unknown_enum_value_fails_that_file_not_silently() {
        assert!(parse(r#"{"translateEngine":"gpt-5"}"#).is_err());
    }

    #[test]
    fn garbage_is_rejected_not_partially_applied() {
        assert!(parse("not json at all").is_err());
        assert!(parse(r#"{"audioSource":123}"#).is_err());
    }

    #[test]
    fn roundtrip_is_stable() {
        let p = Preferences {
            audio_source: "mix".into(),
            device_id: Some("dev-42".into()),
            translate_engine: TranslateEngine::Off,
            source_lang: "zh".into(),
            target_lang: "en".into(),
            ..Default::default()
        };
        let json = to_json(&p).unwrap();
        assert_eq!(parse(&json).unwrap(), p);
    }

    #[test]
    fn source_lang_derivation_survives_roundtrip() {
        let p = parse(r#"{"sourceLang":"zh"}"#).unwrap();
        assert_eq!(p.target_lang, "en");
    }

    fn temp_path(tag: &str) -> PathBuf {
        std::env::temp_dir()
            .join(format!("audioscript-prefs-{tag}-{}", uuid::Uuid::new_v4()))
            .join(FILE_NAME)
    }

    #[test]
    fn save_then_load_roundtrips_through_disk() {
        let path = temp_path("roundtrip");
        let p = Preferences {
            audio_source: "mix".into(),
            device_id: Some("device-7".into()),
            mix_mic_device_id: Some("mic-3".into()),
            mix_system_device_id: Some("out-4".into()),
            translate_engine: TranslateEngine::Off,
            source_lang: "zh".into(),
            target_lang: "en".into(),
            overlay: crate::config::OverlayConfig {
                opacity: 33,
                font_size: 40,
                max_lines: 3,
                ..Default::default()
            },
            overlay_geometry: Some(crate::config::WindowGeometry {
                position: crate::config::WindowPos { x: -1200, y: 980 },
                size: crate::config::WindowSize {
                    width: 800,
                    height: 240,
                },
            }),
            theme: crate::config::Theme::Dark,
            language: crate::config::Language::Ja,
            sidebar_visible: false,
            endpoint: crate::config::EndpointConfig {
                trailing_silence_ms: 600,
                max_utterance_ms: 12_000,
            },
            shortcuts: [(
                "overlay.toggleVisible".to_owned(),
                "CmdOrControl+Alt+H".to_owned(),
            )]
            .into_iter()
            .collect(),
            onboarding_done: true,
            asr_model_tier: crate::config::AsrModelTier::HighAccuracy,
            zh_script: crate::config::ZhScript::Traditional,
            analytics_enabled: false,
        };

        save_to(&path, &p).expect("写入应成功（含创建父目录）");
        assert_eq!(load_from(&path), p, "落盘再读回必须完全一致");

        std::fs::remove_dir_all(path.parent().unwrap()).ok();
    }

    #[test]
    fn save_creates_missing_parent_directory() {
        let path = temp_path("mkdir");
        assert!(!path.parent().unwrap().exists(), "前提：目录事先不存在");

        save_to(&path, &Preferences::default()).expect("应自动创建父目录");

        assert!(path.is_file(), "父目录创建后文件应在");
        std::fs::remove_dir_all(path.parent().unwrap()).ok();
    }

    #[test]
    fn save_leaves_no_temp_file_behind() {
        let path = temp_path("tmp");
        save_to(&path, &Preferences::default()).unwrap();

        let leftover = path.with_extension("json.tmp");
        assert!(!leftover.exists(), "临时文件残留：{}", leftover.display());

        std::fs::remove_dir_all(path.parent().unwrap()).ok();
    }

    #[test]
    fn overwriting_is_atomic_from_the_readers_view() {
        let path = temp_path("overwrite");
        let first = Preferences::default();
        let second = Preferences {
            audio_source: "mic".into(),
            device_id: None,
            translate_engine: TranslateEngine::Mock,
            source_lang: "zh".into(),
            target_lang: "en".into(),
            ..Default::default()
        };

        save_to(&path, &first).unwrap();
        save_to(&path, &second).unwrap();

        assert_eq!(load_from(&path), second, "第二次写入必须完全覆盖第一次");

        std::fs::remove_dir_all(path.parent().unwrap()).ok();
    }

    #[test]
    fn missing_file_loads_defaults_without_panicking() {
        let path = temp_path("absent");
        assert_eq!(load_from(&path), Preferences::default());
    }

    #[test]
    fn files_written_before_onboarding_existed_still_load() {
        let raw = r#"{"audioSource":"mix","sourceLang":"zh","theme":"dark"}"#;
        let p = parse(raw).expect("缺少新字段的旧文件必须能载入");
        assert_eq!(p.audio_source, "mix");
        assert_eq!(p.theme, crate::config::Theme::Dark);
        assert!(!p.onboarding_done, "默认必须是「还没看过引导」");
    }

    #[test]
    fn onboarding_done_round_trips_through_patch() {
        let patch: crate::config::PrefsPatch =
            serde_json::from_str(r#"{"onboardingDone":true}"#).unwrap();
        let mut p = Preferences::default();
        patch.apply_to(&mut p);
        assert!(p.onboarding_done);
    }

    #[test]
    fn corrupt_file_loads_defaults_and_leaves_original_intact() {
        let path = temp_path("corrupt");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"{ this is not json").unwrap();

        assert_eq!(load_from(&path), Preferences::default());
        assert_eq!(
            std::fs::read(&path).unwrap(),
            b"{ this is not json",
            "解析失败后原文件必须保持原样"
        );

        std::fs::remove_dir_all(path.parent().unwrap()).ok();
    }
}
