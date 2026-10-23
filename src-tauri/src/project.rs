use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectLinks {
    pub repository: String,
    pub site: String,
    pub upstream_repository: String,
    pub upstream_sponsor: String,
    pub updater_endpoint: String,
    pub models_release: String,
}

pub fn links() -> &'static ProjectLinks {
    static LINKS: OnceLock<ProjectLinks> = OnceLock::new();
    LINKS.get_or_init(|| {
        serde_json::from_str(include_str!("../../project.json"))
            .expect("项目链接配置必须是有效 JSON")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_and_model_releases_belong_to_the_fork() {
        let links = links();
        assert_eq!(
            links.updater_endpoint,
            format!("{}/releases/latest/download/latest.json", links.repository)
        );
        assert_eq!(
            links.models_release,
            format!("{}/releases/download/models-v1", links.repository)
        );
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(
            config["plugins"]["updater"]["endpoints"][0],
            links.updater_endpoint
        );
        assert_eq!(config["plugins"]["updater"]["pubkey"], "");
    }
}
