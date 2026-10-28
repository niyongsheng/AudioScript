use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager};

use super::catalog::{self, CatalogEntry};
use super::disk;
use super::download;
use super::extract::{self, ExtractSpec};
use super::state::ModelState;
use super::status::{ModelJobStatus, ModelPhase};
use super::{ModelError, ModelKind, EVENT_MODEL_PROGRESS};
use crate::config::AsrModelTier;
use crate::i18n::{tf, tr, Key};
use crate::paths;

const STAGING: &str = ".staging";
const DOWNLOADS: &str = ".downloads";
const TRASH: &str = ".trash";
const SPARE: u64 = 64 * 1024 * 1024;

pub fn start_download(app: &AppHandle, model_id: &str) -> Result<String, ModelError> {
    let entry =
        catalog::entry(model_id).ok_or_else(|| ModelError::UnknownModel(model_id.to_owned()))?;
    if !entry.downloadable {
        return Err(ModelError::NotDownloadable(model_id.to_owned()));
    }
    let archive = entry
        .archive
        .as_ref()
        .ok_or_else(|| ModelError::Internal(tf(Key::ModelNoDownloadInfo, &[&entry.id])))?;
    if archive.sha256.len() != 64 || archive.size_bytes == 0 {
        return Err(ModelError::Internal(tf(
            Key::ModelNoChecksumInfo,
            &[&entry.id],
        )));
    }
    if is_installed_complete(app, entry) {
        return Err(ModelError::AlreadyInstalled(model_id.to_owned()));
    }

    let state = app.state::<ModelState>();
    let Some((job_id, cancel, status)) = state.start(model_id) else {
        return Err(ModelError::JobInProgress);
    };

    let app_for_thread = app.clone();
    let model_for_thread = model_id.to_owned();
    let job_for_thread = job_id.clone();
    let kind_for_thread = entry.kind.as_str().to_owned();
    let tier_for_thread = entry.tier.clone();

    let spawned = std::thread::Builder::new()
        .name("model-install".into())
        .spawn(move || {
            let started = std::time::Instant::now();
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                run_install(
                    &app_for_thread,
                    &model_for_thread,
                    &job_for_thread,
                    &cancel,
                    &status,
                )
            }));

            app_for_thread
                .state::<ModelState>()
                .clear(&job_for_thread);

            let result = match &outcome {
                Ok(Ok(())) => "ok",
                Ok(Err(ModelError::Cancelled)) => "cancelled",
                Ok(Err(_)) => "failed",
                Err(_) => "panicked",
            };

            match outcome {
                Ok(Ok(())) => set_phase(&app_for_thread, &status, ModelPhase::Done, None),
                Ok(Err(ModelError::Cancelled)) => {
                    cleanup_job(&app_for_thread, &job_for_thread);
                    set_phase(&app_for_thread, &status, ModelPhase::Cancelled, None)
                }
                Ok(Err(e)) => {
                    tracing::error!(model = %model_for_thread, "{e}");
                    cleanup_job(&app_for_thread, &job_for_thread);
                    set_phase(
                        &app_for_thread,
                        &status,
                        ModelPhase::Failed,
                        Some(e.to_string()),
                    )
                }
                Err(_) => {
                    cleanup_job(&app_for_thread, &job_for_thread);
                    set_phase(
                        &app_for_thread,
                        &status,
                        ModelPhase::Failed,
                        Some(tr(Key::ModelThreadPanicked).into()),
                    )
                }
            }

            crate::telemetry::track(
                &app_for_thread,
                "model_download",
                Some(serde_json::json!({
                    "model": model_for_thread,
                    "kind": kind_for_thread,
                    "tier": tier_for_thread,
                    "result": result,
                    "duration_bucket": crate::telemetry::duration_bucket(started.elapsed().as_millis() as u64),
                })),
            );
        });

    if let Err(e) = spawned {
        app.state::<ModelState>().clear(&job_id);
        return Err(ModelError::Internal(tf(
            Key::ModelThreadStartFailed,
            &[&e.to_string()],
        )));
    }

    tracing::info!(model = model_id, job = %job_id, "模型下载已开始");
    Ok(job_id)
}

fn cleanup_job(app: &AppHandle, job_id: &str) {
    let Some(root) = paths::user_models_root(app) else {
        return;
    };
    std::fs::remove_dir_all(root.join(STAGING).join(job_id)).ok();
    std::fs::remove_file(root.join(DOWNLOADS).join(format!("{job_id}.part"))).ok();
}

fn is_installed_complete(app: &AppHandle, entry: &CatalogEntry) -> bool {
    let roots = paths::search_roots(app);
    match entry.kind {
        ModelKind::Asr => AsrModelTier::parse(&entry.tier)
            .and_then(|tier| paths::locate_asr(&roots, tier).ok())
            .and_then(|resolved| paths::missing_asr_files(&resolved.path, entry).ok())
            .map(|missing| missing.is_empty())
            .unwrap_or(false),
        ModelKind::Mt => paths::locate_mt_root(&roots)
            .and_then(|resolved| paths::missing_mt_files(&resolved.path, entry))
            .map(|missing| missing.is_empty())
            .unwrap_or(false),
    }
}

fn missing_files(root: &Path, entry: &CatalogEntry) -> Result<Vec<String>, ModelError> {
    match entry.kind {
        ModelKind::Asr => paths::missing_asr_files(root, entry).map_err(ModelError::Internal),
        ModelKind::Mt => paths::missing_mt_files(root, entry).map_err(ModelError::Internal),
    }
}

fn run_install(
    app: &AppHandle,
    model_id: &str,
    job_id: &str,
    cancel: &AtomicBool,
    status: &Mutex<ModelJobStatus>,
) -> Result<(), ModelError> {
    let entry =
        catalog::entry(model_id).ok_or_else(|| ModelError::UnknownModel(model_id.to_owned()))?;
    let archive = entry
        .archive
        .as_ref()
        .ok_or_else(|| ModelError::Internal(tf(Key::ModelNoDownloadInfo, &[&entry.id])))?;

    let user_root = paths::ensure_user_models_root(app).map_err(ModelError::Internal)?;
    let downloads = user_root.join(DOWNLOADS);
    let staging_root = user_root.join(STAGING).join(job_id);
    let payload = staging_root.join("payload");
    std::fs::create_dir_all(&downloads)?;
    std::fs::create_dir_all(&payload)?;

    let need = archive
        .size_bytes
        .saturating_add(entry.sizes.installed_bytes)
        .saturating_add(SPARE);
    if let Some(available) = disk::available_space(&user_root) {
        if available < need {
            return Err(ModelError::DiskSpace { need, available });
        }
    }

    let part = downloads.join(format!("{job_id}.part"));
    let result = download::download_verified(
        &archive.url,
        &archive.sha256,
        archive.size_bytes,
        &part,
        cancel,
        |received, total| update_download(app, status, received, total),
    );

    if let Err(e) = result {
        discard_staging(&part, &staging_root);
        return Err(e);
    }

    set_phase(app, status, ModelPhase::Verifying, None);
    set_phase(app, status, ModelPhase::Extracting, None);

    if let Err(e) = (|| -> Result<(), ModelError> {
        let spec = ExtractSpec::from_entry(entry)?;
        extract::extract_archive(&part, &spec, &payload, cancel)?;
        Ok(())
    })() {
        discard_staging(&part, &staging_root);
        return Err(e);
    }

    set_phase(app, status, ModelPhase::Validating, None);
    let missing = missing_files(&payload, entry)?;
    if !missing.is_empty() {
        discard_staging(&part, &staging_root);
        return Err(ModelError::Incomplete(missing.join(", ")));
    }

    let final_dir = user_root.join(entry.installed_dir());
    if let Some(parent) = final_dir.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let trash_root = user_root.join(TRASH);
    std::fs::create_dir_all(&trash_root)?;
    let old = trash_root.join(format!("{job_id}-old"));
    let moved_old = if final_dir.exists() {
        std::fs::rename(&final_dir, &old).map_err(|e| {
            ModelError::Io(tf(
                Key::ModelReplaceFailed,
                &[&final_dir.display().to_string(), &e.to_string()],
            ))
        })?;
        true
    } else {
        false
    };

    if let Err(e) = std::fs::rename(&payload, &final_dir) {
        if moved_old {
            std::fs::rename(&old, &final_dir).ok();
        }
        discard_staging(&part, &staging_root);
        return Err(ModelError::Io(tf(
            Key::ModelInstallFailed,
            &[&final_dir.display().to_string(), &e.to_string()],
        )));
    }

    discard_staging(&part, &staging_root);
    if moved_old {
        std::fs::remove_dir_all(&old).ok();
    }

    tracing::info!(model = %entry.id, dir = %final_dir.display(), "模型安装完成");
    Ok(())
}

fn discard_staging(part: &std::path::Path, staging_root: &std::path::Path) {
    std::fs::remove_file(part).ok();
    std::fs::remove_dir_all(staging_root).ok();
}

fn update_download(
    app: &AppHandle,
    status: &Mutex<ModelJobStatus>,
    received: u64,
    total: Option<u64>,
) {
    let snapshot = {
        let mut guard = status.lock().unwrap_or_else(|e| e.into_inner());
        guard.set_downloaded(received, total);
        guard.clone()
    };
    let _ = app.emit(EVENT_MODEL_PROGRESS, snapshot);
}

fn set_phase(
    app: &AppHandle,
    status: &Mutex<ModelJobStatus>,
    phase: ModelPhase,
    error: Option<String>,
) {
    let snapshot = {
        let mut guard = status.lock().unwrap_or_else(|e| e.into_inner());
        guard.phase = phase;
        guard.error = error;
        guard.clone()
    };
    let _ = app.emit(EVENT_MODEL_PROGRESS, snapshot);
}

pub fn cancel(app: &AppHandle, job_id: &str) -> bool {
    app.state::<ModelState>().cancel(job_id)
}

pub fn remove_model(app: &AppHandle, model_id: &str) -> Result<(), ModelError> {
    let entry =
        catalog::entry(model_id).ok_or_else(|| ModelError::UnknownModel(model_id.to_owned()))?;
    if entry.bundled {
        return Err(ModelError::Bundled);
    }

    let prefs = crate::prefs::load(app);
    let selected = entry.kind == ModelKind::Asr
        && AsrModelTier::parse(&entry.tier) == Some(prefs.asr_model_tier);
    if selected {
        return Err(ModelError::Selected);
    }

    let state = app.state::<ModelState>();
    if state.is_busy() {
        return Err(ModelError::JobInProgress);
    }
    if app.state::<crate::session::SessionState>().is_running() {
        return Err(ModelError::InUse);
    }

    let root = paths::user_models_root(app)
        .ok_or_else(|| ModelError::Internal(tr(Key::ModelNoDataDir).into()))?;
    let dir = root.join(entry.installed_dir());
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
        tracing::info!(model = %entry.id, dir = %dir.display(), "模型已删除");
    }

    Ok(())
}

pub fn cleanup_stale(app: &AppHandle) {
    let Some(root) = paths::user_models_root(app) else {
        return;
    };
    for name in [DOWNLOADS, STAGING, TRASH] {
        let dir = root.join(name);
        if dir.exists() {
            match std::fs::remove_dir_all(&dir) {
                Ok(()) => tracing::debug!(path = %dir.display(), "已清理模型临时目录"),
                Err(e) => tracing::warn!(path = %dir.display(), "清理模型临时目录失败：{e}"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_files_routes_to_the_right_checker() {
        let asr = catalog::entry("asr-high-accuracy").unwrap();
        let dir = std::env::temp_dir().join(format!("models-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let missing = missing_files(&dir, asr).unwrap();
        assert!(
            missing.iter().any(|m| m.starts_with("encoder")),
            "{missing:?}"
        );

        let mt = catalog::entry("mt-base").unwrap();
        let missing = missing_files(&dir, mt).unwrap();
        assert!(
            missing.iter().any(|m| m.contains("model.bin")),
            "{missing:?}"
        );
    }
}
