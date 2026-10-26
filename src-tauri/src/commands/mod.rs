use serde::Serialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;

use crate::config::{
    direction_label, effective_target_lang, EndpointConfig, EndpointProfile, OverlayConfig,
    OverlayMode, Preferences, PrefsPatch, SessionConfig, TranslateEngine,
};
use crate::i18n::{tf, tr, Key};
use crate::{audio, monitor, overlay, permission, prefs, session, shortcut, update};

#[derive(Debug, thiserror::Error)]
pub enum CmdError {
    #[error("{}", tf(Key::CmdUnknown, &[.0.as_str()]))]
    Unknown(String),

    #[error("{}", tf(Key::CmdBadArgs, &[.0.as_str()]))]
    BadArgs(String),

    #[error("{message}")]
    Permission {
        permission: &'static str,
        message: String,
    },

    #[error("{0}")]
    Failed(String),
}

impl Serialize for CmdError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Permission {
                permission,
                message,
            } => json!({
                "kind": "permission",
                "permission": permission,
                "message": message,
            })
            .serialize(s),
            other => s.serialize_str(&other.to_string()),
        }
    }
}

#[tauri::command]
pub async fn dispatch(app: AppHandle, name: String, args: Value) -> Result<Value, CmdError> {
    tracing::debug!(cmd = %name, "命令到达");
    let t0 = std::time::Instant::now();

    let out = route_async(&app, &name, &args).await;

    if let Err(e) = &out {
        tracing::warn!(cmd = %name, error = %e, "命令失败");
    } else if name == "session_stop" {
        tracing::info!(ms = t0.elapsed().as_millis(), "停止命令已返回");
    }

    out
}

async fn run_blocking<F, R>(task: F, timeout: std::time::Duration) -> Result<R, CmdError>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let (tx, rx) = tokio::sync::oneshot::channel();
    std::thread::Builder::new()
        .name("cmd-blocking".into())
        .spawn(move || {
            let _ = tx.send(task());
        })
        .map_err(|e| CmdError::Failed(tf(Key::CmdThreadStartFailed, &[&e.to_string()])))?;

    await_task(rx, timeout).await
}

async fn await_task<R>(
    rx: tokio::sync::oneshot::Receiver<R>,
    timeout: std::time::Duration,
) -> Result<R, CmdError> {
    match tokio::time::timeout(timeout, rx).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(_)) => Err(CmdError::Failed(tr(Key::CmdTaskPanicked).into())),
        Err(_) => Err(CmdError::Failed(tr(Key::CmdTimeout).into())),
    }
}

type CmdTask = Box<dyn FnOnce() + Send>;

struct CmdExecutors {
    quick: std::sync::mpsc::Sender<CmdTask>,
    session: std::sync::mpsc::Sender<CmdTask>,
}

static CMD_EXECUTORS: std::sync::OnceLock<CmdExecutors> = std::sync::OnceLock::new();

fn spawn_cmd_lane(name: &str) -> Result<std::sync::mpsc::Sender<CmdTask>, String> {
    let (tx, rx) = std::sync::mpsc::channel::<CmdTask>();
    std::thread::Builder::new()
        .name(name.into())
        .spawn(move || {
            while let Ok(task) = rx.recv() {
                task();
            }
        })
        .map_err(|e| format!("启动 {name} 失败：{e}"))?;
    Ok(tx)
}

pub fn init_executors() -> Result<(), String> {
    let quick = spawn_cmd_lane("cmd-quick")?;
    let session = spawn_cmd_lane("cmd-session")?;
    let _ = CMD_EXECUTORS.set(CmdExecutors { quick, session });
    Ok(())
}

pub(crate) fn enqueue_session(task: impl FnOnce() + Send + 'static) {
    let Some(executors) = CMD_EXECUTORS.get() else {
        tracing::warn!("命令执行线程尚未就绪，忽略后台音频任务");
        return;
    };
    let task: CmdTask = Box::new(task);
    if executors.session.send(task).is_err() {
        tracing::warn!("命令执行线程已退出，忽略后台音频任务");
    }
}

#[derive(Clone, Copy)]
enum Lane {
    Quick,
    Session,
}

async fn run_lane<F, R>(lane: Lane, task: F, timeout: std::time::Duration) -> Result<R, CmdError>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let Some(executors) = CMD_EXECUTORS.get() else {
        return run_blocking(task, timeout).await;
    };

    let sender = match lane {
        Lane::Quick => &executors.quick,
        Lane::Session => &executors.session,
    };

    let (tx, rx) = tokio::sync::oneshot::channel();
    sender
        .send(Box::new(move || {
            let _ = tx.send(task());
        }))
        .map_err(|_| CmdError::Failed(tr(Key::CmdLaneExited).into()))?;

    await_task(rx, timeout).await
}

pub async fn route_async(app: &AppHandle, name: &str, args: &Value) -> Result<Value, CmdError> {
    match name {
        "update_check" => {
            return update::check(app)
                .await
                .map(|r| json!(r))
                .map_err(CmdError::Failed)
        }
        "update_install" => {
            return update::install(app)
                .await
                .map(|version| json!({ "version": version }))
                .map_err(CmdError::Failed)
        }
        _ => {}
    }

    let plan = match name {
        "session_start" => Some((Lane::Session, std::time::Duration::from_secs(60))),
        "monitor_start" => Some((Lane::Session, std::time::Duration::from_secs(90))),
        "monitor_stop" => Some((Lane::Session, std::time::Duration::from_secs(15))),
        "permission_probe_system_audio" => Some((Lane::Quick, std::time::Duration::from_secs(30))),
        "permission_request_mic" => Some((Lane::Quick, std::time::Duration::from_secs(40))),
        "models_list" => Some((Lane::Quick, std::time::Duration::from_secs(10))),
        "model_remove" => Some((Lane::Quick, std::time::Duration::from_secs(60))),
        _ => None,
    };

    if let Some((lane, timeout)) = plan {
        let (app, name, args) = (app.clone(), name.to_string(), args.clone());
        return run_lane(lane, move || route(&app, &name, &args), timeout).await?;
    }

    route(app, name, args)
}

pub fn route(app: &AppHandle, name: &str, args: &Value) -> Result<Value, CmdError> {
    match name {
        "app_info" => Ok(app_info()),
        "locale_get" => Ok(json!(crate::i18n::locale().as_str())),

        "update_restart" => {
            tracing::info!("正在重启以应用更新");
            app.restart();
        }
        "overlay_toggle" => overlay::toggle(app)
            .map(|v| json!(v))
            .map_err(CmdError::Failed),
        "overlay_set_mode" => {
            let raw = str_arg(args, "mode")?;
            let mode = OverlayMode::parse(raw)
                .ok_or_else(|| CmdError::BadArgs(tf(Key::CmdUnknownMode, &[raw])))?;
            overlay::set_mode(app, mode)
                .map(|m| json!(m.as_str()))
                .map_err(CmdError::Failed)
        }

        "overlay_debug" => overlay::debug_state(app).map_err(CmdError::Failed),

        "prefs_get" => Ok(json!(prefs::load(app))),

        "prefs_set" => {
            let patch: PrefsPatch = json_arg(args, "patch")?;

            patch.validate().map_err(CmdError::BadArgs)?;

            let updated = prefs::update(app, |p| patch.apply_to(p)).map_err(CmdError::Failed)?;
            crate::apply_language(app, updated.language);
            Ok(json!(updated))
        }

        "prefs_reset" => {
            let fresh = Preferences::default();
            let updated = prefs::update(app, |p| *p = fresh).map_err(CmdError::Failed)?;
            crate::apply_language(app, updated.language);
            Ok(json!(updated))
        }

        "permission_status" => Ok(json!(permission::snapshot())),

        "permission_request_mic" => permission::request_mic()
            .map(|r| json!(r))
            .map_err(CmdError::Failed),

        "permission_probe_system_audio" => {
            if app.state::<session::SessionState>().is_running() {
                return Err(CmdError::Failed(tr(Key::CmdProbeWhileCapturing).into()));
            }
            monitor::stop(app);
            Ok(json!(permission::probe_system_audio(app)))
        }

        "open_privacy_settings" => {
            let pane = args
                .get("pane")
                .and_then(Value::as_str)
                .unwrap_or("systemAudio");
            let url = permission::settings_url(pane)
                .ok_or_else(|| CmdError::BadArgs(tf(Key::CmdUnknownPane, &[pane])))?;
            #[cfg(target_os = "macos")]
            if let Err(e) = std::process::Command::new("open").arg(url).spawn() {
                tracing::warn!(pane, "无法直接打开系统设置页（URI scheme 可能已变更）：{e}");
            }
            Ok(json!(null))
        }

        "open_repo" => {
            open_with_system(std::ffi::OsStr::new(REPO_URL), tr(Key::OpenWhatBrowser))?;
            Ok(json!(null))
        }

        "open_sponsor" => {
            open_with_system(std::ffi::OsStr::new(SPONSOR_URL), tr(Key::OpenWhatBrowser))?;
            Ok(json!(null))
        }

        "open_url" => {
            let url = str_arg(args, "url")?;
            if !is_allowed_link(url) {
                return Err(CmdError::BadArgs(tf(Key::CmdLinkRejected, &[url])));
            }
            open_with_system(std::ffi::OsStr::new(url), tr(Key::OpenWhatLink))?;
            Ok(json!(null))
        }

        "reveal_path" => {
            let target = args
                .get("path")
                .and_then(Value::as_str)
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| app.state::<crate::log::LogDir>().0.clone());
            reveal_in_file_manager(&target)?;
            Ok(json!(null))
        }

        "logs_export" => {
            let dir = app.state::<crate::log::LogDir>().0.clone();
            let dest = export_logs(app, &dir)?;
            Ok(json!({ "path": dest.display().to_string() }))
        }

        "transcript_export" => {
            let stamp = args
                .get("stamp")
                .and_then(Value::as_str)
                .unwrap_or("export");
            let path = str_arg(args, "path")?;
            let path = export_transcript(app, stamp, std::path::Path::new(path))?;
            Ok(json!({ "path": path.display().to_string() }))
        }

        "autostart_get" => Ok(json!({
            "enabled": app.autolaunch().is_enabled().unwrap_or(false),
        })),

        "autostart_set" => {
            let on = args
                .get("enabled")
                .and_then(Value::as_bool)
                .ok_or_else(|| CmdError::BadArgs(tr(Key::CmdMissingEnabled).into()))?;

            let r = if on {
                app.autolaunch().enable()
            } else {
                app.autolaunch().disable()
            };
            r.map_err(|e| CmdError::Failed(tf(Key::CmdAutostartFailed, &[&e.to_string()])))?;

            Ok(json!({
                "enabled": app.autolaunch().is_enabled().unwrap_or(false),
            }))
        }

        "models_list" => Ok(json!(crate::models::status::inspect(app))),

        "model_open_dir" => {
            let dir = crate::paths::ensure_user_models_root(app).map_err(CmdError::Failed)?;
            open_with_system(dir.as_os_str(), tr(Key::OpenWhatModelDir))?;
            Ok(json!(null))
        }

        "model_download" => {
            let id = str_arg(args, "modelId")?;
            let job_id =
                crate::models::install::start_download(app, id).map_err(model_cmd_error)?;
            Ok(json!({ "jobId": job_id }))
        }

        "model_cancel" => {
            let job_id = str_arg(args, "jobId")?;
            Ok(json!({ "cancelled": crate::models::install::cancel(app, job_id) }))
        }

        "model_remove" => {
            let id = str_arg(args, "modelId")?;
            crate::models::install::remove_model(app, id).map_err(model_cmd_error)?;
            Ok(json!(crate::models::status::inspect(app)))
        }

        "model_select" => {
            let id = str_arg(args, "modelId")?;
            Ok(json!(select_model(app, id)?))
        }

        "config_defaults" => Ok(json!({
            "overlay": OverlayConfig::default(),
            "endpoint": EndpointConfig::default(),
            "endpointProfiles": {
                "interpretation": EndpointConfig::for_profile(EndpointProfile::Interpretation),
                "dictation": EndpointConfig::for_profile(EndpointProfile::Dictation),
            },
        })),

        "overlay_config_get" => Ok(json!(overlay::config(app))),

        "overlay_visible" => Ok(json!(overlay::is_visible(app))),

        "overlay_config_set" => {
            let cfg: OverlayConfig = json_arg(args, "config")?;

            let applied = overlay::set_config(app, cfg).map_err(CmdError::Failed)?;
            Ok(json!(applied))
        }

        "audio_devices" => {
            let source = args
                .get("source")
                .and_then(Value::as_str)
                .and_then(audio::AudioSourceKind::parse)
                .unwrap_or_default();
            audio::list_devices(source)
                .map(|d| json!(d))
                .map_err(|e| CmdError::Failed(e.to_string()))
        }

        "monitor_start" => {
            let targets = args
                .get("targets")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| {
                            let source = item
                                .get("source")
                                .and_then(Value::as_str)
                                .and_then(audio::AudioSourceKind::parse)?;
                            let device = item.get("device").and_then(Value::as_str);
                            Some(monitor::Target::new(source, device))
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            for target in &targets {
                if let Some(permission) = permission::certain_denial(target.source) {
                    return Err(CmdError::Permission {
                        permission,
                        message: tr(Key::CmdMicDenied).into(),
                    });
                }
            }
            monitor::start(app, &targets).map_err(|e| match e {
                monitor::MonitorError::Audio(audio::AudioError::Permission {
                    permission,
                    message,
                }) => CmdError::Permission {
                    permission,
                    message,
                },
                other => CmdError::Failed(other.to_string()),
            })?;
            Ok(json!(null))
        }

        "monitor_stop" => {
            monitor::stop(app);
            Ok(json!(null))
        }

        "session_start" => {
            let cfg: SessionConfig = args
                .get("config")
                .map(|v| serde_json::from_value(v.clone()))
                .transpose()
                .map_err(|e| CmdError::BadArgs(tf(Key::CmdConfigParseFailed, &[&e.to_string()])))?
                .unwrap_or_default();

            let device = args
                .get("device")
                .and_then(Value::as_str)
                .map(str::to_owned);

            let source = args
                .get("source")
                .and_then(Value::as_str)
                .and_then(audio::AudioSourceKind::parse)
                .unwrap_or_default();

            let mix_mic = args
                .get("mixMicDeviceId")
                .and_then(Value::as_str)
                .map(str::to_owned);
            let mix_system = args
                .get("mixSystemDeviceId")
                .and_then(Value::as_str)
                .map(str::to_owned);
            let devices = if source == audio::AudioSourceKind::Mix {
                audio::DeviceSelection::mix(mix_mic, mix_system)
            } else {
                audio::DeviceSelection::single(device)
            };

            let mut cfg = cfg;
            cfg.asr.endpoint = prefs::load(app).endpoint.sanitized();

            tracing::info!(
                source_lang = %cfg.translate.source_lang,
                target_lang = %effective_target_lang(
                    &cfg.translate.source_lang,
                    &cfg.translate.target_lang
                ),
                zh_script = cfg.translate.zh_script.as_str(),
                engine = cfg.translate.engine.as_str(),
                trailing_silence_ms = cfg.asr.endpoint.trailing_silence_ms,
                max_utterance_ms = cfg.asr.endpoint.max_utterance_ms,
                "会话配置已解析（缺省字段已用默认值补齐）"
            );

            if let Some(permission) = permission::certain_denial(source) {
                return Err(CmdError::Permission {
                    permission,
                    message: tr(Key::CmdMicDenied).into(),
                });
            }

            monitor::suspend(app);
            match session::start(app, cfg, source, devices.clone()) {
                Ok(()) => {
                    let saved = prefs::update(app, |p| {
                        p.apply_session(source.as_str(), devices.device.clone());
                        if source == audio::AudioSourceKind::Mix {
                            p.apply_mix_devices(
                                devices.mix_mic.clone(),
                                devices.mix_system.clone(),
                            );
                        }
                    });
                    if let Err(e) = saved {
                        tracing::error!("偏好保存失败，本次设置不会被记住：{e}");
                    }
                    Ok(json!(session::TaskStatus::Capturing))
                }
                Err(msg) => {
                    let handle = app.clone();
                    enqueue_session(move || monitor::resume(&handle));
                    match permission::certain_denial(source) {
                        Some(permission) => Err(CmdError::Permission {
                            permission,
                            message: msg,
                        }),
                        None => Err(CmdError::Failed(msg)),
                    }
                }
            }
        }
        "session_stop" => session::stop(app)
            .map(|()| json!(session::TaskStatus::Ready))
            .map_err(CmdError::Failed),

        "session_snapshot" => Ok(json!(app.state::<session::SessionState>().snapshot())),

        "session_status" => Ok(json!(app.state::<session::SessionState>().info())),

        "session_clear" => {
            session::clear(app);
            Ok(json!(null))
        }

        "hotkey_failures" => Ok(json!(app.state::<shortcut::HotkeyState>().failures())),

        "shortcuts_get" => Ok(json!(shortcut::list(app))),

        "shortcuts_set" => {
            let id = str_arg(args, "id")?;
            let accelerator = str_arg(args, "accelerator")?;

            shortcut::rebind(app, id, accelerator)
                .map(|list| json!(list))
                .map_err(CmdError::Failed)
        }

        other => Err(CmdError::Unknown(other.to_string())),
    }
}

fn str_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, CmdError> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| CmdError::BadArgs(tf(Key::CmdMissingArg, &[key])))
}

fn json_arg<T: serde::de::DeserializeOwned>(args: &Value, key: &str) -> Result<T, CmdError> {
    let value = args
        .get(key)
        .ok_or_else(|| CmdError::BadArgs(tf(Key::CmdMissingArg, &[key])))?;
    serde_json::from_value(value.clone())
        .map_err(|e| CmdError::BadArgs(tf(Key::CmdArgParseFailed, &[key, &e.to_string()])))
}

fn app_info() -> Value {
    json!({
        "name": "AudioScript",
        "version": env!("CARGO_PKG_VERSION"),
        "platform": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "builtAtMs": exe_mtime_ms(),
    })
}

const REPO_URL: &str = "https://github.com/niyongsheng/AudioScript";
const SITE_URL: &str = "https://audioscript.looplooptech.com/";

const SPONSOR_URL: &str = "https://ifdian.net/a/nico2026";

fn export_dir(app: &AppHandle) -> Result<std::path::PathBuf, CmdError> {
    app.path()
        .desktop_dir()
        .or_else(|_| app.path().home_dir())
        .map_err(|e| CmdError::Failed(tf(Key::CmdNoWritableDir, &[&e.to_string()])))
}

fn export_transcript(
    app: &AppHandle,
    stamp: &str,
    dest: &std::path::Path,
) -> Result<std::path::PathBuf, CmdError> {
    let segments = app.state::<session::SessionState>().snapshot();
    let p = prefs::load(app);
    let direction = match p.translate_engine {
        TranslateEngine::Off => None,
        _ => Some(direction_label(&p.source_lang, p.zh_script)),
    };

    let mut file = dest.to_path_buf();
    if file.extension().is_none() {
        file.set_extension("md");
    }

    let md = crate::transcript::to_markdown(&segments, stamp, direction);
    std::fs::write(&file, md).map_err(|e| {
        CmdError::Failed(tf(
            Key::CmdWriteFailed,
            &[&file.display().to_string(), &e.to_string()],
        ))
    })?;

    tracing::info!(path = %file.display(), "转写已导出");
    Ok(file)
}

fn is_allowed_link(url: &str) -> bool {
    const PREFIXES: [&str; 7] = [
        REPO_URL,
        SITE_URL,
        "https://github.com/k2-fsa/sherpa-onnx",
        "https://github.com/Helsinki-NLP/Opus-MT",
        "https://huggingface.co/Helsinki-NLP/",
        "https://www.apache.org/licenses/LICENSE-2.0",
        "https://creativecommons.org/licenses/by/4.0/",
    ];

    PREFIXES.iter().any(|prefix| {
        if prefix.ends_with('/') {
            url.starts_with(prefix)
        } else {
            url.strip_prefix(prefix)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
        }
    })
}

fn open_with_system(target: &std::ffi::OsStr, what: &str) -> Result<(), CmdError> {
    #[cfg(target_os = "macos")]
    let mut cmd = std::process::Command::new("open");
    #[cfg(target_os = "windows")]
    let mut cmd = std::process::Command::new("explorer");
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut cmd = std::process::Command::new("xdg-open");

    cmd.arg(target)
        .spawn()
        .map(|_| ())
        .map_err(|e| CmdError::Failed(tf(Key::CmdOpenFailed, &[what, &e.to_string()])))
}

fn reveal_in_file_manager(path: &std::path::Path) -> Result<(), CmdError> {
    if !path.exists() {
        return Err(CmdError::Failed(tf(
            Key::CmdPathMissing,
            &[&path.display().to_string()],
        )));
    }
    open_with_system(path.as_os_str(), tr(Key::OpenWhatFileManager))
}

fn export_logs(app: &AppHandle, src: &std::path::Path) -> Result<std::path::PathBuf, CmdError> {
    let stamp = newest_log_stamp(src).unwrap_or_else(|| "unknown".into());
    let dest = export_dir(app)?.join(tf(Key::CmdLogsFolderName, &[&stamp]));
    let files = copy_logs(src, &dest)?;

    tracing::info!(from = %src.display(), to = %dest.display(), files, "日志已导出");
    Ok(dest)
}

fn copy_logs(src: &std::path::Path, dest: &std::path::Path) -> Result<usize, CmdError> {
    std::fs::create_dir_all(dest).map_err(|e| {
        CmdError::Failed(tf(
            Key::CmdCreateDirFailed,
            &[&dest.display().to_string(), &e.to_string()],
        ))
    })?;

    let entries = std::fs::read_dir(src)
        .map_err(|e| CmdError::Failed(tf(Key::CmdReadLogDirFailed, &[&e.to_string()])))?;

    let mut copied = 0usize;
    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let Some(name) = p.file_name() else { continue };
        std::fs::copy(&p, dest.join(name)).map_err(|e| {
            CmdError::Failed(tf(
                Key::CmdCopyFailed,
                &[&p.display().to_string(), &e.to_string()],
            ))
        })?;
        copied += 1;
    }

    if copied == 0 {
        return Err(CmdError::Failed(tf(
            Key::CmdNoLogs,
            &[&src.display().to_string()],
        )));
    }
    Ok(copied)
}

fn newest_log_stamp(dir: &std::path::Path) -> Option<String> {
    let mut stamps: Vec<String> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter_map(|e| e.file_name().to_str().map(str::to_owned))
        .filter_map(|n| {
            n.strip_prefix(crate::log::LOG_FILE_PREFIX)
                .and_then(|rest| rest.strip_prefix('.'))
                .map(str::to_owned)
        })
        .collect();
    stamps.sort();
    stamps.pop()
}

fn model_cmd_error(e: crate::models::ModelError) -> CmdError {
    use crate::models::ModelError as M;
    match e {
        M::UnknownModel(_)
        | M::NotDownloadable(_)
        | M::AlreadyInstalled(_)
        | M::Bundled
        | M::Selected
        | M::InUse
        | M::JobInProgress => CmdError::BadArgs(e.to_string()),
        _ => CmdError::Failed(e.to_string()),
    }
}

fn select_model(app: &AppHandle, id: &str) -> Result<crate::models::status::ModelsList, CmdError> {
    use crate::config::{AsrModelTier, MtModelTier};

    let entry = crate::models::catalog::entry(id)
        .ok_or_else(|| CmdError::BadArgs(tf(Key::CmdUnknownModel, &[id])))?;

    let mut list = crate::models::status::inspect(app);
    let complete = list
        .asr
        .iter()
        .chain(list.mt.iter())
        .find(|m| m.id == id)
        .map(|m| m.complete)
        .ok_or_else(|| CmdError::BadArgs(tf(Key::CmdUnknownModel, &[id])))?;
    if !complete {
        return Err(CmdError::BadArgs(tf(
            Key::CmdModelNotReady,
            &[&entry.localized_title()],
        )));
    }

    match entry.kind {
        crate::models::ModelKind::Asr => {
            let tier = AsrModelTier::parse(&entry.tier)
                .ok_or_else(|| CmdError::BadArgs(tf(Key::CmdUnknownAsrTier, &[&entry.tier])))?;
            crate::prefs::update(app, |p| p.asr_model_tier = tier).map_err(CmdError::Failed)?;
            for m in &mut list.asr {
                m.selected = m.id == id;
            }
        }
        crate::models::ModelKind::Mt => {
            let tier = MtModelTier::parse(&entry.tier)
                .ok_or_else(|| CmdError::BadArgs(tf(Key::CmdUnknownMtTier, &[&entry.tier])))?;
            crate::prefs::update(app, |p| p.mt_model_tier = tier).map_err(CmdError::Failed)?;
            for m in &mut list.mt {
                m.selected = m.id == id;
            }
        }
    }

    tracing::info!(model = %entry.id, "已切换模型（下次开始转写生效）");
    Ok(list)
}

fn exe_mtime_ms() -> Option<u64> {
    let exe = std::env::current_exe().ok()?;
    let mtime = std::fs::metadata(exe).ok()?.modified().ok()?;
    mtime
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("audioscript-logs-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn copy_logs_copies_every_file() {
        let src = tmp("src");
        std::fs::write(src.join("audioscript.log.2026-10-15"), b"a").unwrap();
        std::fs::write(src.join("audioscript.log.2026-10-14"), b"b").unwrap();
        let dest = src.join("out");

        assert_eq!(copy_logs(&src, &dest).unwrap(), 2);
        assert!(dest.join("audioscript.log.2026-10-15").is_file());
        assert!(dest.join("audioscript.log.2026-10-14").is_file());

        std::fs::create_dir_all(src.join("sub")).unwrap();
        let dest2 = src.join("out2");
        assert_eq!(copy_logs(&src, &dest2).unwrap(), 2, "子目录不该被算进去");

        std::fs::remove_dir_all(&src).ok();
    }

    #[test]
    fn copy_logs_fails_on_empty_source_instead_of_succeeding_silently() {
        let src = tmp("empty");
        let e = copy_logs(&src, &src.join("out")).unwrap_err();
        assert!(e.to_string().contains("没有日志文件"), "{e}");
        std::fs::remove_dir_all(&src).ok();
    }

    #[test]
    fn newest_stamp_picks_the_latest_date() {
        let src = tmp("stamp");
        for d in ["2026-10-13", "2026-10-15", "2026-10-14"] {
            std::fs::write(src.join(format!("audioscript.log.{d}")), b"x").unwrap();
        }
        std::fs::write(src.join("notes.txt"), b"x").unwrap();

        assert_eq!(newest_log_stamp(&src).as_deref(), Some("2026-10-15"));
        std::fs::remove_dir_all(&src).ok();
    }

    #[test]
    fn newest_stamp_is_none_when_there_are_no_logs() {
        let src = tmp("nostamp");
        assert_eq!(newest_log_stamp(&src), None);
        std::fs::remove_dir_all(&src).ok();
    }

    #[test]
    fn only_known_links_can_be_opened() {
        assert!(is_allowed_link(
            "https://github.com/niyongsheng/AudioScript/releases/tag/v0.1.5"
        ));
        assert!(is_allowed_link(
            "https://github.com/niyongsheng/AudioScript"
        ));
        assert!(is_allowed_link(SITE_URL));
        assert!(!is_allowed_link(
            "https://niyongsheng.github.io.evil/AudioScript/"
        ));
        assert!(is_allowed_link(
            "https://github.com/k2-fsa/sherpa-onnx/releases/tag/asr-models"
        ));
        assert!(is_allowed_link(
            "https://huggingface.co/Helsinki-NLP/opus-mt-tc-big-en-zh"
        ));
        assert!(is_allowed_link(
            "https://creativecommons.org/licenses/by/4.0/"
        ));
        assert!(!is_allowed_link("https://github.com/niyongsheng/other"));
        assert!(!is_allowed_link(
            "https://github.com/niyongsheng/AudioScript-evil"
        ));
        assert!(!is_allowed_link(
            "https://github.com/k2-fsa/sherpa-onnx-evil"
        ));
        assert!(!is_allowed_link("https://huggingface.co/other/model"));
        assert!(!is_allowed_link(
            "http://github.com/niyongsheng/AudioScript"
        ));
        assert!(!is_allowed_link("javascript:alert(1)"));
    }
}
