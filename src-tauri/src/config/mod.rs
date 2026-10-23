use serde::{Deserialize, Serialize};
use typeshare::typeshare;

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
    RemoteApi,
    Mock,
}

impl TranslateEngine {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::LocalNmt => "local-nmt",
            Self::RemoteApi => "remote-api",
            Self::Mock => "mock",
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
}

impl Default for TranslateConfig {
    fn default() -> Self {
        Self {
            engine: TranslateEngine::default(),
            source_lang: "en".into(),
            target_lang: "zh".into(),
            zh_script: ZhScript::default(),
            model_dir: None,
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
#[serde(rename_all = "camelCase")]
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
            glow: true,
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
}

impl AsrModelTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Builtin => "builtin",
            Self::HighAccuracy => "high-accuracy",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "builtin" => Some(Self::Builtin),
            "high-accuracy" => Some(Self::HighAccuracy),
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
    Big,
}

impl MtModelTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Base => "base",
            Self::Big => "big",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "base" => Some(Self::Base),
            "big" => Some(Self::Big),
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
    pub translate_engine: TranslateEngine,
    pub source_lang: String,
    pub zh_script: ZhScript,
    pub overlay: OverlayConfig,
    pub theme: Theme,
    pub sidebar_visible: bool,
    pub shortcuts: std::collections::HashMap<String, String>,
    pub endpoint: EndpointConfig,
    pub overlay_geometry: Option<WindowGeometry>,
    pub onboarding_done: bool,
    pub asr_model_tier: AsrModelTier,
    pub mt_model_tier: MtModelTier,
    pub analytics_enabled: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            audio_source: "loopback".into(),
            device_id: None,
            translate_engine: TranslateEngine::default(),
            source_lang: "en".into(),
            zh_script: ZhScript::default(),
            overlay: OverlayConfig::default(),
            theme: Theme::default(),
            sidebar_visible: true,
            endpoint: EndpointConfig::default(),
            shortcuts: std::collections::HashMap::new(),
            overlay_geometry: None,
            onboarding_done: false,
            asr_model_tier: AsrModelTier::default(),
            mt_model_tier: MtModelTier::default(),
            analytics_enabled: false,
        }
    }
}

#[typeshare]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PrefsPatch {
    pub audio_source: Option<String>,
    pub device_id: Option<String>,
    pub translate_engine: Option<TranslateEngine>,
    pub source_lang: Option<String>,
    pub zh_script: Option<ZhScript>,
    pub endpoint: Option<EndpointConfig>,
    pub theme: Option<Theme>,
    pub sidebar_visible: Option<bool>,
    pub onboarding_done: Option<bool>,
    pub asr_model_tier: Option<AsrModelTier>,
    pub mt_model_tier: Option<MtModelTier>,
    pub analytics_enabled: Option<bool>,
}

impl PrefsPatch {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(s) = &self.audio_source {
            if !matches!(s.as_str(), "mic" | "loopback" | "mix") {
                return Err(format!("未知音源 {s}"));
            }
        }
        if let Some(s) = &self.source_lang {
            if !matches!(s.as_str(), "en" | "zh") {
                return Err(format!("未知源语言 {s}（auto 尚未实现）"));
            }
        }
        Ok(())
    }

    pub fn apply_to(self, p: &mut Preferences) {
        if let Some(v) = self.audio_source {
            p.audio_source = v;
        }
        if let Some(v) = self.device_id {
            p.device_id = Some(v);
        }
        if let Some(v) = self.translate_engine {
            p.translate_engine = v;
        }
        if let Some(v) = self.source_lang {
            p.source_lang = v;
        }
        if let Some(v) = self.zh_script {
            p.zh_script = v;
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
        if let Some(v) = self.mt_model_tier {
            p.mt_model_tier = v;
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
        if !matches!(self.source_lang.as_str(), "en" | "zh") {
            self.source_lang = d.source_lang;
        }
        self.device_id = self.device_id.filter(|s| !s.is_empty());

        self.endpoint = self.endpoint.sanitized();

        if let Some(g) = &mut self.overlay_geometry {
            g.size.width = g.size.width.clamp(MIN_OVERLAY_WIDTH, MAX_OVERLAY_WIDTH);
            g.size.height = g.size.height.clamp(MIN_OVERLAY_HEIGHT, MAX_OVERLAY_HEIGHT);
        }

        self
    }

    pub fn apply_session(&mut self, source: &str, device: Option<String>) {
        self.audio_source = source.to_owned();
        self.device_id = device.filter(|s| !s.is_empty());
    }

    pub fn target_lang(&self) -> &'static str {
        target_lang_for(&self.source_lang)
    }
}

pub fn direction_label(source_lang: &str, zh_script: ZhScript) -> &'static str {
    match (source_lang.starts_with("zh"), zh_script) {
        (true, ZhScript::Traditional) => "繁体中文 → 英语",
        (true, ZhScript::Simplified) => "简体中文 → 英语",
        (false, ZhScript::Traditional) => "英语 → 繁体中文",
        (false, ZhScript::Simplified) => "英语 → 简体中文",
    }
}

pub fn target_lang_for(source_lang: &str) -> &'static str {
    if source_lang == "zh" {
        "en"
    } else {
        "zh"
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
    fn direction_label_covers_all_four_directions() {
        assert_eq!(
            direction_label("en", ZhScript::Simplified),
            "英语 → 简体中文"
        );
        assert_eq!(
            direction_label("zh", ZhScript::Simplified),
            "简体中文 → 英语"
        );
        assert_eq!(
            direction_label("en", ZhScript::Traditional),
            "英语 → 繁体中文"
        );
        assert_eq!(
            direction_label("zh", ZhScript::Traditional),
            "繁体中文 → 英语"
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
    fn overlay_defaults_are_the_out_of_box_setup() {
        let c = OverlayConfig::default();
        assert!(c.enable, "出箱即显示悬浮窗");
        assert_eq!(c.mode, OverlayMode::Edit, "出箱即可拖动 / 缩放");
        assert!(c.glow, "发光边框默认开是产品定位，别改回去");
    }

    #[test]
    fn overlay_config_clamps_out_of_range_values() {
        let c = OverlayConfig {
            opacity: 250,
            font_size: 200,
            max_lines: 9999,
            ..Default::default()
        }
        .sanitized();

        assert_eq!(c.opacity, 100, "不透明度上限是 100");
        assert_eq!(c.font_size, MAX_FONT_SIZE);
        assert_eq!(c.max_lines, MAX_OVERLAY_LINES);
    }

    #[test]
    fn overlay_config_clamps_zero_that_would_look_like_a_bug() {
        let c = OverlayConfig {
            max_lines: 0,
            font_size: 0,
            ..Default::default()
        }
        .sanitized();
        assert_eq!(c.max_lines, 1);
        assert_eq!(c.font_size, MIN_FONT_SIZE);
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
    fn analytics_defaults_off_and_patch_can_change_it() {
        assert!(!Preferences::default().analytics_enabled);

        let mut p = Preferences::default();
        PrefsPatch {
            analytics_enabled: Some(true),
            ..Default::default()
        }
        .apply_to(&mut p);
        assert!(p.analytics_enabled);
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
