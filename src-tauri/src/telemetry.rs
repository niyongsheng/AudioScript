use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use sentry::protocol::{Event, Exception, Frame, Stacktrace, Values};
use sentry::Level;
use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, Manager};
use tauri_plugin_aptabase::EventTracker;

use crate::config::AnalyticsTier;

pub const APP_KEY: &str = "A-US-2179981546";

pub const SENTRY_DSN: &str = "https://1cf246b882cdd2d53f589b6aab5a3f78@o4512410989428736.ingest.us.sentry.io/4512411088977920";

pub const ANALYTICS_TIER: AnalyticsTier = AnalyticsTier::Minimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Detail {
    Launch,
    Core,
    Standard,
    Full,
}

static STARTED_AT: OnceLock<Instant> = OnceLock::new();
static EXIT_REPORTED: AtomicBool = AtomicBool::new(false);
static SENT: AtomicUsize = AtomicUsize::new(0);

pub struct SentryGuard(sentry::ClientInitGuard);

pub fn init_error_reporting(app: &AppHandle) -> SentryGuard {
    let gate = app.clone();
    let options = sentry::ClientOptions::new()
        .release(format!("audioscript@{}", env!("CARGO_PKG_VERSION")))
        .environment(if cfg!(debug_assertions) {
            "development"
        } else {
            "production"
        })
        .send_default_pii(false)
        .attach_stacktrace(true)
        .shutdown_timeout(Duration::from_secs(3))
        .before_send(move |event| crate::prefs::load(&gate).analytics_enabled.then_some(event));
    SentryGuard(sentry::init((SENTRY_DSN, options)))
}

pub fn flush_error_reporting(app: &AppHandle) {
    if let Some(guard) = app.try_state::<SentryGuard>() {
        guard.0.flush(Some(Duration::from_secs(3)));
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendError {
    pub kind: String,
    pub name: String,
    pub message: String,
    #[serde(default)]
    pub stack: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub line: Option<u64>,
    #[serde(default)]
    pub column: Option<u64>,
    #[serde(default)]
    pub window: Option<String>,
}

pub fn capture_frontend_error(app: &AppHandle, report: FrontendError) {
    if !crate::prefs::load(app).analytics_enabled {
        return;
    }

    let message = truncate(report.message.trim(), 2_000);
    let name = truncate(report.name.trim(), 120);
    let stacktrace = report
        .stack
        .as_deref()
        .and_then(parse_js_stack)
        .or_else(|| single_frame(report.source.as_deref(), report.line, report.column));

    let mut event = Event::new();
    event.level = Level::Error;
    event.platform = "javascript".into();
    event.tags.insert("source".into(), "webview".into());
    event.tags.insert(
        "kind".into(),
        match report.kind.as_str() {
            "unhandledrejection" => "unhandledrejection".into(),
            _ => "error".into(),
        },
    );
    if let Some(window) = report
        .window
        .as_deref()
        .filter(|w| *w == "main" || *w == "overlay")
    {
        event.tags.insert("window".into(), window.to_owned());
    }
    event.exception = Values::from(vec![Exception {
        ty: if name.is_empty() {
            "Error".into()
        } else {
            name
        },
        value: (!message.is_empty()).then_some(message),
        stacktrace,
        ..Default::default()
    }]);
    sentry::capture_event(event);
}

fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

fn parse_js_stack(stack: &str) -> Option<Stacktrace> {
    let frames: Vec<Frame> = stack
        .lines()
        .filter_map(|line| parse_frame(line.trim()))
        .collect();
    Stacktrace::from_frames_reversed(frames)
}

fn parse_frame(line: &str) -> Option<Frame> {
    let (function, location) = if let Some(rest) = line.strip_prefix("at ") {
        let rest = rest.strip_prefix("async ").unwrap_or(rest).trim();
        match rest.find(" (") {
            Some(at) if rest.ends_with(')') => (Some(&rest[..at]), &rest[at + 2..rest.len() - 1]),
            _ => (None, rest),
        }
    } else {
        let (function, location) = line.rsplit_once('@')?;
        (Some(function.trim()), location)
    };

    let mut parts = location.rsplitn(3, ':');
    let colno = parts.next()?.parse::<u64>().ok()?;
    let lineno = parts.next()?.parse::<u64>().ok()?;
    let file = parts.next()?.trim();
    if file.is_empty() {
        return None;
    }

    Some(Frame {
        function: function
            .map(|f| f.trim().to_owned())
            .filter(|f| !f.is_empty()),
        filename: basename(file),
        abs_path: Some(file.to_owned()),
        lineno: Some(lineno),
        colno: Some(colno),
        in_app: Some(is_app_frame(file)),
        ..Default::default()
    })
}

fn single_frame(
    source: Option<&str>,
    line: Option<u64>,
    column: Option<u64>,
) -> Option<Stacktrace> {
    let source = source?;
    Some(Stacktrace {
        frames: vec![Frame {
            filename: basename(source),
            abs_path: Some(source.to_owned()),
            lineno: line,
            colno: column,
            in_app: Some(true),
            ..Default::default()
        }],
        ..Default::default()
    })
}

fn basename(path: &str) -> Option<String> {
    path.rsplit(|c| c == '/' || c == '\\')
        .next()
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
}

fn is_app_frame(file: &str) -> bool {
    file.contains("/assets/")
        || file.contains("/src/")
        || file.starts_with("tauri://")
        || file.starts_with("http://tauri.localhost")
}

pub fn track(app: &AppHandle, event: &str, props: Option<Value>) {
    track_at(app, Detail::Core, event, props);
}

pub fn track_at(app: &AppHandle, detail: Detail, event: &str, props: Option<Value>) {
    if !crate::prefs::load(app).analytics_enabled || !allows(ANALYTICS_TIER, detail) {
        return;
    }
    if SENT.fetch_add(1, Ordering::Relaxed) >= budget(ANALYTICS_TIER) {
        return;
    }
    if let Err(e) = app.track_event(event, props) {
        tracing::debug!(event, "统计事件入队失败：{e}");
    }
}

fn allows(tier: AnalyticsTier, detail: Detail) -> bool {
    match tier {
        AnalyticsTier::UltraMinimal => detail <= Detail::Launch,
        AnalyticsTier::Minimal => detail <= Detail::Core,
        AnalyticsTier::Standard => detail <= Detail::Standard,
        AnalyticsTier::Full => true,
    }
}

fn budget(tier: AnalyticsTier) -> usize {
    match tier {
        AnalyticsTier::UltraMinimal => 4,
        AnalyticsTier::Minimal => 30,
        AnalyticsTier::Standard => 150,
        AnalyticsTier::Full => 1_000,
    }
}

pub fn mark_start() {
    let _ = STARTED_AT.set(Instant::now());
}

pub fn report_exit(app: &AppHandle, reason: &'static str) {
    if EXIT_REPORTED.swap(true, Ordering::SeqCst) {
        return;
    }
    let uptime_ms = STARTED_AT
        .get()
        .map(|t| t.elapsed().as_millis() as u64)
        .unwrap_or(0);
    let capturing = app
        .try_state::<crate::session::SessionState>()
        .is_some_and(|s| s.is_running());
    track_at(
        app,
        Detail::Launch,
        "app_exited",
        Some(serde_json::json!({
            "reason": reason,
            "uptime_bucket": duration_bucket(uptime_ms),
            "capturing": if capturing { 1 } else { 0 },
        })),
    );
    app.flush_events_blocking();
}

pub fn duration_bucket(ms: u64) -> &'static str {
    match ms {
        0..=9_999 => "<10s",
        10_000..=59_999 => "<1m",
        60_000..=299_999 => "<5m",
        300_000..=899_999 => "<15m",
        900_000..=3_599_999 => "<60m",
        _ => ">=60m",
    }
}

pub fn first_text_bucket(ms: u64) -> &'static str {
    match ms {
        0..=500 => "<=0.5s",
        501..=1_000 => "<=1s",
        1_001..=2_000 => "<=2s",
        2_001..=5_000 => "<=5s",
        5_001..=10_000 => "<=10s",
        _ => ">10s",
    }
}

pub fn sample_rate_bucket(hz: u32) -> &'static str {
    match hz {
        0 => "unknown",
        1..=16_000 => "<=16k",
        16_001..=32_000 => "<=32k",
        32_001..=44_100 => "44.1k",
        44_101..=48_000 => "48k",
        _ => ">48k",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentry_dsn_parses() {
        assert!(SENTRY_DSN.parse::<sentry::types::Dsn>().is_ok());
    }

    #[test]
    fn js_stack_frames_are_reversed_and_parsed() {
        let stack = parse_js_stack(
            "TypeError: boom\n    at handle (http://localhost/assets/index.js:12:34)\n    at http://localhost/assets/index.js:56:78\n    at async run (http://localhost/src/main.tsx:9:10)",
        )
        .expect("应解析出堆栈");

        assert_eq!(stack.frames.len(), 3);
        assert_eq!(
            stack.frames[0].function.as_deref(),
            Some("run"),
            "最外层帧应排在最前"
        );
        assert_eq!(stack.frames[2].function.as_deref(), Some("handle"));
        assert_eq!(stack.frames[2].lineno, Some(12));
        assert_eq!(stack.frames[2].colno, Some(34));
        assert_eq!(
            stack.frames[1].abs_path.as_deref(),
            Some("http://localhost/assets/index.js")
        );
        assert_eq!(stack.frames[0].in_app, Some(true));
    }

    #[test]
    fn js_stack_parses_jsc_format() {
        let stack = parse_js_stack(
            "TypeError: boom\nhandle@http://localhost/assets/index.js:12:34\nrun@http://localhost/src/main.tsx:9:10",
        )
        .expect("JSC 堆栈应可解析");

        assert_eq!(stack.frames.len(), 2);
        assert_eq!(stack.frames[0].function.as_deref(), Some("run"));
        assert_eq!(stack.frames[1].function.as_deref(), Some("handle"));
        assert_eq!(stack.frames[1].lineno, Some(12));
        assert_eq!(stack.frames[1].colno, Some(34));
    }

    #[test]
    fn junk_stack_lines_are_ignored() {
        assert!(parse_js_stack("Error: boom\n    at native\n    at <anonymous>").is_none());
    }

    #[test]
    fn tiers_only_widen_with_detail() {
        assert!(allows(AnalyticsTier::UltraMinimal, Detail::Launch));
        assert!(!allows(AnalyticsTier::UltraMinimal, Detail::Core));
        assert!(allows(AnalyticsTier::Minimal, Detail::Launch));
        assert!(allows(AnalyticsTier::Minimal, Detail::Core));
        assert!(!allows(AnalyticsTier::Minimal, Detail::Standard));
        assert!(!allows(AnalyticsTier::Minimal, Detail::Full));
        assert!(allows(AnalyticsTier::Standard, Detail::Core));
        assert!(allows(AnalyticsTier::Standard, Detail::Standard));
        assert!(!allows(AnalyticsTier::Standard, Detail::Full));
        assert!(allows(AnalyticsTier::Full, Detail::Full));
    }

    #[test]
    fn duration_buckets_cover_boundaries() {
        assert_eq!(duration_bucket(0), "<10s");
        assert_eq!(duration_bucket(9_999), "<10s");
        assert_eq!(duration_bucket(10_000), "<1m");
        assert_eq!(duration_bucket(59_999), "<1m");
        assert_eq!(duration_bucket(60_000), "<5m");
        assert_eq!(duration_bucket(3_599_999), "<60m");
        assert_eq!(duration_bucket(3_600_000), ">=60m");
    }

    #[test]
    fn first_text_buckets_cover_boundaries() {
        assert_eq!(first_text_bucket(0), "<=0.5s");
        assert_eq!(first_text_bucket(500), "<=0.5s");
        assert_eq!(first_text_bucket(501), "<=1s");
        assert_eq!(first_text_bucket(1_000), "<=1s");
        assert_eq!(first_text_bucket(2_001), "<=5s");
        assert_eq!(first_text_bucket(10_001), ">10s");
    }

    #[test]
    fn sample_rate_buckets_cover_common_rates() {
        assert_eq!(sample_rate_bucket(0), "unknown");
        assert_eq!(sample_rate_bucket(16_000), "<=16k");
        assert_eq!(sample_rate_bucket(44_100), "44.1k");
        assert_eq!(sample_rate_bucket(48_000), "48k");
        assert_eq!(sample_rate_bucket(96_000), ">48k");
    }
}
