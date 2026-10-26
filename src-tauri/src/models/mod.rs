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

    #[error("{0}")]
    Internal(String),
}

impl From<std::io::Error> for ModelError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}
