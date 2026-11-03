pub mod catalog;
pub mod disk;
pub mod download;
pub mod extract;
pub mod install;
pub mod state;
pub mod status;

use serde::{Deserialize, Serialize};

use crate::i18n::{tf, tr, Key};

pub const EVENT_MODEL_PROGRESS: &str = "models://progress";

pub(crate) const IO_CHUNK: usize = 64 * 1024;

#[typeshare::typeshare]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelKind {
    Asr,
    Mt,
}

impl ModelKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Asr => "asr",
            Self::Mt => "mt",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("{}", tf(Key::ModelUnknown, &[.0.as_str()]))]
    UnknownModel(String),

    #[error("{}", tf(Key::ModelNotDownloadable, &[.0.as_str()]))]
    NotDownloadable(String),

    #[error("{}", tf(Key::ModelAlreadyInstalled, &[.0.as_str()]))]
    AlreadyInstalled(String),

    #[error("{}", tr(Key::ModelBundled))]
    Bundled,

    #[error("{}", tr(Key::ModelSelected))]
    Selected,

    #[error("{}", tr(Key::ModelInUse))]
    InUse,

    #[error("{}", tr(Key::ModelJobInProgress))]
    JobInProgress,

    #[error("{}", tf(Key::ModelDownloadFailed, &[.0.as_str()]))]
    Network(String),

    #[error("{}", tf(Key::ModelHttpFailed, &[&.0.to_string()]))]
    Http(u16),

    #[error("{}", tf(Key::ModelChecksumFailed, &[.expected.as_str(), .actual.as_str()]))]
    Checksum { expected: String, actual: String },

    #[error("{}", tf(Key::ModelUnsafeArchive, &[.0.as_str()]))]
    UnsafeArchive(String),

    #[error("{}", tf(Key::ModelExtractFailed, &[.0.as_str()]))]
    Extract(String),

    #[error("{}", tf(Key::ModelDiskFull, &[&(.need / 1048576).to_string(), &(.available / 1048576).to_string()]))]
    DiskSpace { need: u64, available: u64 },

    #[error("{}", tf(Key::ModelIncomplete, &[.0.as_str()]))]
    Incomplete(String),

    #[error("{}", tf(Key::ModelFileOpFailed, &[.0.as_str()]))]
    Io(String),

    #[error("{}", tr(Key::ModelCancelled))]
    Cancelled,

    #[error("{}", tf(Key::ModelThreadStartFailed, &[.0.as_str()]))]
    ThreadStartFailed(String),

    #[error("{}", tf(Key::ModelNoDownloadInfo, &[.0.as_str()]))]
    MissingDownloadInfo(String),

    #[error("{}", tf(Key::ModelNoChecksumInfo, &[.0.as_str()]))]
    MissingChecksumInfo(String),

    #[error("{0}")]
    Internal(String),
}

impl ModelError {
    pub fn telemetry_code(&self) -> &'static str {
        match self {
            Self::UnknownModel(_) => "unknown_model",
            Self::NotDownloadable(_) => "not_downloadable",
            Self::AlreadyInstalled(_) => "already_installed",
            Self::Bundled => "bundled",
            Self::Selected => "selected",
            Self::InUse => "in_use",
            Self::JobInProgress => "job_in_progress",
            Self::Network(_) => "network",
            Self::Http(_) => "http",
            Self::Checksum { .. } => "checksum",
            Self::UnsafeArchive(_) => "unsafe_archive",
            Self::Extract(_) => "extract",
            Self::DiskSpace { .. } => "disk_space",
            Self::Incomplete(_) => "incomplete",
            Self::Io(_) => "io",
            Self::Cancelled => "cancelled",
            Self::ThreadStartFailed(_) => "thread_start_failed",
            Self::MissingDownloadInfo(_) => "no_download_info",
            Self::MissingChecksumInfo(_) => "no_checksum_info",
            Self::Internal(_) => "internal",
        }
    }

    pub fn http_status(&self) -> u16 {
        match self {
            Self::Http(code) => *code,
            _ => 0,
        }
    }
}

impl From<std::io::Error> for ModelError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telemetry_codes_are_stable() {
        assert_eq!(
            ModelError::UnknownModel("m".into()).telemetry_code(),
            "unknown_model"
        );
        assert_eq!(
            ModelError::NotDownloadable("m".into()).telemetry_code(),
            "not_downloadable"
        );
        assert_eq!(ModelError::Http(404).telemetry_code(), "http");
        assert_eq!(ModelError::Http(404).http_status(), 404);
        assert_eq!(
            ModelError::Checksum {
                expected: "a".into(),
                actual: "b".into()
            }
            .telemetry_code(),
            "checksum"
        );
        assert_eq!(
            ModelError::DiskSpace {
                need: 1,
                available: 0
            }
            .telemetry_code(),
            "disk_space"
        );
        assert_eq!(ModelError::Cancelled.telemetry_code(), "cancelled");
        assert_eq!(
            ModelError::ThreadStartFailed("x".into()).telemetry_code(),
            "thread_start_failed"
        );
        assert_eq!(
            ModelError::MissingDownloadInfo("x".into()).telemetry_code(),
            "no_download_info"
        );
        assert_eq!(
            ModelError::MissingChecksumInfo("x".into()).telemetry_code(),
            "no_checksum_info"
        );
        assert_eq!(ModelError::Network("x".into()).http_status(), 0);
    }
}
