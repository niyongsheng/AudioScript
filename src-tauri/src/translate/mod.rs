pub mod lmt;
pub mod local_nmt;
#[cfg(target_os = "macos")]
pub mod mac_translation;
pub mod mock;
pub mod nllb;
pub mod queue;
pub mod script;

use std::path::Path;

use ct2rs::tokenizers::hf::Tokenizer as HfTokenizer;

use crate::config::{base_lang, TranslateConfig, ZhScript};
use crate::i18n::{tf, tr, Key};

#[derive(Debug, thiserror::Error)]
pub enum TranslateError {
    #[error("{}", tr(Key::TranslateNotInitialized))]
    NotInitialized,

    #[error("{}", tf(Key::TranslateModelLoadFailed, &[.0.as_str()]))]
    ModelLoad(String),

    #[error("{}", tf(Key::TranslateModelDirMissing, &[.0.as_str()]))]
    MissingModelFile(String),

    #[error("{}", tf(Key::TranslateUnsupportedPair, &[.from.as_str(), .to.as_str()]))]
    UnsupportedPair { from: String, to: String },

    #[error("{}", tf(Key::TranslatePlatformUnavailable, &[.0.as_str()]))]
    PlatformUnavailable(String),

    #[error("{}", tf(Key::TranslateInvalidOutput, &[.reason.as_str()]))]
    Degenerate { reason: String },

    #[error("{}", tf(Key::TranslateInternal, &[.0.as_str()]))]
    Internal(String),
}

const UNK_MARKERS: [&str; 2] = ["<unk>", "⁇"];

const REPEAT_MIN_LEN: usize = 8;

const REPEAT_RATIO: f32 = 0.6;

const REPEAT_FLOOR: usize = 4;

const TRAILING_RUN: usize = 4;

pub(crate) const TOKENIZER: &str = "tokenizer.json";

pub(crate) fn load_hf_tokenizer(dir: &Path) -> Result<HfTokenizer, TranslateError> {
    let mut tokenizer = HfTokenizer::from_file(dir.join(TOKENIZER))
        .map_err(|e| TranslateError::ModelLoad(format!("{}/{}：{e}", dir.display(), TOKENIZER)))?;
    tokenizer.disable_spacial_token();
    Ok(tokenizer)
}

pub(crate) fn warm_text(source_lang: &str) -> &'static str {
    match base_lang(source_lang) {
        "zh" => "你好",
        "ja" => "こんにちは",
        "ko" => "안녕하세요",
        _ => "hello",
    }
}

pub(crate) fn apply_zh_script(text: &str, zh_script: Option<ZhScript>) -> String {
    match zh_script {
        Some(ZhScript::Simplified) => script::to_simplified(text),
        Some(ZhScript::Traditional) => script::to_traditional(text),
        None => text.to_string(),
    }
}

pub fn check_model_dir(dir: &Path, files: &[&str]) -> Result<(), TranslateError> {
    let remedy = tr(Key::TranslateRemedyFetchMt);

    if !dir.is_dir() {
        return Err(TranslateError::MissingModelFile(tf(
            Key::TranslatePathMissing,
            &[&dir.display().to_string(), remedy],
        )));
    }

    for f in files {
        if !dir.join(f).is_file() {
            return Err(TranslateError::MissingModelFile(tf(
                Key::TranslateFileMissing,
                &[&dir.display().to_string(), f, remedy],
            )));
        }
    }
    Ok(())
}

pub fn degenerate_translation(text: &str) -> Option<String> {
    let t = text.trim();
    if t.is_empty() {
        return Some("空译文".into());
    }
    if let Some(m) = UNK_MARKERS.iter().find(|m| t.contains(**m)) {
        return Some(format!("含 {m}"));
    }

    let chars: Vec<char> = t.chars().filter(|c| !c.is_whitespace()).collect();
    if chars.len() < REPEAT_FLOOR {
        return None;
    }

    let mut counts: std::collections::HashMap<char, usize> = std::collections::HashMap::new();
    for c in &chars {
        *counts.entry(*c).or_default() += 1;
    }
    let (top, n) = counts
        .iter()
        .map(|(c, n)| (*c, *n))
        .max_by_key(|(_, n)| *n)?;
    let ratio = n as f32 / chars.len() as f32;

    if n == chars.len() {
        return Some(format!("整串重复同一字符「{top}」×{n}"));
    }

    if chars.len() >= REPEAT_MIN_LEN && ratio >= REPEAT_RATIO {
        return Some(format!("重复输出：「{top}」占 {:.0}%", ratio * 100.0));
    }

    if let Some(last) = chars.last() {
        let trailing = chars.iter().rev().take_while(|c| *c == last).count();
        if trailing >= TRAILING_RUN {
            return Some(format!("尾部重复「{last}」×{trailing}"));
        }
    }

    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TranslateCaps {
    pub incremental: bool,
    pub offline: bool,
    pub typical_latency_ms: u32,
}

impl TranslateCaps {
    pub const LOCAL_NMT: Self = Self {
        incremental: false,
        offline: true,
        typical_latency_ms: 300,
    };

    pub const MAC_TRANSLATION: Self = Self {
        incremental: false,
        offline: true,
        typical_latency_ms: 200,
    };

    pub const LOCAL_MULTILINGUAL: Self = Self {
        incremental: false,
        offline: true,
        typical_latency_ms: 500,
    };

    pub const LOCAL_LMT_FAST: Self = Self::LOCAL_MULTILINGUAL;

    pub const LOCAL_LMT_HIGH: Self = Self {
        incremental: false,
        offline: true,
        typical_latency_ms: 1500,
    };
}

pub(crate) fn mt_threads() -> usize {
    std::thread::available_parallelism()
        .map(|n| (n.get() / 2).clamp(2, 4))
        .unwrap_or(2)
}

pub trait Translator: Send {
    fn init(&mut self, cfg: &TranslateConfig) -> Result<(), TranslateError>;

    fn capabilities(&self) -> TranslateCaps;

    fn translate(&mut self, text: &str) -> Result<String, TranslateError>;

    fn shutdown(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mt_threads_stays_within_bounds() {
        let n = mt_threads();
        assert!((2..=4).contains(&n), "MT 线程数越界：{n}");
    }

    #[test]
    fn catches_unk_from_wrong_direction() {
        assert!(degenerate_translation("<unk>").is_some());
        assert!(degenerate_translation("他说 <unk> 和 <unk> 了").is_some());
    }

    #[test]
    fn catches_repetition_loops() {
        for s in [
            "来来来来来来",
            "的 的 的 的 的 的 的 的",
            "所以这里的关键洞见 关键是 我们需要谨慎地处理 长期间预算预算 。。。。。。。",
        ] {
            assert!(degenerate_translation(s).is_some(), "应判为退化：{s}");
        }
    }

    #[test]
    fn empty_output_is_degenerate() {
        assert!(degenerate_translation("").is_some());
        assert!(degenerate_translation("   ").is_some());
    }

    #[test]
    fn normal_translations_are_not_flagged() {
        for s in [
            "所以这里的关键洞察是我们需要处理延迟预算",
            "So the key insight here is that we need to handle the latency budget.",
            "让我来给你们讲讲架构如何将采集和推论分开。",
            "This is actually a really common mistake people make.",
            "The biggest problem with this programme is the delay.",
        ] {
            assert!(
                degenerate_translation(s).is_none(),
                "不该判为退化：{s}（{:?}）",
                degenerate_translation(s)
            );
        }
    }

    #[test]
    fn short_repeats_are_tolerated() {
        assert!(degenerate_translation("的的").is_none());
        assert!(degenerate_translation("嗯嗯").is_none());
    }

    #[test]
    fn natural_repetition_in_real_speech_survives() {
        assert!(degenerate_translation("这个真的很重要很重要").is_none());
    }
}
