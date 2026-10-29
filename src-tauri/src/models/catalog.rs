use std::path::{Component, Path};
use std::sync::OnceLock;

use serde::Deserialize;

use super::ModelKind;
use crate::config::{AsrModelTier, MtModelTier};
use crate::i18n::{tr, Key};

#[derive(Debug, Deserialize)]
pub struct Catalog {
    pub version: u32,
    pub models: Vec<CatalogEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogEntry {
    pub id: String,
    pub kind: ModelKind,
    pub tier: String,
    pub title: String,
    pub description: String,
    pub bundled: bool,
    pub downloadable: bool,
    #[serde(default)]
    pub multilingual: bool,
    #[serde(default)]
    pub languages: Vec<String>,
    pub install: InstallSpec,
    pub required: RequiredSpec,
    #[serde(default)]
    pub archive: Option<ArchiveSpec>,
    #[serde(default)]
    pub prune_profile: Option<String>,
    #[serde(default)]
    pub sizes: SizeSpec,
    #[serde(default)]
    pub licenses: Vec<LicenseSpec>,
    #[serde(default)]
    pub source_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallSpec {
    pub dir: String,
    #[serde(default)]
    pub archive_root: Option<String>,
    #[serde(default)]
    pub archive_roots: Vec<String>,
    #[serde(default)]
    pub directions: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct RequiredSpec {
    #[serde(default)]
    pub asr: Option<AsrRequired>,
    #[serde(default)]
    pub mt: Option<MtRequired>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum AsrFormat {
    #[default]
    Transducer,
    Qwen3Asr,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AsrRequired {
    #[serde(default)]
    pub format: AsrFormat,
    #[serde(default)]
    pub encoder: Option<String>,
    #[serde(default)]
    pub decoder: Option<String>,
    #[serde(default)]
    pub joiner: Option<String>,
    #[serde(default)]
    pub conv_frontend: Option<String>,
    #[serde(default)]
    pub tokenizer_dir: Option<String>,
    #[serde(default)]
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum MtFormat {
    #[default]
    Marian,
    Nllb,
}

#[derive(Debug, Deserialize)]
pub struct MtRequired {
    #[serde(default)]
    pub format: MtFormat,
    #[serde(default)]
    pub files: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveSpec {
    pub url: String,
    pub format: ArchiveFormat,
    pub sha256: String,
    pub size_bytes: u64,
    pub max_extracted_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArchiveFormat {
    TarGz,
    TarBz2,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SizeSpec {
    #[serde(default)]
    pub installed_bytes: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseSpec {
    pub component: String,
    pub license: String,
    #[serde(default)]
    pub license_url: Option<String>,
    #[serde(default)]
    pub requires_attribution: bool,
    #[serde(default)]
    pub attribution: Option<String>,
}

static CATALOG: OnceLock<Catalog> = OnceLock::new();

pub fn catalog() -> &'static Catalog {
    CATALOG.get_or_init(|| {
        let parsed: Catalog = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/model-catalog.json"
        )))
        .expect("embedded model catalog is invalid JSON");
        if let Err(e) = validate(&parsed) {
            panic!("embedded model catalog failed validation: {e}");
        }
        parsed
    })
}

pub fn entry(id: &str) -> Option<&'static CatalogEntry> {
    catalog().models.iter().find(|m| m.id == id)
}

pub fn asr_entry(tier: AsrModelTier) -> &'static CatalogEntry {
    by_tier(ModelKind::Asr, tier.as_str())
}

pub fn mt_entry(tier: MtModelTier) -> &'static CatalogEntry {
    by_tier(ModelKind::Mt, tier.as_str())
}

fn by_tier(kind: ModelKind, tier: &str) -> &'static CatalogEntry {
    catalog()
        .models
        .iter()
        .find(|m| m.kind == kind && m.tier == tier)
        .expect("catalog must contain every model tier")
}

impl CatalogEntry {
    pub fn asr_format(&self) -> Option<AsrFormat> {
        self.required.asr.as_ref().map(|r| r.format)
    }

    pub fn mt_format(&self) -> Option<MtFormat> {
        self.required.mt.as_ref().map(|r| r.format)
    }

    pub fn streaming(&self) -> Option<bool> {
        self.asr_format().map(|f| f == AsrFormat::Transducer)
    }

    pub fn localized_title(&self) -> &str {
        let key = match self.id.as_str() {
            "asr-builtin" => Some(Key::ModelAsrBuiltinTitle),
            "asr-high-accuracy" => Some(Key::ModelAsrHighTitle),
            "asr-qwen3" => Some(Key::ModelAsrQwen3Title),
            "mt-base" => Some(Key::ModelMtBaseTitle),
            "mt-multilingual" => Some(Key::ModelMtMultilingualTitle),
            _ => None,
        };
        match key {
            Some(key) => tr(key),
            None => self.title.as_str(),
        }
    }

    pub fn installed_dir(&self) -> &str {
        &self.install.dir
    }

    pub fn direction_dirs(&self) -> &[String] {
        &self.install.directions
    }

    pub fn archive_roots(&self) -> Vec<&str> {
        let mut roots = Vec::new();
        if let Some(root) = self.install.archive_root.as_deref() {
            roots.push(root);
        }
        roots.extend(self.install.archive_roots.iter().map(String::as_str));
        roots
    }
}

fn is_safe_relative(path: &str) -> bool {
    if path.is_empty() || path.contains('\\') || path.contains(':') {
        return false;
    }
    let p = Path::new(path);
    let mut components = p.components();
    let Some(first) = components.next() else {
        return false;
    };
    if !matches!(first, Component::Normal(_)) {
        return false;
    }
    components.all(|c| matches!(c, Component::Normal(_)))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn is_language_code(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|c| c.is_ascii_lowercase())
}

pub fn validate(c: &Catalog) -> Result<(), String> {
    if c.version != 1 {
        return Err(format!("不支持的 catalog 版本 {}", c.version));
    }

    let mut ids = std::collections::HashSet::new();
    for m in &c.models {
        if !ids.insert(m.id.as_str()) {
            return Err(format!("模型 id 重复：{}", m.id));
        }
        if !is_safe_relative(&m.install.dir) {
            return Err(format!("模型 {} 的安装目录不安全：{}", m.id, m.install.dir));
        }
        if !m.bundled
            && m.install.directions.is_empty()
            && m.install.archive_roots.is_empty()
            && m.install.archive_root.is_none()
        {
            return Err(format!("模型 {} 既没有方向目录也没有归档根目录", m.id));
        }
        if let Some(asr) = &m.required.asr {
            match asr.format {
                AsrFormat::Transducer => {
                    if asr.encoder.is_none() || asr.decoder.is_none() || asr.joiner.is_none() {
                        return Err(format!("模型 {} 缺少 encoder/decoder/joiner", m.id));
                    }
                }
                AsrFormat::Qwen3Asr => {
                    if asr.conv_frontend.is_none()
                        || asr.encoder.is_none()
                        || asr.decoder.is_none()
                        || asr.tokenizer_dir.is_none()
                    {
                        return Err(format!(
                            "模型 {} 缺少 convFrontend/encoder/decoder/tokenizerDir",
                            m.id
                        ));
                    }
                }
            }
        }
        if let Some(mt) = &m.required.mt {
            if mt.files.is_empty() {
                return Err(format!("模型 {} 没有声明 MT 必需文件", m.id));
            }
            if mt.format == MtFormat::Nllb
                && !(mt.files.iter().any(|f| f == "model.bin")
                    && mt.files.iter().any(|f| f == "tokenizer.json"))
            {
                return Err(format!(
                    "模型 {} 的 NLLB 条目必须声明 model.bin 与 tokenizer.json",
                    m.id
                ));
            }
        }
        if m.multilingual && m.languages.is_empty() {
            return Err(format!("多语言模型 {} 必须声明 languages", m.id));
        }
        if let Some(code) = m.languages.iter().find(|code| !is_language_code(code)) {
            return Err(format!("模型 {} 的语言代码无效：{code}", m.id));
        }
        if let Some(archive) = &m.archive {
            if !archive.url.starts_with("https://") {
                return Err(format!("模型 {} 的下载地址必须是 HTTPS", m.id));
            }
            if m.downloadable {
                if !is_sha256(&archive.sha256) {
                    return Err(format!("模型 {} 的 SHA-256 未填写或格式错误", m.id));
                }
                if archive.size_bytes == 0 || archive.max_extracted_bytes == 0 {
                    return Err(format!("模型 {} 的归档体积未填写", m.id));
                }
            }
        } else if m.downloadable {
            return Err(format!("模型 {} 标记为可下载但没有归档信息", m.id));
        }
    }

    for (kind, tier) in [
        (ModelKind::Asr, "builtin"),
        (ModelKind::Asr, "high-accuracy"),
        (ModelKind::Asr, "qwen3"),
        (ModelKind::Mt, "base"),
        (ModelKind::Mt, "multilingual"),
    ] {
        let count = c
            .models
            .iter()
            .filter(|m| m.kind == kind && m.tier == tier)
            .count();
        if count != 1 {
            return Err(format!(
                "{}/{} 档必须恰好有一个模型（当前 {count}）",
                kind.as_str(),
                tier
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_catalog_is_valid() {
        validate(catalog()).unwrap();
    }

    #[test]
    fn every_tier_has_an_entry() {
        assert_eq!(asr_entry(AsrModelTier::Builtin).id, "asr-builtin");
        assert_eq!(
            asr_entry(AsrModelTier::HighAccuracy).id,
            "asr-high-accuracy"
        );
        assert_eq!(asr_entry(AsrModelTier::Qwen3).id, "asr-qwen3");
        assert_eq!(mt_entry(MtModelTier::Base).id, "mt-base");
        assert_eq!(mt_entry(MtModelTier::Multilingual).id, "mt-multilingual");
    }

    #[test]
    fn asr_entries_report_streaming() {
        assert_eq!(entry("asr-builtin").unwrap().streaming(), Some(true));
        assert_eq!(entry("asr-high-accuracy").unwrap().streaming(), Some(true));
        assert_eq!(entry("asr-qwen3").unwrap().streaming(), Some(false));
        assert_eq!(mt_entry(MtModelTier::Base).streaming(), None);
        assert_eq!(mt_entry(MtModelTier::Multilingual).streaming(), None);
    }

    #[test]
    fn bilingual_entries_declare_languages() {
        for id in ["asr-builtin", "asr-high-accuracy", "mt-base"] {
            let entry = entry(id).unwrap();
            assert_eq!(entry.languages, ["zh", "en"], "{id} 缺少中英语言声明");
        }
    }

    #[test]
    fn install_dirs_are_safe_relative_paths() {
        assert!(is_safe_relative("asr-high"));
        assert!(is_safe_relative("mt/big"));
        assert!(!is_safe_relative(""));
        assert!(!is_safe_relative("../x"));
        assert!(!is_safe_relative("/abs"));
        assert!(!is_safe_relative("C:\\x"));
        assert!(!is_safe_relative("a\\b"));
    }

    #[test]
    fn sha256_shape_is_enforced() {
        assert!(is_sha256(&"a".repeat(64)));
        assert!(!is_sha256("abc"));
        assert!(!is_sha256(&"z".repeat(64)));
    }
}
