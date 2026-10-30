use serde::Serialize;
use serde_json::json;

use crate::i18n::{tf, Key};

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

    #[error("{message}")]
    Settings { pane: &'static str, message: String },

    #[error("{0}")]
    Failed(String),
}

impl From<String> for CmdError {
    fn from(message: String) -> Self {
        Self::Failed(message)
    }
}

impl From<&'static str> for CmdError {
    fn from(message: &'static str) -> Self {
        Self::Failed(message.to_owned())
    }
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
            Self::Settings { pane, message } => json!({
                "kind": "settings",
                "pane": pane,
                "message": message,
            })
            .serialize(s),
            other => s.serialize_str(&other.to_string()),
        }
    }
}
