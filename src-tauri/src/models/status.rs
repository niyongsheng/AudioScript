use serde::Serialize;
use tauri::{AppHandle, Manager};

use super::catalog::{self, CatalogEntry};
use super::state::ModelState;
use super::ModelKind;
use crate::config::AsrModelTier;

#[typeshare::typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelLicenseDto {
    pub component: String,
    pub license: String,
    pub license_url: Option<String>,
    pub requires_attribution: bool,
    pub attribution: Option<String>,
}

#[typeshare::typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelStatus {
    pub id: String,
    pub kind: ModelKind,
    pub tier: String,
    pub title: String,
    pub description: String,
    pub bundled: bool,
    pub downloadable: bool,
    pub installed: bool,
    pub complete: bool,
    pub selected: bool,
    pub multilingual: bool,
    pub path: Option<String>,
    pub via: Option<String>,
    pub installed_bytes: u32,
    pub download_bytes: u32,
    pub licenses: Vec<ModelLicenseDto>,
    pub source_url: Option<String>,
    pub missing: Vec<String>,
}

#[typeshare::typeshare]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelPhase {
    Downloading,
    Verifying,
    Extracting,
    Validating,
    Done,
    Cancelled,
    Failed,
}

#[typeshare::typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelJobStatus {
    pub job_id: String,
    pub model_id: String,
    pub phase: ModelPhase,
    pub downloaded_bytes: u32,
    pub total_bytes: Option<u32>,
    pub percent: Option<f64>,
    pub error: Option<String>,
}

impl ModelJobStatus {
    pub fn new(job_id: String, model_id: String) -> Self {
        Self {
            job_id,
            model_id,
            phase: ModelPhase::Downloading,
            downloaded_bytes: 0,
            total_bytes: None,
            percent: None,
            error: None,
        }
    }

    pub fn set_downloaded(&mut self, received: u64, total: Option<u64>) {
        let max = u32::MAX as u64;
        self.downloaded_bytes = received.min(max) as u32;
        self.total_bytes = total.map(|v| v.min(max) as u32);
        self.percent = total
            .filter(|t| *t > 0)
            .map(|t| (received as f64 / t as f64 * 100.0).clamp(0.0, 100.0));
    }
}

#[typeshare::typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelsList {
    pub asr: Vec<ModelStatus>,
    pub mt: Vec<ModelStatus>,
    pub job: Option<ModelJobStatus>,
    pub user_root: Option<String>,
}

pub fn inspect(app: &AppHandle) -> ModelsList {
    let prefs = crate::prefs::load(app);
    let roots = crate::paths::search_roots(app);
    let mut asr = Vec::new();
    let mut mt = Vec::new();

    for entry in &catalog::catalog().models {
        let status = match entry.kind {
            ModelKind::Asr => {
                let selected = AsrModelTier::parse(&entry.tier) == Some(prefs.asr_model_tier);
                inspect_asr(&roots, entry, selected)
            }
            ModelKind::Mt => inspect_mt(&roots, entry, true),
        };
        match entry.kind {
            ModelKind::Asr => asr.push(status),
            ModelKind::Mt => mt.push(status),
        }
    }

    let job = app
        .try_state::<ModelState>()
        .and_then(|state| state.snapshot());

    ModelsList {
        asr,
        mt,
        job,
        user_root: roots.user.map(|p| p.display().to_string()),
    }
}

fn license_rows(entry: &CatalogEntry) -> Vec<ModelLicenseDto> {
    entry
        .licenses
        .iter()
        .map(|l| ModelLicenseDto {
            component: l.component.clone(),
            license: l.license.clone(),
            license_url: l.license_url.clone(),
            requires_attribution: l.requires_attribution,
            attribution: l.attribution.clone(),
        })
        .collect()
}

fn base_status(entry: &CatalogEntry, selected: bool) -> ModelStatus {
    ModelStatus {
        id: entry.id.clone(),
        kind: entry.kind,
        tier: entry.tier.clone(),
        title: entry.title.clone(),
        description: entry.description.clone(),
        bundled: entry.bundled,
        downloadable: entry.downloadable,
        installed: false,
        complete: false,
        selected,
        multilingual: entry.multilingual,
        path: None,
        via: None,
        installed_bytes: entry.sizes.installed_bytes.min(u32::MAX as u64) as u32,
        download_bytes: entry
            .archive
            .as_ref()
            .map(|a| a.size_bytes.min(u32::MAX as u64) as u32)
            .unwrap_or(0),
        licenses: license_rows(entry),
        source_url: entry.source_url.clone(),
        missing: Vec::new(),
    }
}

fn inspect_asr(
    roots: &crate::paths::ModelSearchRoots,
    entry: &CatalogEntry,
    selected: bool,
) -> ModelStatus {
    let mut out = base_status(entry, selected);
    let Some(tier) = AsrModelTier::parse(&entry.tier) else {
        return out;
    };

    if let Ok(resolved) = crate::paths::locate_asr(roots, tier) {
        out.installed = true;
        out.path = Some(resolved.path.display().to_string());
        out.via = Some(resolved.via.as_str().to_string());
        match crate::paths::missing_asr_files(&resolved.path, entry) {
            Ok(missing) => {
                out.complete = missing.is_empty();
                out.missing = missing;
            }
            Err(e) => out.missing = vec![e],
        }
    } else {
        out.path = expected_path(roots, entry);
    }

    out
}

fn inspect_mt(
    roots: &crate::paths::ModelSearchRoots,
    entry: &CatalogEntry,
    selected: bool,
) -> ModelStatus {
    let mut out = base_status(entry, selected);

    if let Ok(resolved) = crate::paths::locate_mt_root(roots) {
        out.installed = true;
        out.path = Some(resolved.path.display().to_string());
        out.via = Some(resolved.via.as_str().to_string());
        match crate::paths::missing_mt_files(&resolved.path, entry) {
            Ok(missing) => {
                out.complete = missing.is_empty();
                out.missing = missing;
            }
            Err(e) => out.missing = vec![e],
        }
    } else {
        out.path = expected_path(roots, entry);
    }

    out
}

fn expected_path(roots: &crate::paths::ModelSearchRoots, entry: &CatalogEntry) -> Option<String> {
    let root = if entry.bundled {
        roots.bundled.as_ref().or(roots.dev.as_ref())
    } else {
        roots.user.as_ref()
    }?;
    Some(root.join(entry.installed_dir()).display().to_string())
}
