use translation::{
    LanguageAvailability, LanguageAvailabilityStatus, TranslationError, TranslationSession,
    TranslationSessionConfiguration, TranslationStrategy,
};

use super::{degenerate_translation, TranslateCaps, TranslateError, Translator};
use crate::config::{TranslateConfig, ZhScript};

pub struct MacTranslator {
    session: Option<TranslationSession>,
}

// SAFETY: TranslationSession is not main-actor-bound. The translator object is shared through a
// mutex and its session is only accessed while that mutex is held, so the raw Swift handle is never
// used concurrently. The session is created during init and moved to the translate worker thread.
unsafe impl Send for MacTranslator {}

impl MacTranslator {
    pub fn new() -> Self {
        Self { session: None }
    }
}

fn apple_language<'a>(lang: &'a str, zh_script: ZhScript, target: bool) -> &'a str {
    match lang {
        "en" => "en",
        "zh" if target && zh_script == ZhScript::Traditional => "zh-Hant",
        "zh" => "zh-Hans",
        _ => lang,
    }
}

fn map_error(error: TranslationError) -> TranslateError {
    match error {
        TranslationError::UnsupportedLanguagePairing(message)
        | TranslationError::UnsupportedSourceLanguage(message)
        | TranslationError::UnsupportedTargetLanguage(message) => {
            TranslateError::PlatformUnavailable(message)
        }
        other => TranslateError::PlatformUnavailable(other.to_string()),
    }
}

impl Default for MacTranslator {
    fn default() -> Self {
        Self::new()
    }
}

impl Translator for MacTranslator {
    fn init(&mut self, cfg: &TranslateConfig) -> Result<(), TranslateError> {
        let source = apple_language(&cfg.source_lang, cfg.zh_script, false);
        let target = apple_language(&cfg.target_lang, cfg.zh_script, true);
        if source == target {
            return Err(TranslateError::UnsupportedPair {
                from: source.to_string(),
                to: target.to_string(),
            });
        }

        let availability = LanguageAvailability::new()
            .map_err(|e| TranslateError::PlatformUnavailable(e.to_string()))?;
        match availability
            .status_for_pair(source, target)
            .map_err(|e| TranslateError::PlatformUnavailable(e.to_string()))?
        {
            LanguageAvailabilityStatus::Installed => {}
            LanguageAvailabilityStatus::Supported => {
                return Err(TranslateError::PlatformUnavailable(format!(
                    "{source} → {target} 语言包未安装"
                )));
            }
            LanguageAvailabilityStatus::Unsupported => {
                return Err(TranslateError::UnsupportedPair {
                    from: source.to_string(),
                    to: target.to_string(),
                });
            }
            LanguageAvailabilityStatus::Unknown(raw) => {
                return Err(TranslateError::PlatformUnavailable(format!(
                    "未知状态 {raw}"
                )));
            }
        }

        let configuration = TranslationSessionConfiguration::new(source, target)
            .with_preferred_strategy(TranslationStrategy::LowLatency);
        let session = TranslationSession::new(configuration).map_err(map_error)?;
        if !session.is_ready().map_err(map_error)? {
            return Err(TranslateError::PlatformUnavailable(format!(
                "{source} → {target} 尚未就绪"
            )));
        }

        self.session = Some(session);
        Ok(())
    }

    fn capabilities(&self) -> TranslateCaps {
        TranslateCaps::MAC_TRANSLATION
    }

    fn translate(&mut self, text: &str) -> Result<String, TranslateError> {
        let session = self
            .session
            .as_ref()
            .ok_or(TranslateError::NotInitialized)?;
        let response = session.translate(text).map_err(map_error)?;
        let translated = response.target_text().to_string();
        if let Some(reason) = degenerate_translation(&translated) {
            return Err(TranslateError::Degenerate { reason });
        }
        Ok(translated)
    }

    fn shutdown(&mut self) {
        self.session = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_script_maps_to_the_matching_apple_language() {
        assert_eq!(apple_language("zh", ZhScript::Simplified, true), "zh-Hans");
        assert_eq!(apple_language("zh", ZhScript::Traditional, true), "zh-Hant");
        assert_eq!(
            apple_language("zh", ZhScript::Traditional, false),
            "zh-Hans"
        );
        assert_eq!(apple_language("en", ZhScript::Traditional, true), "en");
    }
}
