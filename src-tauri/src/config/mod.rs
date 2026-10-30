use serde::{Deserialize, Serialize};
use typeshare::typeshare;

use crate::models::catalog;

#[typeshare]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum EngineType {
    #[default]
    LocalStreaming,
    LocalWhisper,
    MacNative,
    RemoteApi,
}

impl EngineType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LocalStreaming => "local-streaming",
            Self::LocalWhisper => "local-whisper",
            Self::MacNative => "mac-native",
            Self::RemoteApi => "remote-api",
        }
    }

    pub fn is_wired(self) -> bool {
        matches!(self, Self::LocalStreaming | Self::MacNative)
    }
}

#[typeshare]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteApiConfig {
    pub endpoint: String,
    pub api_key_ref: String,
    pub timeout_ms: u32,
}

#[typeshare]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct EndpointConfig {
    pub trailing_silence_ms: u32,
    pub max_utterance_ms: u32,
}

#[typeshare]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum EndpointProfile {
    #[default]
    Interpretation,
    Dictation,
}

impl EndpointConfig {
    pub fn for_profile(profile: EndpointProfile) -> Self {
        match profile {
            EndpointProfile::Interpretation => Self {
                trailing_silence_ms: 800,
                max_utterance_ms: 8_000,
            },
            EndpointProfile::Dictation => Self {
                trailing_silence_ms: 1_200,
                max_utterance_ms: 20_000,
            },
        }
    }
}

impl EndpointConfig {
    pub fn sanitized(mut self) -> Self {
        self.trailing_silence_ms = self
            .trailing_silence_ms
            .clamp(MIN_TRAILING_SILENCE_MS, MAX_TRAILING_SILENCE_MS);
        self.max_utterance_ms = self
            .max_utterance_ms
            .clamp(MIN_MAX_UTTERANCE_MS, MAX_MAX_UTTERANCE_MS);

        if self.max_utterance_ms < self.trailing_silence_ms {
            self.max_utterance_ms = self.trailing_silence_ms;
        }
        self
    }
}

impl Default for EndpointConfig {
    fn default() -> Self {
        Self::for_profile(EndpointProfile::Interpretation)
    }
}

impl EndpointConfig {
    pub fn to_recognizer_rules(&self) -> (f32, f32, f32) {
        (
            2.4,
            self.trailing_silence_ms as f32 / 1000.0,
            self.max_utterance_ms as f32 / 1000.0,
        )
    }
}

#[typeshare]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AsrConfig {
    pub engine_type: EngineType,
    pub source_lang: String,
    pub target_lang: String,
    pub model_dir: Option<String>,
    pub remote_api: Option<RemoteApiConfig>,
    pub endpoint: EndpointConfig,
}

impl Default for AsrConfig {
    fn default() -> Self {
        Self {
            engine_type: EngineType::LocalStreaming,
            source_lang: "zh".into(),
            target_lang: "zh".into(),
            model_dir: None,
            remote_api: None,
            endpoint: EndpointConfig::default(),
        }
    }
}

#[typeshare]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum TranslateEngine {
    Off,
    #[default]
    LocalNmt,
    MacTranslation,
    RemoteApi,
    Mock,
}

impl TranslateEngine {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::LocalNmt => "local-nmt",
            Self::MacTranslation => "mac-translation",
            Self::RemoteApi => "remote-api",
            Self::Mock => "mock",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "off" => Some(Self::Off),
            "local-nmt" => Some(Self::LocalNmt),
            "mac-translation" => Some(Self::MacTranslation),
            "remote-api" => Some(Self::RemoteApi),
            "mock" => Some(Self::Mock),
            _ => None,
        }
    }
}

#[typeshare]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TranslateConfig {
    pub engine: TranslateEngine,
    pub source_lang: String,
    pub target_lang: String,
    pub zh_script: ZhScript,
    pub model_dir: Option<String>,
    pub mt_model_tier: MtModelTier,
}

impl Default for TranslateConfig {
    fn default() -> Self {
        Self {
            engine: TranslateEngine::default(),
            source_lang: "en".into(),
            target_lang: "zh".into(),
            zh_script: ZhScript::default(),
            model_dir: None,
            mt_model_tier: MtModelTier::default(),
        }
    }
}

#[typeshare]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SessionConfig {
    pub asr: AsrConfig,
    pub translate: TranslateConfig,
}

#[typeshare]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OverlayConfig {
    pub enable: bool,
    pub mode: OverlayMode,
    pub always_on_top: bool,
    pub opacity: u8,
    pub font_size: u8,
    pub font_color: String,
    pub bg_color: String,
    pub max_lines: u16,
    pub glow: bool,
    pub glow_speed: u8,
    pub glow_width: u8,
}

#[typeshare]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum OverlayMode {
    Locked,
    #[default]
    Edit,
}

impl OverlayMode {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "locked" => Some(Self::Locked),
            "edit" => Some(Self::Edit),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Locked => "locked",
            Self::Edit => "edit",
        }
    }

    pub fn is_click_through(self) -> bool {
        matches!(self, Self::Locked)
    }
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            enable: true,
            mode: OverlayMode::Edit,
            always_on_top: true,
            opacity: 60,
            font_size: 22,
            font_color: "#ffffff".into(),
            bg_color: "#101018".into(),
            max_lines: 6,
            glow: false,
            glow_speed: 0,
            glow_width: 2,
        }
    }
}

#[typeshare]
pub const MIN_FONT_SIZE: u8 = 10;
#[typeshare]
pub const MAX_FONT_SIZE: u8 = 48;
#[typeshare]
pub const MAX_OVERLAY_LINES: u16 = 20;
#[typeshare]
pub const MAX_GLOW_SPEED: u8 = 100;
#[typeshare]
pub const MIN_GLOW_WIDTH: u8 = 1;
#[typeshare]
pub const MAX_GLOW_WIDTH: u8 = 8;

#[typeshare]
pub const MIN_TRAILING_SILENCE_MS: u32 = 200;
#[typeshare]
pub const MAX_TRAILING_SILENCE_MS: u32 = 2_000;
#[typeshare]
pub const MIN_MAX_UTTERANCE_MS: u32 = 2_000;
#[typeshare]
pub const MAX_MAX_UTTERANCE_MS: u32 = 20_000;

#[typeshare]
pub const MIN_OVERLAY_WIDTH: u32 = 320;
#[typeshare]
pub const MIN_OVERLAY_HEIGHT: u32 = 80;
#[typeshare]
pub const MAX_OVERLAY_WIDTH: u32 = 4096;
#[typeshare]
pub const MAX_OVERLAY_HEIGHT: u32 = 2160;

#[typeshare]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum AsrModelTier {
    #[default]
    Builtin,
    HighAccuracy,
    XAsr,
    Nemotron,
    Qwen3,
}

impl AsrModelTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Builtin => "builtin",
            Self::HighAccuracy => "high-accuracy",
            Self::XAsr => "x-asr",
            Self::Nemotron => "nemotron",
            Self::Qwen3 => "qwen3",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "builtin" => Some(Self::Builtin),
            "high-accuracy" => Some(Self::HighAccuracy),
            "x-asr" => Some(Self::XAsr),
            "nemotron" => Some(Self::Nemotron),
            "qwen3" => Some(Self::Qwen3),
            _ => None,
        }
    }
}

#[typeshare]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum MtModelTier {
    #[default]
    Base,
    Multilingual,
    LmtFast,
    LmtHigh,
}

impl MtModelTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Base => "base",
            Self::Multilingual => "multilingual",
            Self::LmtFast => "lmt-fast",
            Self::LmtHigh => "lmt-high",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "base" => Some(Self::Base),
            "multilingual" => Some(Self::Multilingual),
            "lmt-fast" => Some(Self::LmtFast),
            "lmt-high" => Some(Self::LmtHigh),
            _ => None,
        }
    }
}

#[typeshare]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ZhScript {
    #[default]
    Simplified,
    Traditional,
}

impl ZhScript {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Simplified => "simplified",
            Self::Traditional => "traditional",
        }
    }
}

#[typeshare]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[typeshare]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Language {
    #[default]
    #[serde(rename = "system")]
    System,
    #[serde(rename = "zh-CN")]
    ZhCn,
    #[serde(rename = "zh-TW")]
    ZhTw,
    #[serde(rename = "en")]
    En,
    #[serde(rename = "ja")]
    Ja,
    #[serde(rename = "ko")]
    Ko,
    #[serde(rename = "fr")]
    Fr,
    #[serde(rename = "de")]
    De,
    #[serde(rename = "es")]
    Es,
}

#[typeshare]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowPos {
    pub x: i32,
    pub y: i32,
}

#[typeshare]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

#[typeshare]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowGeometry {
    pub position: WindowPos,
    pub size: WindowSize,
}

impl Default for WindowGeometry {
    fn default() -> Self {
        Self {
            position: WindowPos { x: 0, y: 0 },
            size: WindowSize {
                width: 520,
                height: 160,
            },
        }
    }
}

impl OverlayConfig {
    pub fn sanitized(mut self) -> Self {
        let d = Self::default();

        self.opacity = self.opacity.min(100);
        self.font_size = self.font_size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE);
        self.max_lines = self.max_lines.clamp(1, MAX_OVERLAY_LINES);
        self.glow_speed = self.glow_speed.min(MAX_GLOW_SPEED);
        self.glow_width = self.glow_width.clamp(MIN_GLOW_WIDTH, MAX_GLOW_WIDTH);
        self.font_color = normalize_color(&self.font_color, &d.font_color);
        self.bg_color = normalize_color(&self.bg_color, &d.bg_color);

        self
    }
}

fn normalize_color(raw: &str, fallback: &str) -> String {
    let t = raw.trim().trim_start_matches('#');
    if !t.chars().all(|c| c.is_ascii_hexdigit()) {
        return fallback.to_owned();
    }
    match t.len() {
        6 => format!("#{}", t.to_ascii_lowercase()),
        3 => {
            let mut out = String::with_capacity(7);
            out.push('#');
            for c in t.chars() {
                out.push(c.to_ascii_lowercase());
                out.push(c.to_ascii_lowercase());
            }
            out
        }
        _ => fallback.to_owned(),
    }
}

#[typeshare]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Preferences {
    pub audio_source: String,
    pub device_id: Option<String>,
    pub mix_mic_device_id: Option<String>,
    pub mix_system_device_id: Option<String>,
    pub translate_engine: TranslateEngine,
    pub asr_engine: EngineType,
    pub source_lang: String,
    pub target_lang: String,
    pub zh_script: ZhScript,
    pub overlay: OverlayConfig,
    pub theme: Theme,
    pub language: Language,
    pub sidebar_visible: bool,
    pub shortcuts: std::collections::HashMap<String, String>,
    pub endpoint: EndpointConfig,
    pub overlay_geometry: Option<WindowGeometry>,
    pub onboarding_done: bool,
    pub asr_model_tier: AsrModelTier,
    pub mt_model_tier: MtModelTier,
    pub models_dir: Option<String>,
    pub analytics_enabled: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            audio_source: "loopback".into(),
            device_id: None,
            mix_mic_device_id: None,
            mix_system_device_id: None,
            translate_engine: TranslateEngine::default(),
            asr_engine: EngineType::default(),
            source_lang: "en".into(),
            target_lang: "zh".into(),
            zh_script: ZhScript::default(),
            overlay: OverlayConfig::default(),
            theme: Theme::default(),
            language: Language::default(),
            sidebar_visible: true,
            endpoint: EndpointConfig::default(),
            shortcuts: std::collections::HashMap::new(),
            overlay_geometry: None,
            onboarding_done: false,
            asr_model_tier: AsrModelTier::default(),
            mt_model_tier: MtModelTier::default(),
            models_dir: None,
            analytics_enabled: true,
        }
    }
}

#[typeshare]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PrefsPatch {
    pub audio_source: Option<String>,
    pub device_id: Option<String>,
    pub mix_mic_device_id: Option<String>,
    pub mix_system_device_id: Option<String>,
    pub translate_engine: Option<TranslateEngine>,
    pub asr_engine: Option<EngineType>,
    pub source_lang: Option<String>,
    pub target_lang: Option<String>,
    pub zh_script: Option<ZhScript>,
    pub endpoint: Option<EndpointConfig>,
    pub theme: Option<Theme>,
    pub language: Option<Language>,
    pub sidebar_visible: Option<bool>,
    pub onboarding_done: Option<bool>,
    pub asr_model_tier: Option<AsrModelTier>,
    pub analytics_enabled: Option<bool>,
}

impl PrefsPatch {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(s) = &self.audio_source {
            if !matches!(s.as_str(), "mic" | "loopback" | "mix") {
                return Err(crate::i18n::tf(
                    crate::i18n::Key::PrefsUnknownAudioSource,
                    &[s],
                ));
            }
        }
        if let Some(engine) = self.asr_engine {
            if !engine.is_wired() {
                return Err(crate::i18n::tf(
                    crate::i18n::Key::PrefsUnknownAsrEngine,
                    &[engine.as_str()],
                ));
            }
        }
        if let Some(s) = &self.source_lang {
            if s == "auto" || !is_language_tag(s) {
                return Err(crate::i18n::tf(
                    crate::i18n::Key::PrefsUnknownSourceLang,
                    &[s],
                ));
            }
        }
        if let Some(s) = &self.target_lang {
            if !is_language_tag(s) {
                return Err(crate::i18n::tf(
                    crate::i18n::Key::PrefsUnknownTargetLang,
                    &[s],
                ));
            }
        }
        Ok(())
    }

    pub fn apply_to(self, p: &mut Preferences) {
        if let Some(v) = self.audio_source {
            p.audio_source = v;
        }
        if let Some(v) = self.device_id {
            p.device_id = normalize_device(Some(v));
        }
        if let Some(v) = self.mix_mic_device_id {
            p.mix_mic_device_id = normalize_device(Some(v));
        }
        if let Some(v) = self.mix_system_device_id {
            p.mix_system_device_id = normalize_device(Some(v));
        }
        if let Some(v) = self.translate_engine {
            p.translate_engine = v;
        }
        if let Some(v) = self.asr_engine {
            p.asr_engine = v;
        }
        if let Some(v) = self.source_lang {
            p.source_lang = v;
        }
        if let Some(v) = self.target_lang {
            p.target_lang = v;
        }
        if let Some(v) = self.zh_script {
            p.zh_script = v;
        }
        if let Some(v) = self.language {
            p.language = v;
        }
        if let Some(v) = self.endpoint {
            p.endpoint = v;
        }
        if let Some(v) = self.theme {
            p.theme = v;
        }
        if let Some(v) = self.sidebar_visible {
            p.sidebar_visible = v;
        }
        if let Some(v) = self.onboarding_done {
            p.onboarding_done = v;
        }
        if let Some(v) = self.asr_model_tier {
            p.asr_model_tier = v;
        }
        if let Some(v) = self.analytics_enabled {
            p.analytics_enabled = v;
        }
    }
}

impl Preferences {
    pub fn sanitized(mut self) -> Self {
        let d = Self::default();

        if !matches!(self.audio_source.as_str(), "mic" | "loopback" | "mix") {
            self.audio_source = d.audio_source;
        }
        if !self.asr_engine.is_wired() {
            self.asr_engine = d.asr_engine;
        }
        if !source_lang_allowed(
            self.translate_engine,
            self.asr_engine,
            self.asr_model_tier,
            self.mt_model_tier,
            &self.source_lang,
        ) {
            self.source_lang = d.source_lang;
        }
        let target_lang = effective_target_lang(
            self.translate_engine,
            self.mt_model_tier,
            &self.source_lang,
            &self.target_lang,
        )
        .to_owned();
        self.target_lang = target_lang;
        self.device_id = normalize_device(self.device_id.take());
        self.mix_mic_device_id = normalize_device(self.mix_mic_device_id.take());
        self.mix_system_device_id = normalize_device(self.mix_system_device_id.take());
        self.models_dir = self
            .models_dir
            .take()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty());

        self.endpoint = self.endpoint.sanitized();

        if let Some(g) = &mut self.overlay_geometry {
            g.size.width = g.size.width.clamp(MIN_OVERLAY_WIDTH, MAX_OVERLAY_WIDTH);
            g.size.height = g.size.height.clamp(MIN_OVERLAY_HEIGHT, MAX_OVERLAY_HEIGHT);
        }

        self
    }

    pub fn apply_session(&mut self, source: &str, device: Option<String>) {
        self.audio_source = source.to_owned();
        self.device_id = normalize_device(device);
    }

    pub fn apply_mix_devices(&mut self, mic: Option<String>, system: Option<String>) {
        self.mix_mic_device_id = normalize_device(mic);
        self.mix_system_device_id = normalize_device(system);
    }
}

pub fn direction_label(source_lang: &str, target_lang: &str, zh_script: ZhScript) -> String {
    use crate::i18n::{tf, Key};
    tf(
        Key::DirectionPair,
        &[
            lang_name(source_lang, zh_script),
            lang_name(target_lang, zh_script),
        ],
    )
}

fn lang_name(lang: &str, zh_script: ZhScript) -> &str {
    use crate::i18n::{tr, Key};
    match lang {
        "en" => tr(Key::LangEnglish),
        "zh" if zh_script == ZhScript::Traditional => tr(Key::LangChineseTraditional),
        "zh" => tr(Key::LangChineseSimplified),
        "ja" => tr(Key::LangJapanese),
        "ko" => tr(Key::LangKorean),
        "fr" => tr(Key::LangFrench),
        "de" => tr(Key::LangGerman),
        "es" => tr(Key::LangSpanish),
        other => other,
    }
}

pub fn target_lang_for(source_lang: &str) -> &'static str {
    if source_lang == "zh" {
        "en"
    } else {
        "zh"
    }
}

pub fn is_supported_source_lang(lang: &str) -> bool {
    matches!(lang, "en" | "zh")
}

fn asr_supports_source(asr_engine: EngineType, asr_tier: AsrModelTier, lang: &str) -> bool {
    asr_engine == EngineType::MacNative || catalog::asr_entry(asr_tier).supports_language(lang)
}

pub fn source_lang_allowed(
    engine: TranslateEngine,
    asr_engine: EngineType,
    asr_tier: AsrModelTier,
    mt_tier: MtModelTier,
    lang: &str,
) -> bool {
    if lang == "auto" || !asr_supports_source(asr_engine, asr_tier, lang) {
        return false;
    }
    match engine {
        TranslateEngine::MacTranslation => is_language_tag(lang),
        TranslateEngine::LocalNmt => catalog::mt_entry(mt_tier).supports_language(lang),
        _ => is_supported_source_lang(lang),
    }
}

pub fn is_language_tag(value: &str) -> bool {
    !value.is_empty()
        && value.split('-').all(|part| {
            !part.is_empty() && part.len() <= 8 && part.chars().all(|c| c.is_ascii_alphanumeric())
        })
}

pub(crate) fn base_lang(lang: &str) -> &str {
    lang.split('-').next().unwrap_or(lang)
}

pub fn normalize_device(id: Option<String>) -> Option<String> {
    id.filter(|s| !s.is_empty())
}

pub fn effective_target_lang<'a>(
    engine: TranslateEngine,
    mt_tier: MtModelTier,
    source_lang: &str,
    target_lang: &'a str,
) -> &'a str {
    let supported = base_lang(target_lang) != base_lang(source_lang)
        && match engine {
            TranslateEngine::MacTranslation => is_language_tag(target_lang),
            TranslateEngine::LocalNmt => catalog::mt_entry(mt_tier).supports_language(target_lang),
            _ => matches!(target_lang, "en" | "zh"),
        };
    if supported {
        target_lang
    } else {
        target_lang_for(source_lang)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_thresholds_are_never_zero() {
        for profile in [EndpointProfile::Interpretation, EndpointProfile::Dictation] {
            let (r1, r2, r3) = EndpointConfig::for_profile(profile).to_recognizer_rules();
            assert!(r1 > 0.0, "{profile:?} 的 rule1 为 0");
            assert!(r2 > 0.0, "{profile:?} 的 rule2 为 0");
            assert!(r3 > 0.0, "{profile:?} 的 rule3 为 0");
        }
    }

    #[test]
    fn interpretation_segments_are_shorter_than_dictation() {
        let interp = EndpointConfig::for_profile(EndpointProfile::Interpretation);
        let dict = EndpointConfig::for_profile(EndpointProfile::Dictation);

        assert!(
            interp.trailing_silence_ms < dict.trailing_silence_ms,
            "同传的静音阈值必须更短：{} vs {}",
            interp.trailing_silence_ms,
            dict.trailing_silence_ms
        );
        assert!(
            interp.max_utterance_ms < dict.max_utterance_ms,
            "同传的最长句必须更短：{} vs {}",
            interp.max_utterance_ms,
            dict.max_utterance_ms
        );
        assert!(
            interp.max_utterance_ms <= 8_000,
            "同传最长句 {}ms 过长，译文会明显滞后",
            interp.max_utterance_ms
        );
    }

    #[test]
    fn default_profile_is_interpretation() {
        assert_eq!(
            EndpointConfig::default(),
            EndpointConfig::for_profile(EndpointProfile::Interpretation)
        );
    }

    #[test]
    fn asr_engine_pref_accepts_only_wired_engines() {
        let ok: PrefsPatch = serde_json::from_str(r#"{"asrEngine":"mac-native"}"#).unwrap();
        assert_eq!(ok.asr_engine, Some(EngineType::MacNative));
        assert!(ok.validate().is_ok());

        let bad: PrefsPatch = serde_json::from_str(r#"{"asrEngine":"local-whisper"}"#).unwrap();
        assert!(bad.validate().is_err(), "未接线的引擎必须被偏好校验拒绝");
    }

    #[test]
    fn asr_engine_defaults_to_local_streaming() {
        assert_eq!(
            Preferences::default().asr_engine,
            EngineType::LocalStreaming
        );
    }

    #[test]
    fn engine_type_serializes_kebab_case() {
        let json = serde_json::to_string(&EngineType::LocalStreaming).unwrap();
        assert_eq!(json, r#""local-streaming""#);
    }

    #[test]
    fn overlay_mode_serializes_camel_case() {
        assert_eq!(
            serde_json::to_string(&OverlayMode::Locked).unwrap(),
            r#""locked""#
        );
    }

    #[test]
    fn asr_config_roundtrips_in_camel_case() {
        let cfg = AsrConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(json.contains("\"engineType\""));
        assert!(json.contains("\"sourceLang\""));
        let back: AsrConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    fn default_audio_source_is_system_loopback() {
        assert_eq!(Preferences::default().audio_source, "loopback");
    }

    #[test]
    fn sanitize_falls_back_per_field_not_wholesale() {
        let p = Preferences {
            audio_source: "speaker".into(),
            device_id: Some("keep-me".into()),
            translate_engine: TranslateEngine::Off,
            asr_engine: crate::config::EngineType::LocalStreaming,
            source_lang: "zh".into(),
            ..Default::default()
        }
        .sanitized();

        assert_eq!(p.audio_source, "loopback", "坏字段应回落");
        assert_eq!(p.device_id.as_deref(), Some("keep-me"), "好字段必须留下");
        assert_eq!(p.translate_engine, TranslateEngine::Off, "好字段必须留下");
        assert_eq!(p.source_lang, "zh", "好字段必须留下");
    }

    #[test]
    fn sanitize_rejects_auto_as_source_lang() {
        let p = Preferences {
            source_lang: "auto".into(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(p.source_lang, "en");
    }

    #[test]
    fn asr_tier_strings_roundtrip() {
        for tier in [
            AsrModelTier::Builtin,
            AsrModelTier::HighAccuracy,
            AsrModelTier::XAsr,
            AsrModelTier::Nemotron,
            AsrModelTier::Qwen3,
        ] {
            assert_eq!(AsrModelTier::parse(tier.as_str()), Some(tier));
        }
    }

    #[test]
    fn mt_tier_strings_roundtrip() {
        for tier in [
            MtModelTier::Base,
            MtModelTier::Multilingual,
            MtModelTier::LmtFast,
            MtModelTier::LmtHigh,
        ] {
            assert_eq!(MtModelTier::parse(tier.as_str()), Some(tier));
        }
    }

    #[test]
    fn sanitize_keeps_multilingual_source_only_for_multilingual_asr() {
        let kept = Preferences {
            translate_engine: TranslateEngine::MacTranslation,
            asr_model_tier: AsrModelTier::Qwen3,
            source_lang: "ja".into(),
            target_lang: "zh".into(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(kept.source_lang, "ja");

        let nemotron = Preferences {
            translate_engine: TranslateEngine::MacTranslation,
            asr_model_tier: AsrModelTier::Nemotron,
            source_lang: "ja".into(),
            target_lang: "zh".into(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(nemotron.source_lang, "ja");

        let streaming = Preferences {
            translate_engine: TranslateEngine::MacTranslation,
            source_lang: "ja".into(),
            target_lang: "zh".into(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(streaming.source_lang, "en");

        let x_asr = Preferences {
            translate_engine: TranslateEngine::MacTranslation,
            asr_model_tier: AsrModelTier::XAsr,
            source_lang: "ja".into(),
            target_lang: "zh".into(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(x_asr.source_lang, "en");

        let local = Preferences {
            translate_engine: TranslateEngine::LocalNmt,
            asr_model_tier: AsrModelTier::Qwen3,
            source_lang: "ja".into(),
            target_lang: "zh".into(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(local.source_lang, "en");

        let multilingual = Preferences {
            translate_engine: TranslateEngine::LocalNmt,
            asr_model_tier: AsrModelTier::Qwen3,
            mt_model_tier: MtModelTier::Multilingual,
            source_lang: "ja".into(),
            target_lang: "zh".into(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(multilingual.source_lang, "ja");

        let nemotron = Preferences {
            translate_engine: TranslateEngine::LocalNmt,
            asr_model_tier: AsrModelTier::Nemotron,
            mt_model_tier: MtModelTier::Multilingual,
            source_lang: "ja".into(),
            target_lang: "zh".into(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(nemotron.source_lang, "ja");
    }

    #[test]
    fn source_lang_allowed_follows_engine_and_tier() {
        assert!(source_lang_allowed(
            TranslateEngine::MacTranslation,
            EngineType::LocalStreaming,
            AsrModelTier::Qwen3,
            MtModelTier::Base,
            "ja"
        ));
        assert!(!source_lang_allowed(
            TranslateEngine::MacTranslation,
            EngineType::LocalStreaming,
            AsrModelTier::Builtin,
            MtModelTier::Base,
            "ja"
        ));
        assert!(!source_lang_allowed(
            TranslateEngine::LocalNmt,
            EngineType::LocalStreaming,
            AsrModelTier::Qwen3,
            MtModelTier::Base,
            "ja"
        ));
        assert!(source_lang_allowed(
            TranslateEngine::LocalNmt,
            EngineType::LocalStreaming,
            AsrModelTier::Qwen3,
            MtModelTier::Multilingual,
            "ja"
        ));
        assert!(source_lang_allowed(
            TranslateEngine::LocalNmt,
            EngineType::LocalStreaming,
            AsrModelTier::Qwen3,
            MtModelTier::LmtFast,
            "ja"
        ));
        assert!(source_lang_allowed(
            TranslateEngine::LocalNmt,
            EngineType::LocalStreaming,
            AsrModelTier::Qwen3,
            MtModelTier::LmtHigh,
            "ja"
        ));
        assert!(source_lang_allowed(
            TranslateEngine::LocalNmt,
            EngineType::LocalStreaming,
            AsrModelTier::Nemotron,
            MtModelTier::Multilingual,
            "ja"
        ));
        assert!(!source_lang_allowed(
            TranslateEngine::LocalNmt,
            EngineType::LocalStreaming,
            AsrModelTier::XAsr,
            MtModelTier::Multilingual,
            "ja"
        ));
        assert!(!source_lang_allowed(
            TranslateEngine::LocalNmt,
            EngineType::LocalStreaming,
            AsrModelTier::Builtin,
            MtModelTier::Multilingual,
            "ja"
        ));
        assert!(!source_lang_allowed(
            TranslateEngine::MacTranslation,
            EngineType::LocalStreaming,
            AsrModelTier::Qwen3,
            MtModelTier::Base,
            "auto"
        ));
        assert!(source_lang_allowed(
            TranslateEngine::MacTranslation,
            EngineType::MacNative,
            AsrModelTier::Builtin,
            MtModelTier::Base,
            "ja"
        ));
        assert!(!source_lang_allowed(
            TranslateEngine::LocalNmt,
            EngineType::MacNative,
            AsrModelTier::Builtin,
            MtModelTier::Base,
            "ja"
        ));
    }

    #[test]
    fn source_lang_allowed_uses_the_language_intersection() {
        assert!(source_lang_allowed(
            TranslateEngine::LocalNmt,
            EngineType::LocalStreaming,
            AsrModelTier::Qwen3,
            MtModelTier::LmtFast,
            "th"
        ));
        assert!(!source_lang_allowed(
            TranslateEngine::LocalNmt,
            EngineType::LocalStreaming,
            AsrModelTier::Qwen3,
            MtModelTier::LmtFast,
            "mk"
        ));
        assert!(!source_lang_allowed(
            TranslateEngine::LocalNmt,
            EngineType::LocalStreaming,
            AsrModelTier::Nemotron,
            MtModelTier::LmtFast,
            "et"
        ));
        assert!(!source_lang_allowed(
            TranslateEngine::LocalNmt,
            EngineType::LocalStreaming,
            AsrModelTier::Nemotron,
            MtModelTier::Multilingual,
            "uk"
        ));
        assert!(source_lang_allowed(
            TranslateEngine::LocalNmt,
            EngineType::LocalStreaming,
            AsrModelTier::Qwen3,
            MtModelTier::LmtFast,
            "zh-TW"
        ));
    }

    #[test]
    fn sanitize_repairs_unsupported_or_same_target_language() {
        let same = Preferences {
            source_lang: "zh".into(),
            target_lang: "zh".into(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(same.target_lang, "en");

        let unknown = Preferences {
            source_lang: "en".into(),
            target_lang: "xx".into(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(unknown.target_lang, "zh");

        let valid = Preferences {
            source_lang: "zh".into(),
            target_lang: "en".into(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(valid.target_lang, "en");
    }

    #[test]
    fn empty_device_id_means_system_default() {
        let p = Preferences {
            device_id: Some(String::new()),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(p.device_id, None);
    }

    #[test]
    fn target_lang_derivation_is_a_single_function() {
        assert_eq!(target_lang_for("en"), "zh");
        assert_eq!(target_lang_for("zh"), "en");
        assert_eq!(target_lang_for("auto"), "zh");
    }

    #[test]
    fn extra_target_languages_survive_sanitizing() {
        assert_eq!(
            effective_target_lang(
                TranslateEngine::MacTranslation,
                MtModelTier::Base,
                "en",
                "ja"
            ),
            "ja"
        );
        assert_eq!(
            effective_target_lang(
                TranslateEngine::MacTranslation,
                MtModelTier::Base,
                "zh",
                "fr"
            ),
            "fr"
        );
        assert_eq!(
            effective_target_lang(
                TranslateEngine::MacTranslation,
                MtModelTier::Base,
                "en",
                "ko"
            ),
            "ko"
        );
        assert_eq!(
            effective_target_lang(
                TranslateEngine::MacTranslation,
                MtModelTier::Base,
                "en",
                "de-Latn-DE"
            ),
            "de-Latn-DE"
        );
        assert_eq!(
            effective_target_lang(
                TranslateEngine::LocalNmt,
                MtModelTier::Multilingual,
                "en",
                "ja"
            ),
            "ja"
        );

        let mac = Preferences {
            translate_engine: TranslateEngine::MacTranslation,
            source_lang: "en".into(),
            target_lang: "ja".into(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(mac.target_lang, "ja");

        let local = Preferences {
            translate_engine: TranslateEngine::LocalNmt,
            source_lang: "en".into(),
            target_lang: "ja".into(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(local.target_lang, "zh", "Opus-MT 不支持的目标语言必须回退");

        let multilingual = Preferences {
            translate_engine: TranslateEngine::LocalNmt,
            mt_model_tier: MtModelTier::Multilingual,
            source_lang: "en".into(),
            target_lang: "ja".into(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(multilingual.target_lang, "ja");

        assert_eq!(
            effective_target_lang(TranslateEngine::LocalNmt, MtModelTier::LmtFast, "en", "th"),
            "th"
        );
        assert_eq!(
            effective_target_lang(TranslateEngine::LocalNmt, MtModelTier::LmtFast, "en", "uk"),
            "zh"
        );

        let lmt_source = Preferences {
            translate_engine: TranslateEngine::LocalNmt,
            asr_model_tier: AsrModelTier::Qwen3,
            mt_model_tier: MtModelTier::LmtFast,
            source_lang: "mk".into(),
            target_lang: "zh".into(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(lmt_source.source_lang, "en", "LMT 不支持的语言必须回退");

        let lmt_target = Preferences {
            translate_engine: TranslateEngine::LocalNmt,
            asr_model_tier: AsrModelTier::Qwen3,
            mt_model_tier: MtModelTier::LmtFast,
            source_lang: "en".into(),
            target_lang: "uk".into(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(lmt_target.target_lang, "zh", "LMT 不支持的译文语言必须回退");
    }

    #[test]
    fn extra_sources_pass_patch_validation_and_are_gated_at_sanitize() {
        let target = PrefsPatch {
            target_lang: Some("fr".into()),
            ..Default::default()
        };
        assert!(target.validate().is_ok());

        let source = PrefsPatch {
            source_lang: Some("fr".into()),
            ..Default::default()
        };
        assert!(source.validate().is_ok(), "补丁层只校验语言标签格式");

        let auto = PrefsPatch {
            source_lang: Some("auto".into()),
            ..Default::default()
        };
        assert!(auto.validate().is_err(), "auto 必须在补丁层拒绝");

        let streaming = Preferences {
            source_lang: "fr".into(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(streaming.source_lang, "en", "非 Qwen3+macOS 组合必须回退");

        let multilingual = Preferences {
            translate_engine: TranslateEngine::MacTranslation,
            asr_model_tier: AsrModelTier::Qwen3,
            source_lang: "fr".into(),
            target_lang: "zh".into(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(multilingual.source_lang, "fr");
    }

    #[test]
    fn direction_label_renders_language_names() {
        assert_eq!(
            direction_label("en", "zh", ZhScript::Simplified),
            "英语 → 简体中文"
        );
        assert_eq!(
            direction_label("zh", "en", ZhScript::Simplified),
            "简体中文 → 英语"
        );
        assert_eq!(
            direction_label("en", "zh", ZhScript::Traditional),
            "英语 → 繁体中文"
        );
        assert_eq!(
            direction_label("zh", "en", ZhScript::Traditional),
            "繁体中文 → 英语"
        );
        assert_eq!(
            direction_label("en", "ja", ZhScript::Simplified),
            "英语 → 日语"
        );
        assert_eq!(
            direction_label("zh", "fr", ZhScript::Traditional),
            "繁体中文 → 法语"
        );
        assert_eq!(
            direction_label("en", "de", ZhScript::Simplified),
            "英语 → 德语"
        );
        assert_eq!(
            direction_label("en", "es", ZhScript::Simplified),
            "英语 → 西班牙语"
        );
    }

    #[test]
    fn preferences_roundtrip_in_camel_case() {
        let p = Preferences {
            audio_source: "mic".into(),
            device_id: Some("dev-1".into()),
            translate_engine: TranslateEngine::RemoteApi,
            source_lang: "zh".into(),
            ..Default::default()
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"audioSource\""), "{json}");
        assert!(json.contains("\"translateEngine\""), "{json}");
        assert!(json.contains("\"sourceLang\""), "{json}");
        assert!(json.contains("\"remote-api\""), "{json}");
        assert_eq!(serde_json::from_str::<Preferences>(&json).unwrap(), p);
    }

    #[test]
    fn apply_session_preserves_overlay_and_geometry() {
        let mut p = Preferences {
            overlay: OverlayConfig {
                opacity: 33,
                font_size: 40,
                ..Default::default()
            },
            overlay_geometry: Some(WindowGeometry {
                position: WindowPos { x: -1200, y: 980 },
                size: WindowSize {
                    width: 800,
                    height: 240,
                },
            }),
            translate_engine: TranslateEngine::Off,
            asr_engine: crate::config::EngineType::LocalStreaming,
            source_lang: "zh".into(),
            ..Default::default()
        };

        p.apply_session("mic", Some("dev-9".into()));

        assert_eq!(p.audio_source, "mic");
        assert_eq!(p.device_id.as_deref(), Some("dev-9"));
        assert_eq!(p.overlay.opacity, 33, "调好的不透明度不能被会话启动冲掉");
        assert_eq!(p.overlay.font_size, 40, "调好的字号不能被会话启动冲掉");
        assert!(p.overlay_geometry.is_some(), "窗口位置不能被会话启动冲掉");
        assert_eq!(p.translate_engine, TranslateEngine::Off);
        assert_eq!(p.source_lang, "zh");
    }

    #[test]
    fn apply_session_drops_empty_device_id() {
        let mut p = Preferences::default();
        p.apply_session("loopback", Some(String::new()));
        assert_eq!(p.device_id, None);
    }

    #[test]
    fn apply_mix_devices_keeps_single_device_untouched() {
        let mut p = Preferences {
            device_id: Some("dev-1".into()),
            ..Default::default()
        };
        p.apply_mix_devices(Some("mic-1".into()), Some("out-1".into()));
        assert_eq!(p.mix_mic_device_id.as_deref(), Some("mic-1"));
        assert_eq!(p.mix_system_device_id.as_deref(), Some("out-1"));
        assert_eq!(p.device_id.as_deref(), Some("dev-1"));

        p.apply_mix_devices(Some(String::new()), None);
        assert_eq!(p.mix_mic_device_id, None);
        assert_eq!(p.mix_system_device_id, None);
    }

    #[test]
    fn overlay_defaults_are_the_out_of_box_setup() {
        let c = OverlayConfig::default();
        assert!(c.enable, "出箱即显示悬浮窗");
        assert_eq!(c.mode, OverlayMode::Edit, "出箱即可拖动 / 缩放");
        assert!(!c.glow, "彩色边框默认关");
        assert_eq!(c.glow_speed, 0, "流光速度默认 0：静止");
        assert_eq!(c.glow_width, 2, "边框宽度默认 2px");
    }

    #[test]
    fn overlay_config_accepts_files_without_glow_speed() {
        let c: OverlayConfig = serde_json::from_str(
            r##"{"enable":true,"mode":"edit","alwaysOnTop":true,"opacity":60,"fontSize":22,"fontColor":"#ffffff","bgColor":"#101018","maxLines":6,"glow":true}"##,
        )
        .expect("旧版偏好文件缺少 glowSpeed 时不能整份回退默认值");
        assert!(c.glow);
        assert_eq!(c.glow_speed, 0);
        assert_eq!(c.glow_width, 2, "旧版偏好文件缺少 glowWidth 时用默认宽度");
    }

    #[test]
    fn overlay_config_clamps_out_of_range_values() {
        let c = OverlayConfig {
            opacity: 250,
            font_size: 200,
            max_lines: 9999,
            glow_speed: 250,
            glow_width: 250,
            ..Default::default()
        }
        .sanitized();

        assert_eq!(c.opacity, 100, "不透明度上限是 100");
        assert_eq!(c.font_size, MAX_FONT_SIZE);
        assert_eq!(c.max_lines, MAX_OVERLAY_LINES);
        assert_eq!(c.glow_speed, MAX_GLOW_SPEED);
        assert_eq!(c.glow_width, MAX_GLOW_WIDTH);
    }

    #[test]
    fn overlay_config_clamps_zero_that_would_look_like_a_bug() {
        let c = OverlayConfig {
            max_lines: 0,
            font_size: 0,
            glow_width: 0,
            ..Default::default()
        }
        .sanitized();
        assert_eq!(c.max_lines, 1);
        assert_eq!(c.font_size, MIN_FONT_SIZE);
        assert_eq!(c.glow_width, MIN_GLOW_WIDTH);
    }

    #[test]
    fn overlay_opacity_zero_is_legal() {
        let c = OverlayConfig {
            opacity: 0,
            ..Default::default()
        }
        .sanitized();
        assert_eq!(c.opacity, 0);
    }

    #[test]
    fn overlay_sanitize_leaves_colors_and_mode_alone() {
        let c = OverlayConfig {
            font_color: "#ff0000".into(),
            bg_color: "#00ff00".into(),
            mode: OverlayMode::Edit,
            enable: false,
            ..Default::default()
        }
        .sanitized();

        assert_eq!(c.font_color, "#ff0000");
        assert_eq!(c.bg_color, "#00ff00");
        assert_eq!(c.mode, OverlayMode::Edit);
        assert!(!c.enable);
    }

    #[test]
    fn color_is_normalized_to_lowercase_hex() {
        let c = OverlayConfig {
            font_color: "  #FFAA00  ".into(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(c.font_color, "#ffaa00", "去空白并转小写");
    }

    #[test]
    fn shorthand_color_is_expanded() {
        let c = OverlayConfig {
            bg_color: "#abc".into(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(c.bg_color, "#aabbcc");
    }

    #[test]
    fn illegal_color_falls_back_rather_than_passing_through() {
        let d = OverlayConfig::default();
        for bad in ["红色", "rgb(0,0,0)", "#12", "#12345", "#gggggg", ""] {
            let c = OverlayConfig {
                font_color: bad.into(),
                ..Default::default()
            }
            .sanitized();
            assert_eq!(c.font_color, d.font_color, "非法值 {bad:?} 应回落");
        }
    }

    #[test]
    fn sanitize_does_not_clobber_a_legal_custom_color() {
        let c = OverlayConfig {
            font_color: "#00ff88".into(),
            bg_color: "#000000".into(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(c.font_color, "#00ff88");
        assert_eq!(c.bg_color, "#000000");
    }

    #[test]
    fn partial_session_config_parses_with_defaults() {
        let json = serde_json::json!({
            "translate": { "engine": "local-nmt", "sourceLang": "zh", "targetLang": "en" }
        });
        let cfg: SessionConfig =
            serde_json::from_value(json).expect("缺少 asr 时应补默认，而不是拒绝整个对象");

        assert_eq!(cfg.translate.engine, TranslateEngine::LocalNmt);
        assert_eq!(cfg.translate.source_lang, "zh");
        assert_eq!(cfg.asr, AsrConfig::default(), "缺的整块应回落默认");
    }

    #[test]
    fn empty_object_yields_all_defaults() {
        let cfg: SessionConfig = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(cfg, SessionConfig::default());
    }

    #[test]
    fn partial_nested_config_keeps_known_fields() {
        let json = serde_json::json!({ "translate": { "engine": "off" } });
        let cfg: SessionConfig = serde_json::from_value(json).unwrap();

        assert_eq!(
            cfg.translate.engine,
            TranslateEngine::Off,
            "提供的字段必须留下"
        );
        assert_eq!(cfg.translate.source_lang, "en", "未提供的字段补默认");
    }

    #[test]
    fn unknown_field_is_ignored_not_rejected() {
        let json = serde_json::json!({ "translate": { "source_lang": "zh" } });
        let cfg: SessionConfig = serde_json::from_value(json).expect("未知字段被忽略，不报错");

        assert_eq!(
            cfg.translate.source_lang,
            TranslateConfig::default().source_lang,
            "拼错的字段被丢弃，值回落到默认——这正是要小心的地方"
        );
    }

    #[test]
    fn endpoint_clamps_out_of_range() {
        let e = EndpointConfig {
            trailing_silence_ms: 0,
            max_utterance_ms: 999_999,
        }
        .sanitized();

        assert!(
            e.trailing_silence_ms >= MIN_TRAILING_SILENCE_MS,
            "rule2 为 0 会让端点检测形同虚设——能出字但永不切分，看起来像成功"
        );
        assert!(e.max_utterance_ms <= MAX_MAX_UTTERANCE_MS);
    }

    #[test]
    fn endpoint_never_lets_max_utterance_fall_below_trailing_silence() {
        let e = EndpointConfig {
            trailing_silence_ms: 1_500,
            max_utterance_ms: 400,
        }
        .sanitized();

        assert!(
            e.max_utterance_ms >= e.trailing_silence_ms,
            "最长句 {} 短于静音阈值 {}——后者永远没机会触发",
            e.max_utterance_ms,
            e.trailing_silence_ms
        );
    }

    #[test]
    fn endpoint_keeps_legal_values_untouched() {
        let e = EndpointConfig {
            trailing_silence_ms: 600,
            max_utterance_ms: 12_000,
        }
        .sanitized();
        assert_eq!(e.trailing_silence_ms, 600);
        assert_eq!(e.max_utterance_ms, 12_000);
    }

    #[test]
    fn patch_leaves_absent_fields_alone() {
        let mut p = Preferences {
            overlay: OverlayConfig {
                font_size: 40,
                ..Default::default()
            },
            source_lang: "zh".into(),
            ..Default::default()
        };

        PrefsPatch {
            audio_source: Some("mic".into()),
            ..Default::default()
        }
        .apply_to(&mut p);

        assert_eq!(p.audio_source, "mic", "给的字段要生效");
        assert_eq!(p.overlay.font_size, 40, "没给的字段必须原样保留");
        assert_eq!(p.source_lang, "zh", "没给的字段必须原样保留");
    }

    #[test]
    fn analytics_defaults_on_and_patch_can_disable_it() {
        assert!(Preferences::default().analytics_enabled);

        let mut p = Preferences::default();
        PrefsPatch {
            analytics_enabled: Some(false),
            ..Default::default()
        }
        .apply_to(&mut p);
        assert!(!p.analytics_enabled);
    }

    #[test]
    fn patch_rejects_unknown_key() {
        let r = serde_json::from_value::<PrefsPatch>(serde_json::json!({ "audioSorce": "mic" }));
        assert!(r.is_err(), "未知键必须报错，不能静默忽略");
    }

    #[test]
    fn patch_validation_rejects_bad_values() {
        assert!(PrefsPatch {
            audio_source: Some("speaker".into()),
            ..Default::default()
        }
        .validate()
        .is_err());

        assert!(PrefsPatch {
            source_lang: Some("auto".into()),
            ..Default::default()
        }
        .validate()
        .is_err());

        assert!(PrefsPatch {
            audio_source: Some("loopback".into()),
            source_lang: Some("zh".into()),
            ..Default::default()
        }
        .validate()
        .is_ok());
    }

    #[test]
    fn patch_roundtrips_in_camel_case() {
        let patch = PrefsPatch {
            translate_engine: Some(TranslateEngine::Off),
            ..Default::default()
        };
        let json = serde_json::to_string(&patch).unwrap();
        assert!(json.contains("\"translateEngine\""), "{json}");
        assert_eq!(serde_json::from_str::<PrefsPatch>(&json).unwrap(), patch);
    }

    #[test]
    fn theme_serializes_camel_case() {
        assert_eq!(
            serde_json::to_string(&Theme::System).unwrap(),
            r#""system""#
        );
        assert_eq!(serde_json::to_string(&Theme::Dark).unwrap(), r#""dark""#);
    }

    #[test]
    fn config_module_stays_free_of_tauri() {
        let src = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/config/mod.rs"),
        )
        .expect("读不到 config/mod.rs");

        for (i, line) in src.lines().enumerate() {
            let t = line.trim_start();
            assert!(
                !t.starts_with("use tauri"),
                "config/mod.rs:{} 引入了 tauri：{line}\n\
                 → 配置类型必须保持无 Tauri 依赖。需要 AppHandle 的 IO \
                 请放进 prefs / paths 这类模块。",
                i + 1
            );
        }
    }
}
