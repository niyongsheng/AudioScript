use serde::Deserialize;
use typeshare::typeshare;

use super::segment::SegmentBuilder;
use super::{AsrEngine, AsrError, EngineCaps};
use crate::config::AsrConfig;
use crate::transcript::TranscriptSegment;

pub const AVAILABLE: i32 = 1;

pub fn available() -> i32 {
    backend::available()
}

pub fn authorization() -> i32 {
    backend::authorization()
}

pub fn request_authorization(timeout_ms: i32) -> i32 {
    backend::request_authorization(timeout_ms)
}

#[typeshare]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MacSpeechStatus {
    pub authorization: String,
    pub installed_locales: Vec<String>,
}

pub fn status() -> MacSpeechStatus {
    let supported = available() == AVAILABLE;
    MacSpeechStatus {
        authorization: authorization_name(if supported { authorization() } else { -1 }),
        installed_locales: if supported {
            backend::installed_locales()
        } else {
            Vec::new()
        },
    }
}

fn authorization_name(code: i32) -> String {
    match code {
        0 => "notDetermined",
        1 => "denied",
        2 => "restricted",
        3 => "authorized",
        _ => "unsupported",
    }
    .into()
}

#[derive(Debug, Deserialize)]
struct BridgeResult {
    i: i64,
    t: String,
    f: bool,
    s: u64,
    e: u64,
}

fn parse_results(json: &str) -> Result<Vec<BridgeResult>, AsrError> {
    serde_json::from_str::<Vec<BridgeResult>>(json)
        .map_err(|e| AsrError::MacSpeechFailed(e.to_string()))
}

fn apply_results(
    builder: &mut SegmentBuilder,
    results: Vec<BridgeResult>,
) -> Vec<TranscriptSegment> {
    let mut out = Vec::with_capacity(results.len());
    for item in results {
        let text = item.t.trim();
        if text.is_empty() {
            continue;
        }
        out.push(builder.build(item.i, text, item.s, item.e, item.f));
        if item.f {
            builder.forget(item.i);
        }
    }
    out
}

pub struct MacNativeAdapter {
    backend: backend::Backend,
    builder: SegmentBuilder,
}

impl Default for MacNativeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl MacNativeAdapter {
    pub fn new() -> Self {
        Self {
            backend: backend::Backend::default(),
            builder: SegmentBuilder::new(),
        }
    }
}

impl AsrEngine for MacNativeAdapter {
    fn init(&mut self, cfg: &AsrConfig) -> Result<(), AsrError> {
        let code = backend::available();
        if code == -2 {
            return Err(AsrError::MacSpeechUsageDescription);
        }
        if code != AVAILABLE {
            return Err(AsrError::MacSpeechUnsupported);
        }

        let mut auth = backend::authorization();
        if auth == 0 {
            auth = backend::request_authorization(20_000);
        }
        if auth != 3 {
            return Err(AsrError::MacSpeechPermission);
        }

        let language = cfg.source_lang.trim();
        if language.is_empty() || language == "auto" {
            return Err(AsrError::MacSpeechLocaleUnavailable(language.to_owned()));
        }

        self.backend
            .create(language)
            .map_err(|(kind, detail)| match kind {
                1 => AsrError::MacSpeechUnsupported,
                2 => AsrError::MacSpeechUsageDescription,
                3 => AsrError::MacSpeechLocaleUnavailable(detail),
                4 => AsrError::MacSpeechAssetsMissing(detail),
                _ => AsrError::MacSpeechFailed(detail),
            })?;
        self.builder = SegmentBuilder::new();
        Ok(())
    }

    fn capabilities(&self) -> EngineCaps {
        EngineCaps::MAC_NATIVE
    }

    fn feed_pcm(&mut self, pcm: &[f32]) -> Result<Vec<TranscriptSegment>, AsrError> {
        if !self.backend.is_open() {
            return Err(AsrError::NotInitialized);
        }
        self.backend.feed(pcm)?;
        let json = self.backend.poll()?;
        if json.is_empty() {
            return Ok(Vec::new());
        }
        Ok(apply_results(&mut self.builder, parse_results(&json)?))
    }

    fn flush(&mut self) -> Result<Vec<TranscriptSegment>, AsrError> {
        if !self.backend.is_open() {
            return Ok(Vec::new());
        }
        let json = self.backend.flush(20_000)?;
        if json.is_empty() {
            return Ok(Vec::new());
        }
        Ok(apply_results(&mut self.builder, parse_results(&json)?))
    }

    fn shutdown(&mut self) {
        self.backend.shutdown();
    }
}

#[cfg(all(target_os = "macos", mac_speech))]
mod backend {
    use std::ffi::{c_char, c_void, CStr, CString};
    use std::ptr;

    use super::super::AsrError;

    extern "C" {
        fn audioscript_speech_available() -> i32;
        fn audioscript_speech_authorization() -> i32;
        fn audioscript_speech_request_authorization(timeout_ms: i32) -> i32;
        fn audioscript_speech_locales(installed_only: i32) -> *mut c_char;
        fn audioscript_speech_create(
            language: *const c_char,
            error_out: *mut *mut c_char,
            kind_out: *mut i32,
        ) -> *mut c_void;
        fn audioscript_speech_feed(
            handle: *mut c_void,
            samples: *const f32,
            count: i32,
            error_out: *mut *mut c_char,
        ) -> i32;
        fn audioscript_speech_poll(
            handle: *mut c_void,
            out_json: *mut *mut c_char,
            error_out: *mut *mut c_char,
        ) -> i32;
        fn audioscript_speech_flush(
            handle: *mut c_void,
            timeout_ms: i32,
            out_json: *mut *mut c_char,
            error_out: *mut *mut c_char,
        ) -> i32;
        fn audioscript_speech_destroy(handle: *mut c_void);
        fn audioscript_speech_free(pointer: *mut c_char);
    }

    fn take_string(pointer: *mut c_char) -> Option<String> {
        if pointer.is_null() {
            return None;
        }
        let value = unsafe { CStr::from_ptr(pointer) }
            .to_string_lossy()
            .into_owned();
        unsafe { audioscript_speech_free(pointer) };
        Some(value)
    }

    fn take_error(pointer: *mut c_char) -> String {
        take_string(pointer).unwrap_or_else(|| "unknown error".into())
    }

    pub fn available() -> i32 {
        unsafe { audioscript_speech_available() }
    }

    pub fn authorization() -> i32 {
        unsafe { audioscript_speech_authorization() }
    }

    pub fn request_authorization(timeout_ms: i32) -> i32 {
        unsafe { audioscript_speech_request_authorization(timeout_ms) }
    }

    pub fn installed_locales() -> Vec<String> {
        let pointer = unsafe { audioscript_speech_locales(1) };
        let Some(json) = take_string(pointer) else {
            return Vec::new();
        };
        serde_json::from_str(&json).unwrap_or_default()
    }

    struct Handle(*mut c_void);
    unsafe impl Send for Handle {}

    impl Drop for Handle {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe { audioscript_speech_destroy(self.0) };
                self.0 = ptr::null_mut();
            }
        }
    }

    #[derive(Default)]
    pub struct Backend {
        handle: Option<Handle>,
    }

    impl Backend {
        pub fn is_open(&self) -> bool {
            self.handle.is_some()
        }

        pub fn create(&mut self, language: &str) -> Result<(), (i32, String)> {
            self.shutdown();
            let language = CString::new(language).map_err(|e| (5, e.to_string()))?;
            let mut error: *mut c_char = ptr::null_mut();
            let mut kind: i32 = 5;
            let pointer =
                unsafe { audioscript_speech_create(language.as_ptr(), &mut error, &mut kind) };
            if pointer.is_null() {
                return Err((kind, take_error(error)));
            }
            self.handle = Some(Handle(pointer));
            Ok(())
        }

        pub fn feed(&mut self, pcm: &[f32]) -> Result<(), AsrError> {
            let Some(handle) = &self.handle else {
                return Err(AsrError::NotInitialized);
            };
            if pcm.is_empty() {
                return Ok(());
            }
            let mut error: *mut c_char = ptr::null_mut();
            let code = unsafe {
                audioscript_speech_feed(
                    handle.0,
                    pcm.as_ptr(),
                    pcm.len().min(i32::MAX as usize) as i32,
                    &mut error,
                )
            };
            if code != 0 {
                return Err(AsrError::MacSpeechFailed(take_error(error)));
            }
            Ok(())
        }

        pub fn poll(&mut self) -> Result<String, AsrError> {
            let Some(handle) = &self.handle else {
                return Err(AsrError::NotInitialized);
            };
            let mut json: *mut c_char = ptr::null_mut();
            let mut error: *mut c_char = ptr::null_mut();
            let code = unsafe { audioscript_speech_poll(handle.0, &mut json, &mut error) };
            if code != 0 {
                return Err(AsrError::MacSpeechFailed(take_error(error)));
            }
            Ok(take_string(json).unwrap_or_default())
        }

        pub fn flush(&mut self, timeout_ms: i32) -> Result<String, AsrError> {
            let Some(handle) = &self.handle else {
                return Ok("[]".into());
            };
            let mut json: *mut c_char = ptr::null_mut();
            let mut error: *mut c_char = ptr::null_mut();
            let code =
                unsafe { audioscript_speech_flush(handle.0, timeout_ms, &mut json, &mut error) };
            if code != 0 {
                return Err(AsrError::MacSpeechFailed(take_error(error)));
            }
            Ok(take_string(json).unwrap_or_default())
        }

        pub fn shutdown(&mut self) {
            self.handle = None;
        }
    }
}

#[cfg(not(all(target_os = "macos", mac_speech)))]
mod backend {
    use super::super::AsrError;

    pub fn available() -> i32 {
        0
    }

    pub fn authorization() -> i32 {
        -1
    }

    pub fn request_authorization(_timeout_ms: i32) -> i32 {
        -1
    }

    pub fn installed_locales() -> Vec<String> {
        Vec::new()
    }

    #[derive(Default)]
    pub struct Backend;

    impl Backend {
        pub fn is_open(&self) -> bool {
            false
        }

        pub fn create(&mut self, _language: &str) -> Result<(), (i32, String)> {
            Err((1, "unsupported".into()))
        }

        pub fn feed(&mut self, _pcm: &[f32]) -> Result<(), AsrError> {
            Err(AsrError::MacSpeechUnsupported)
        }

        pub fn poll(&mut self) -> Result<String, AsrError> {
            Err(AsrError::MacSpeechUnsupported)
        }

        pub fn flush(&mut self, _timeout_ms: i32) -> Result<String, AsrError> {
            Err(AsrError::MacSpeechUnsupported)
        }

        pub fn shutdown(&mut self) {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(i: i64, text: &str, f: bool, s: u64, e: u64) -> BridgeResult {
        BridgeResult {
            i,
            t: text.into(),
            f,
            s,
            e,
        }
    }

    #[test]
    fn partial_then_final_share_the_same_id() {
        let mut builder = SegmentBuilder::new();
        let partial = apply_results(&mut builder, vec![result(0, "你好", false, 0, 500)]);
        let final_seg = apply_results(&mut builder, vec![result(0, "你好世界", true, 0, 900)]);

        assert_eq!(partial[0].id, final_seg[0].id);
        assert_eq!(partial[0].state, crate::transcript::SegmentState::Partial);
        assert_eq!(final_seg[0].state, crate::transcript::SegmentState::Final);
        assert_eq!(final_seg[0].revision, 2);
    }

    #[test]
    fn empty_results_are_dropped() {
        let mut builder = SegmentBuilder::new();
        assert!(apply_results(&mut builder, vec![result(0, "   ", true, 0, 10)]).is_empty());
    }

    #[test]
    fn distinct_indices_get_distinct_ids() {
        let mut builder = SegmentBuilder::new();
        let segments = apply_results(
            &mut builder,
            vec![
                result(0, "第一句", true, 0, 900),
                result(1, "第二句", true, 900, 1800),
            ],
        );
        assert_ne!(segments[0].id, segments[1].id);
    }

    #[test]
    fn parse_rejects_malformed_payload() {
        assert!(parse_results("not json").is_err());
        assert!(parse_results(r#"[{"i":0}]"#).is_err());
    }

    #[test]
    fn status_reports_unsupported_on_this_build() {
        let status = status();
        if available() != AVAILABLE {
            assert_eq!(status.authorization, "unsupported");
            assert!(status.installed_locales.is_empty());
        }
    }
}
