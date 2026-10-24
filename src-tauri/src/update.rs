use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

const PROGRESS_EVENT: &str = "update://progress";
pub const DISABLED_REASON: &str =
    "此构建未配置本项目的更新公钥，自动更新已禁用。请从本项目 Releases 手动下载。";

pub fn public_key() -> Option<&'static str> {
    option_env!("TAURI_UPDATER_PUBLIC_KEY")
        .map(str::trim)
        .filter(|key| !key.is_empty())
}

static INSTALLING: AtomicBool = AtomicBool::new(false);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheck {
    current_version: String,
    enabled: bool,
    disabled_reason: Option<String>,
    available: bool,
    version: Option<String>,
    notes: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress {
    downloaded: u64,
    total: Option<u64>,
}

struct InstallGuard;

impl Drop for InstallGuard {
    fn drop(&mut self) {
        INSTALLING.store(false, Ordering::SeqCst);
    }
}

pub async fn check(app: &AppHandle) -> Result<UpdateCheck, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    if public_key().is_none() {
        return Ok(UpdateCheck {
            current_version,
            enabled: false,
            disabled_reason: Some(DISABLED_REASON.into()),
            available: false,
            version: None,
            notes: None,
        });
    }
    let updater = app
        .updater_builder()
        .endpoints(vec![crate::project::links()
            .updater_endpoint
            .parse()
            .map_err(|e| format!("更新地址无效：{e}"))?])
        .map_err(|e| format!("更新地址无效：{e}"))?
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("更新组件未就绪：{e}"))?;
    let update = updater
        .check()
        .await
        .map_err(|e| format!("检查更新失败：{e}"))?;

    match update {
        Some(update) => {
            tracing::info!(version = %update.version, "发现新版本");
            Ok(UpdateCheck {
                current_version,
                enabled: true,
                disabled_reason: None,
                available: true,
                version: Some(update.version),
                notes: update.body,
            })
        }
        None => Ok(UpdateCheck {
            current_version,
            enabled: true,
            disabled_reason: None,
            available: false,
            version: None,
            notes: None,
        }),
    }
}

pub async fn install(app: &AppHandle) -> Result<String, String> {
    if public_key().is_none() {
        return Err(DISABLED_REASON.into());
    }
    if INSTALLING.swap(true, Ordering::SeqCst) {
        return Err("正在更新，请稍候".into());
    }
    let _guard = InstallGuard;

    let updater = app
        .updater_builder()
        .endpoints(vec![crate::project::links()
            .updater_endpoint
            .parse()
            .map_err(|e| format!("更新地址无效：{e}"))?])
        .map_err(|e| format!("更新地址无效：{e}"))?
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| format!("更新组件未就绪：{e}"))?;
    let Some(update) = updater
        .check()
        .await
        .map_err(|e| format!("检查更新失败：{e}"))?
    else {
        return Err("已是最新版本，无需更新".into());
    };

    let version = update.version.clone();
    tracing::info!(version = %version, "开始下载更新");

    let handle = app.clone();
    let mut downloaded = 0u64;
    let mut last_emit = std::time::Instant::now();
    update
        .download_and_install(
            move |chunk, total| {
                downloaded += chunk as u64;
                let done = total.is_some_and(|t| downloaded >= t);
                if done || last_emit.elapsed().as_millis() >= 100 {
                    last_emit = std::time::Instant::now();
                    let _ = handle.emit(PROGRESS_EVENT, Progress { downloaded, total });
                }
            },
            || {},
        )
        .await
        .map_err(|e| format!("安装更新失败：{e}"))?;

    tracing::info!(version = %version, "更新已安装，重启后生效");
    Ok(version)
}
