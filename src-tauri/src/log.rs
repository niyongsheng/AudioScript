use std::path::PathBuf;

use tauri::{AppHandle, Manager};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub const LOG_FILE_PREFIX: &str = "audioscript.log";

#[derive(Debug)]
pub struct LogGuard(#[allow(dead_code)] tracing_appender::non_blocking::WorkerGuard);

#[derive(Debug, Clone)]
pub struct LogDir(pub PathBuf);

pub fn init(app: &AppHandle) -> PathBuf {
    let fallback = std::env::temp_dir().join("audioscript-logs");

    let dir = app
        .path()
        .app_log_dir()
        .inspect_err(|e| eprintln!("无法解析日志目录，回退到 {}：{e}", fallback.display()))
        .unwrap_or_else(|_| fallback.clone());

    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("无法创建日志目录 {}：{e}", dir.display());
        return dir;
    }

    let file = tracing_appender::rolling::daily(&dir, LOG_FILE_PREFIX);
    let (writer, guard) = tracing_appender::non_blocking(file);
    app.manage(LogGuard(guard));

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,app_lib=debug"));

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(writer)
        .with_ansi(false)
        .with_target(true);

    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_target(false);

    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(stderr_layer)
        .try_init();

    tracing::info!(dir = %dir.display(), "日志已启动");
    dir
}
