use std::path::{Path, PathBuf};

use ct2rs::tokenizers::hf::Tokenizer as HfTokenizer;

use super::{script, TranslateCaps, TranslateError, Translator};
use crate::config::{MtModelTier, TranslateConfig, ZhScript};
use crate::i18n::{tf, tr, Key};
use crate::models::catalog;

const SOURCE_TOKENIZER: &str = "source-tokenizer.json";
const TARGET_TOKENIZER: &str = "target-tokenizer.json";

pub struct MarianTokenizer {
    source: HfTokenizer,
    target: HfTokenizer,
}

impl MarianTokenizer {
    fn load(dir: &Path) -> Result<Self, TranslateError> {
        let load = |f: &str| {
            HfTokenizer::from_file(dir.join(f))
                .map_err(|e| TranslateError::ModelLoad(format!("{}/{f}：{e}", dir.display())))
        };
        Ok(Self {
            source: load(SOURCE_TOKENIZER)?,
            target: load(TARGET_TOKENIZER)?,
        })
    }
}

impl ct2rs::Tokenizer for MarianTokenizer {
    fn encode(&self, input: &str) -> anyhow::Result<Vec<String>> {
        let mut tokens = ct2rs::Tokenizer::encode(&self.source, input)?;

        tokens.push("</s>".to_string());
        Ok(tokens)
    }

    fn decode(&self, tokens: Vec<String>) -> anyhow::Result<String> {
        ct2rs::Tokenizer::decode(&self.target, tokens)
    }
}

pub struct LocalNmt {
    engine: Option<ct2rs::Translator<MarianTokenizer>>,
    direction: Direction,
    script: ZhScript,
    tier: MtModelTier,
    model_root: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    EnZh,
    ZhEn,
}

impl Direction {
    fn dir_name(self, tier: MtModelTier) -> &'static str {
        let dirs = catalog::mt_entry(tier).direction_dirs();
        match self {
            Self::EnZh => dirs
                .first()
                .map(String::as_str)
                .expect("catalog 缺少 en→zh 方向目录"),
            Self::ZhEn => dirs
                .get(1)
                .map(String::as_str)
                .expect("catalog 缺少 zh→en 方向目录"),
        }
    }

    fn resolve(from: &str, to: &str) -> Result<Self, TranslateError> {
        let from = from.to_ascii_lowercase();
        let to = to.to_ascii_lowercase();

        if from == "auto" {
            return Err(TranslateError::UnsupportedPair {
                from: tr(Key::TranslateAutoUnsupported).into(),
                to: to.to_string(),
            });
        }
        let from_is_en = from.starts_with("en");
        let from_is_zh = from.starts_with("zh");
        let to_is_zh = to.starts_with("zh");
        let to_is_en = to.starts_with("en");

        match (from_is_en || from_is_zh, to_is_zh, to_is_en) {
            (true, true, _) if !from_is_zh => Ok(Self::EnZh),
            (true, _, true) if !from_is_en => Ok(Self::ZhEn),
            _ => Err(TranslateError::UnsupportedPair {
                from: from.to_string(),
                to: to.to_string(),
            }),
        }
    }
}

impl LocalNmt {
    pub fn new(model_root: PathBuf, tier: MtModelTier) -> Self {
        Self {
            engine: None,
            direction: Direction::EnZh,
            script: ZhScript::default(),
            tier,
            model_root,
        }
    }

    fn source_for_model(&self, text: &str) -> String {
        if self.direction == Direction::ZhEn {
            script::to_simplified(text)
        } else {
            text.to_string()
        }
    }

    fn output_for_script(&self, text: String) -> String {
        if self.direction == Direction::EnZh && self.script == ZhScript::Traditional {
            script::to_traditional(&text)
        } else {
            text
        }
    }

    fn model_dir(&self) -> PathBuf {
        self.model_root.join(self.direction.dir_name(self.tier))
    }

    fn check_model_dir(dir: &Path, tier: MtModelTier) -> Result<(), TranslateError> {
        let remedy = match tier {
            MtModelTier::Base => tr(Key::TranslateRemedyFetchMt),
            MtModelTier::Big => tr(Key::TranslateRemedyBig),
        };

        if !dir.is_dir() {
            return Err(TranslateError::MissingModelFile(tf(
                Key::TranslatePathMissing,
                &[&dir.display().to_string(), remedy],
            )));
        }

        for f in ["model.bin", SOURCE_TOKENIZER, TARGET_TOKENIZER] {
            if !dir.join(f).is_file() {
                return Err(TranslateError::MissingModelFile(tf(
                    Key::TranslateFileMissing,
                    &[&dir.display().to_string(), f, remedy],
                )));
            }
        }
        Ok(())
    }
}

impl Translator for LocalNmt {
    fn init(&mut self, cfg: &TranslateConfig) -> Result<(), TranslateError> {
        self.direction = Direction::resolve(&cfg.source_lang, &cfg.target_lang)?;
        self.script = cfg.zh_script;

        if let Some(dir) = &cfg.model_dir {
            self.model_root = dunce::simplified(Path::new(dir)).to_path_buf();
        }

        let dir = self.model_dir();
        Self::check_model_dir(&dir, self.tier)?;

        let config = ct2rs::Config {
            num_threads_per_replica: 1,
            ..Default::default()
        };

        let tok = MarianTokenizer::load(&dir)?;
        let engine = ct2rs::Translator::with_tokenizer(&dir, tok, &config)
            .map_err(|e| TranslateError::ModelLoad(format!("{}：{e}", dir.display())))?;

        self.engine = Some(engine);

        let warm = match self.direction {
            Direction::EnZh => "hello",
            Direction::ZhEn => "你好",
        };
        if let Err(e) = self.translate(warm) {
            tracing::warn!("翻译引擎预热返回异常（忽略）：{e}");
        }

        Ok(())
    }

    fn capabilities(&self) -> TranslateCaps {
        TranslateCaps::LOCAL_NMT
    }

    fn translate(&mut self, text: &str) -> Result<String, TranslateError> {
        let engine = self.engine.as_ref().ok_or(TranslateError::NotInitialized)?;

        let sources = vec![self.source_for_model(text)];
        let options = ct2rs::TranslationOptions {
            beam_size: 1,
            ..Default::default()
        };

        let results = engine
            .translate_batch(&sources, &options, None)
            .map_err(|e| TranslateError::Internal(e.to_string()))?;

        let (translated, _) = results
            .into_iter()
            .next()
            .ok_or_else(|| TranslateError::Internal(tr(Key::TranslateEmptyResult).into()))?;

        if let Some(reason) = crate::translate::degenerate_translation(&translated) {
            return Err(TranslateError::Degenerate { reason });
        }

        Ok(self.output_for_script(translated))
    }

    fn shutdown(&mut self) {
        self.engine = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_resolves_both_ways() {
        assert_eq!(Direction::resolve("en", "zh").unwrap(), Direction::EnZh);
        assert_eq!(Direction::resolve("zh", "en").unwrap(), Direction::ZhEn);
        assert!(matches!(
            Direction::resolve("auto", "zh"),
            Err(TranslateError::UnsupportedPair { .. })
        ));
        assert_eq!(
            Direction::resolve("en-US", "zh-CN").unwrap(),
            Direction::EnZh
        );
    }

    #[test]
    fn unsupported_pairs_are_rejected_not_silently_defaulted() {
        assert!(matches!(
            Direction::resolve("ja", "zh"),
            Err(TranslateError::UnsupportedPair { .. })
        ));
        assert!(matches!(
            Direction::resolve("en", "fr"),
            Err(TranslateError::UnsupportedPair { .. })
        ));
        assert!(Direction::resolve("zh", "zh").is_err());
    }

    #[test]
    fn missing_model_dir_gives_actionable_error() {
        let e =
            LocalNmt::check_model_dir(Path::new("/nonexistent/mt"), MtModelTier::Base).unwrap_err();
        let msg = e.to_string();
        assert!(msg.contains("fetch:mt"), "错误里要给出修复动作：{msg}");

        let e =
            LocalNmt::check_model_dir(Path::new("/nonexistent/mt"), MtModelTier::Big).unwrap_err();
        assert!(e.to_string().contains("模型"), "{e}");
    }

    #[test]
    fn tiers_map_to_their_direction_directories() {
        assert_eq!(
            Direction::EnZh.dir_name(MtModelTier::Base),
            "opus-mt-en-zh-ct2"
        );
        assert_eq!(
            Direction::ZhEn.dir_name(MtModelTier::Base),
            "opus-mt-zh-en-ct2"
        );
        assert_eq!(
            Direction::EnZh.dir_name(MtModelTier::Big),
            "opus-mt-tc-big-en-zh-ct2"
        );
        assert_eq!(
            Direction::ZhEn.dir_name(MtModelTier::Big),
            "opus-mt-tc-big-zh-en-ct2"
        );
    }

    #[test]
    fn traditional_script_wraps_the_model_input_and_output() {
        let mut t = LocalNmt::new(PathBuf::from("/tmp"), MtModelTier::Base);

        t.script = ZhScript::Simplified;
        t.direction = Direction::ZhEn;
        assert_eq!(t.source_for_model("機器學習"), "机器学习");
        t.direction = Direction::EnZh;
        assert_eq!(t.output_for_script("机器学习".into()), "机器学习");

        t.script = ZhScript::Traditional;
        t.direction = Direction::ZhEn;
        assert_eq!(t.source_for_model("機器學習"), "机器学习");
        t.direction = Direction::EnZh;
        assert_eq!(t.output_for_script("机器学习".into()), "機器學習");
    }

    #[test]
    fn translate_before_init_is_rejected() {
        let mut t = LocalNmt::new(PathBuf::from("/tmp"), MtModelTier::Base);
        assert!(matches!(
            t.translate("hello"),
            Err(TranslateError::NotInitialized)
        ));
    }
}
