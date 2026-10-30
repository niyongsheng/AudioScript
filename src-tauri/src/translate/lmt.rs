use std::path::{Path, PathBuf};

use ct2rs::tokenizers::hf::Tokenizer as HfTokenizer;

use super::{
    apply_zh_script, load_hf_tokenizer, mt_threads, warm_text, TranslateCaps, TranslateError,
    Translator,
};
use crate::config::{base_lang, MtModelTier, TranslateConfig, ZhScript};
use crate::i18n::{tr, Key};
use crate::models::catalog::{self, CatalogEntry};

const MAX_NEW_TOKENS: usize = 256;
const SPECIAL_TOKENS: [&str; 3] = ["<|im_end|>", "<|im_start|>", "<|endoftext|>"];

pub struct LmtTranslator {
    engine: Option<ct2rs::Generator<LmtTokenizer>>,
    source_name: &'static str,
    target_name: &'static str,
    zh_script: ZhScript,
    model_root: PathBuf,
    caps: TranslateCaps,
    entry: &'static CatalogEntry,
}

struct LmtTokenizer {
    inner: HfTokenizer,
}

impl LmtTokenizer {
    fn load(dir: &Path) -> Result<Self, TranslateError> {
        Ok(Self {
            inner: load_hf_tokenizer(dir)?,
        })
    }
}

impl ct2rs::Tokenizer for LmtTokenizer {
    fn encode(&self, input: &str) -> anyhow::Result<Vec<String>> {
        ct2rs::Tokenizer::encode(&self.inner, input)
    }

    fn decode(&self, tokens: Vec<String>) -> anyhow::Result<String> {
        ct2rs::Tokenizer::decode(&self.inner, tokens)
    }
}

fn lmt_language(lang: &str) -> Option<&'static str> {
    let base = base_lang(lang);
    Some(match base {
        "zh" => "Chinese",
        "en" => "English",
        "yue" => "Yue Chinese",
        "ar" => "Arabic",
        "de" => "German",
        "fr" => "French",
        "es" => "Spanish",
        "pt" => "Portuguese",
        "id" => "Indonesian",
        "it" => "Italian",
        "ko" => "Korean",
        "ru" => "Russian",
        "th" => "Thai",
        "vi" => "Vietnamese",
        "ja" => "Japanese",
        "tr" => "Turkish",
        "hi" => "Hindi",
        "ms" => "Malay",
        "nl" => "Dutch",
        "sv" => "Swedish",
        "da" => "Danish",
        "fi" => "Finnish",
        "pl" => "Polish",
        "cs" => "Czech",
        "fil" => "Tagalog",
        "fa" => "Persian",
        "el" => "Modern Greek",
        "hu" => "Hungarian",
        "ro" => "Romanian",
        _ => return None,
    })
}

fn build_prompt(source_name: &str, target_name: &str, text: &str) -> String {
    let text = if text.contains(['\r', '\n']) {
        std::borrow::Cow::Owned(text.replace(['\r', '\n'], " "))
    } else {
        std::borrow::Cow::Borrowed(text)
    };
    format!(
        "<|im_start|>user\nTranslate the following text from {source_name} into {target_name}:\n\
         {source_name}: {}\n{target_name}:<|im_end|>\n<|im_start|>assistant\n",
        text.trim()
    )
}

fn clean_output(text: &str) -> String {
    let mut out = std::borrow::Cow::Borrowed(text);
    for marker in SPECIAL_TOKENS {
        if out.contains(marker) {
            out = std::borrow::Cow::Owned(out.replace(marker, ""));
        }
    }
    out.trim().to_string()
}

impl LmtTranslator {
    pub fn new(model_root: PathBuf, tier: MtModelTier) -> Self {
        let caps = match tier {
            MtModelTier::LmtHigh => TranslateCaps::LOCAL_LMT_HIGH,
            _ => TranslateCaps::LOCAL_LMT_FAST,
        };
        Self {
            engine: None,
            source_name: "English",
            target_name: "Chinese",
            zh_script: ZhScript::Simplified,
            model_root,
            caps,
            entry: catalog::mt_entry(tier),
        }
    }

    fn output_for_script(&self, text: &str) -> String {
        apply_zh_script(
            text,
            (self.target_name == "Chinese").then_some(self.zh_script),
        )
    }

    fn check_model_dir(&self, dir: &Path) -> Result<(), TranslateError> {
        let files: Vec<&str> = self
            .entry
            .required
            .mt
            .as_ref()
            .map(|mt| mt.files.iter().map(String::as_str).collect())
            .unwrap_or_default();
        super::check_model_dir(dir, &files)
    }
}

impl Translator for LmtTranslator {
    fn init(&mut self, cfg: &TranslateConfig) -> Result<(), TranslateError> {
        let (source_name, target_name) = match (
            lmt_language(&cfg.source_lang),
            lmt_language(&cfg.target_lang),
        ) {
            (Some(source), Some(target)) if source != target => (source, target),
            _ => {
                return Err(TranslateError::UnsupportedPair {
                    from: cfg.source_lang.clone(),
                    to: cfg.target_lang.clone(),
                })
            }
        };

        if let Some(dir) = &cfg.model_dir {
            self.model_root = dunce::simplified(Path::new(dir)).to_path_buf();
        }

        let dir = self.model_root.clone();
        self.check_model_dir(&dir)?;

        let config = ct2rs::Config {
            compute_type: ct2rs::ComputeType::INT8_FLOAT32,
            num_threads_per_replica: mt_threads(),
            ..Default::default()
        };
        let tok = LmtTokenizer::load(&dir)?;
        let engine = ct2rs::Generator::with_tokenizer(&dir, tok, &config)
            .map_err(|e| TranslateError::ModelLoad(format!("{}：{e}", dir.display())))?;

        self.engine = Some(engine);
        self.source_name = source_name;
        self.target_name = target_name;
        self.zh_script = cfg.zh_script;

        if let Err(e) = self.translate(warm_text(&cfg.source_lang)) {
            tracing::warn!("LMT 预热返回异常（忽略）：{e}");
        }

        Ok(())
    }

    fn capabilities(&self) -> TranslateCaps {
        self.caps
    }

    fn translate(&mut self, text: &str) -> Result<String, TranslateError> {
        let engine = self.engine.as_ref().ok_or(TranslateError::NotInitialized)?;

        let prompt = build_prompt(self.source_name, self.target_name, text);
        let options = ct2rs::GenerationOptions {
            beam_size: 1,
            max_length: MAX_NEW_TOKENS,
            include_prompt_in_result: false,
            ..Default::default()
        };

        let results = engine
            .generate_batch(&[prompt], &options, None)
            .map_err(|e| TranslateError::Internal(e.to_string()))?;
        let sequence = results
            .into_iter()
            .next()
            .and_then(|(sequences, _)| sequences.into_iter().next())
            .ok_or_else(|| TranslateError::Internal(tr(Key::TranslateEmptyResult).into()))?;
        let translated = clean_output(&sequence);

        if let Some(reason) = crate::translate::degenerate_translation(&translated) {
            return Err(TranslateError::Degenerate { reason });
        }

        Ok(self.output_for_script(&translated))
    }

    fn shutdown(&mut self) {
        self.engine = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_catalog_language_maps_to_an_lmt_name() {
        for id in ["mt-lmt-fast", "mt-lmt-high"] {
            let entry = crate::models::catalog::entry(id).unwrap_or_else(|| panic!("缺少 {id}"));
            for lang in &entry.languages {
                assert!(
                    lmt_language(lang).is_some(),
                    "{id} 的 {lang} 缺少 LMT 语言名"
                );
            }
            assert!(!entry.languages.iter().any(|l| l == "mk"));
        }
    }

    #[test]
    fn lmt_rejects_languages_outside_its_training_set() {
        assert_eq!(lmt_language("mk"), None);
        assert_eq!(lmt_language("mk-MK"), None);
        assert_eq!(lmt_language("xx"), None);
        assert_eq!(lmt_language("auto"), None);
    }

    #[test]
    fn language_tags_use_the_base_code() {
        assert_eq!(lmt_language("zh-TW"), Some("Chinese"));
        assert_eq!(lmt_language("en-US"), Some("English"));
        assert_eq!(lmt_language("fil"), Some("Tagalog"));
        assert_eq!(lmt_language("yue"), Some("Yue Chinese"));
        assert_eq!(lmt_language("el"), Some("Modern Greek"));
    }

    #[test]
    fn prompt_uses_the_qwen_chat_wrapper() {
        let p = build_prompt("English", "Chinese", "hello\nworld");
        assert!(p.starts_with("<|im_start|>user\n"));
        assert!(p.contains("English: hello world"));
        assert!(!p.contains("hello\nworld"), "源码换行必须被压平：{p}");
        assert!(p.ends_with("<|im_start|>assistant\n"));
    }

    #[test]
    fn special_tokens_are_removed_from_output() {
        assert_eq!(clean_output(" 你好<|im_end|> "), "你好");
    }

    #[test]
    fn missing_model_dir_gives_an_actionable_error() {
        let translator =
            LmtTranslator::new(PathBuf::from("/nonexistent/lmt"), MtModelTier::LmtFast);
        let e = translator
            .check_model_dir(Path::new("/nonexistent/lmt"))
            .unwrap_err();
        assert!(
            e.to_string().contains("fetch:mt"),
            "错误里要给出修复动作：{e}"
        );
    }

    #[test]
    fn translate_before_init_is_rejected() {
        let mut t = LmtTranslator::new(PathBuf::from("/tmp"), MtModelTier::LmtFast);
        assert!(matches!(
            t.translate("hello"),
            Err(TranslateError::NotInitialized)
        ));
    }

    #[test]
    #[ignore = "需要本地 LMT 模型：AUDIOSCRIPT_LMT_DIR"]
    fn translates_with_a_local_model() {
        let dir = std::env::var("AUDIOSCRIPT_LMT_DIR")
            .expect("设置 AUDIOSCRIPT_LMT_DIR 指向转换后的模型目录");
        let mut t = LmtTranslator::new(PathBuf::from(dir), MtModelTier::LmtFast);
        let cfg = TranslateConfig {
            source_lang: "ja".into(),
            target_lang: "zh".into(),
            ..Default::default()
        };
        t.init(&cfg).unwrap();
        let out = t.translate("こんにちは、世界。").unwrap();
        assert!(!out.trim().is_empty(), "译文为空");
        assert!(out.contains("世界"), "译文不符合预期：{out}");
    }
}
