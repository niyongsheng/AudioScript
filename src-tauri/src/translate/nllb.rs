use std::path::{Path, PathBuf};

use ct2rs::tokenizers::hf::Tokenizer as HfTokenizer;

use super::{
    apply_zh_script, load_hf_tokenizer, mt_threads, warm_text, TranslateCaps, TranslateError,
    Translator, TOKENIZER,
};
use crate::config::{base_lang, TranslateConfig, ZhScript};
use crate::i18n::{tr, Key};

const EOS: &str = "</s>";
const REQUIRED_FILES: [&str; 4] = [
    "model.bin",
    "shared_vocabulary.json",
    "config.json",
    TOKENIZER,
];
const MAX_DECODING_LENGTH: usize = 512;

pub struct NllbTranslator {
    engine: Option<ct2rs::Translator<NllbTokenizer>>,
    target_code: &'static str,
    zh_script: ZhScript,
    model_root: PathBuf,
}

struct NllbTokenizer {
    inner: HfTokenizer,
    source_code: &'static str,
}

impl NllbTokenizer {
    fn load(dir: &Path, source_code: &'static str) -> Result<Self, TranslateError> {
        Ok(Self {
            inner: load_hf_tokenizer(dir)?,
            source_code,
        })
    }
}

impl ct2rs::Tokenizer for NllbTokenizer {
    fn encode(&self, input: &str) -> anyhow::Result<Vec<String>> {
        let mut tokens = vec![self.source_code.to_string()];
        tokens.extend(ct2rs::Tokenizer::encode(&self.inner, input)?);
        tokens.push(EOS.to_string());
        Ok(tokens)
    }

    fn decode(&self, tokens: Vec<String>) -> anyhow::Result<String> {
        ct2rs::Tokenizer::decode(&self.inner, tokens)
    }
}

fn nllb_code(lang: &str, zh_script: ZhScript) -> Option<&'static str> {
    let base = base_lang(lang);
    Some(match base {
        "zh" => match zh_script {
            ZhScript::Simplified => "zho_Hans",
            ZhScript::Traditional => "zho_Hant",
        },
        "en" => "eng_Latn",
        "yue" => "yue_Hant",
        "ar" => "arb_Arab",
        "de" => "deu_Latn",
        "fr" => "fra_Latn",
        "es" => "spa_Latn",
        "pt" => "por_Latn",
        "id" => "ind_Latn",
        "it" => "ita_Latn",
        "ko" => "kor_Hang",
        "ru" => "rus_Cyrl",
        "th" => "tha_Thai",
        "vi" => "vie_Latn",
        "ja" => "jpn_Jpan",
        "tr" => "tur_Latn",
        "hi" => "hin_Deva",
        "ms" => "zsm_Latn",
        "nl" => "nld_Latn",
        "sv" => "swe_Latn",
        "da" => "dan_Latn",
        "fi" => "fin_Latn",
        "pl" => "pol_Latn",
        "cs" => "ces_Latn",
        "fil" => "tgl_Latn",
        "fa" => "pes_Arab",
        "el" => "ell_Grek",
        "hu" => "hun_Latn",
        "mk" => "mkd_Cyrl",
        "ro" => "ron_Latn",
        _ => return None,
    })
}

impl NllbTranslator {
    pub fn new(model_root: PathBuf) -> Self {
        Self {
            engine: None,
            target_code: "zho_Hans",
            zh_script: ZhScript::Simplified,
            model_root,
        }
    }

    fn output_for_script(&self, text: &str) -> String {
        apply_zh_script(
            text,
            self.target_code
                .starts_with("zho_")
                .then_some(self.zh_script),
        )
    }

    fn check_model_dir(dir: &Path) -> Result<(), TranslateError> {
        super::check_model_dir(dir, &REQUIRED_FILES)
    }
}

impl Translator for NllbTranslator {
    fn init(&mut self, cfg: &TranslateConfig) -> Result<(), TranslateError> {
        let source_code = nllb_code(&cfg.source_lang, cfg.zh_script).ok_or_else(|| {
            TranslateError::UnsupportedPair {
                from: cfg.source_lang.clone(),
                to: cfg.target_lang.clone(),
            }
        })?;
        let target_code = nllb_code(&cfg.target_lang, cfg.zh_script).ok_or_else(|| {
            TranslateError::UnsupportedPair {
                from: cfg.source_lang.clone(),
                to: cfg.target_lang.clone(),
            }
        })?;
        if source_code == target_code {
            return Err(TranslateError::UnsupportedPair {
                from: cfg.source_lang.clone(),
                to: cfg.target_lang.clone(),
            });
        }

        if let Some(dir) = &cfg.model_dir {
            self.model_root = dunce::simplified(Path::new(dir)).to_path_buf();
        }

        let dir = self.model_root.clone();
        Self::check_model_dir(&dir)?;

        let config = ct2rs::Config {
            num_threads_per_replica: mt_threads(),
            ..Default::default()
        };

        let tok = NllbTokenizer::load(&dir, source_code)?;
        let engine = ct2rs::Translator::with_tokenizer(&dir, tok, &config)
            .map_err(|e| TranslateError::ModelLoad(format!("{}：{e}", dir.display())))?;

        self.engine = Some(engine);
        self.target_code = target_code;
        self.zh_script = cfg.zh_script;

        if let Err(e) = self.translate(warm_text(&cfg.source_lang)) {
            tracing::warn!("NLLB 预热返回异常（忽略）：{e}");
        }

        Ok(())
    }

    fn capabilities(&self) -> TranslateCaps {
        TranslateCaps::LOCAL_MULTILINGUAL
    }

    fn translate(&mut self, text: &str) -> Result<String, TranslateError> {
        let engine = self.engine.as_ref().ok_or(TranslateError::NotInitialized)?;

        let sources = vec![text.to_string()];
        let target_prefix = vec![vec![self.target_code.to_string()]];
        let options = ct2rs::TranslationOptions {
            beam_size: 1,
            max_decoding_length: MAX_DECODING_LENGTH,
            ..Default::default()
        };

        let results = engine
            .translate_batch_with_target_prefix(&sources, &target_prefix, &options, None)
            .map_err(|e| TranslateError::Internal(e.to_string()))?;

        let (translated, _) = results
            .into_iter()
            .next()
            .ok_or_else(|| TranslateError::Internal(tr(Key::TranslateEmptyResult).into()))?;

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
    fn every_catalog_language_maps_to_an_nllb_code() {
        let entry =
            crate::models::catalog::entry("mt-multilingual").expect("catalog 缺少 mt-multilingual");
        for lang in &entry.languages {
            assert!(
                nllb_code(lang, ZhScript::Simplified).is_some(),
                "{lang} 缺少 NLLB 语言码"
            );
        }
        assert_eq!(nllb_code("xx", ZhScript::Simplified), None);
        assert_eq!(nllb_code("auto", ZhScript::Simplified), None);
    }

    #[test]
    fn qwen3_and_nllb_declare_the_same_languages() {
        let asr = crate::models::catalog::entry("asr-qwen3").expect("catalog 缺少 asr-qwen3");
        let mt =
            crate::models::catalog::entry("mt-multilingual").expect("catalog 缺少 mt-multilingual");
        let mut asr_langs = asr.languages.clone();
        let mut mt_langs = mt.languages.clone();
        asr_langs.sort();
        mt_langs.sort();
        assert_eq!(asr_langs, mt_langs, "Qwen3 与 NLLB 的语言清单必须一致");
    }

    #[test]
    fn chinese_script_selects_the_matching_nllb_code() {
        assert_eq!(nllb_code("zh", ZhScript::Simplified), Some("zho_Hans"));
        assert_eq!(nllb_code("zh-TW", ZhScript::Traditional), Some("zho_Hant"));
        assert_eq!(nllb_code("en-US", ZhScript::Traditional), Some("eng_Latn"));
    }

    #[test]
    fn missing_model_dir_gives_an_actionable_error() {
        let e = NllbTranslator::check_model_dir(Path::new("/nonexistent/nllb")).unwrap_err();
        assert!(
            e.to_string().contains("fetch:mt"),
            "错误里要给出修复动作：{e}"
        );
    }

    #[test]
    fn translate_before_init_is_rejected() {
        let mut t = NllbTranslator::new(PathBuf::from("/tmp"));
        assert!(matches!(
            t.translate("hello"),
            Err(TranslateError::NotInitialized)
        ));
    }

    #[test]
    #[ignore = "需要本地 NLLB 模型：AUDIOSCRIPT_NLLB_DIR"]
    fn translates_with_a_local_model() {
        let dir = std::env::var("AUDIOSCRIPT_NLLB_DIR")
            .expect("设置 AUDIOSCRIPT_NLLB_DIR 指向转换后的模型目录");
        let mut t = NllbTranslator::new(PathBuf::from(dir));
        let cfg = TranslateConfig {
            source_lang: "ja".into(),
            target_lang: "zh".into(),
            ..Default::default()
        };
        t.init(&cfg).unwrap();
        let out = t.translate("こんにちは、世界。").unwrap();
        assert!(!out.trim().is_empty(), "译文为空");
    }
}
