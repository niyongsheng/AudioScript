use std::backtrace::Backtrace;
use std::panic::PanicHookInfo;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, OnceLock};
use std::time::{Duration, Instant};

use serde_json::Value;
use tauri::{AppHandle, Manager};
use tauri_plugin_aptabase::{ErrorReport, ErrorTracker, EventTracker};

pub const APP_KEY: &str = "A-US-2179981546";

const PANIC_FLUSH_TIMEOUT: Duration = Duration::from_secs(3);

static STARTED_AT: OnceLock<Instant> = OnceLock::new();
static EXIT_REPORTED: AtomicBool = AtomicBool::new(false);
static PANIC_REPORTING: AtomicBool = AtomicBool::new(false);

pub fn track(app: &AppHandle, event: &str, props: Option<Value>) {
    if !crate::prefs::load(app).analytics_enabled {
        return;
    }
    if let Err(e) = app.track_event(event, props) {
        tracing::debug!(event, "统计事件入队失败：{e}");
    }
}

pub fn mark_start() {
    let _ = STARTED_AT.set(Instant::now());
}

// 不用插件的 with_crash_reporting()：它没有开关，会在用户关掉匿名统计后继续上报错误。
pub fn report_panic(app: &AppHandle, info: &PanicHookInfo<'_>) {
    if PANIC_REPORTING.swap(true, Ordering::SeqCst) {
        return;
    }
    let _reset = ResetOnDrop;
    if !crate::prefs::load(app).analytics_enabled {
        return;
    }

    let thread = std::thread::current();
    let location = info
        .location()
        .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()));
    let payload = panic_payload(info);
    let message = match &location {
        Some(at) => format!("{payload} (at {at})"),
        None => payload,
    };
    let stack_trace = format!(
        "thread '{}' panicked at {}\n{}",
        thread.name().unwrap_or("<unnamed>"),
        location.as_deref().unwrap_or("an unknown location"),
        Backtrace::force_capture()
    );
    let fatal = thread.name() == Some("main") || cfg!(panic = "abort");
    app.track_error_report(
        ErrorReport::new("panic", message)
            .with_stack_trace(stack_trace)
            .with_fatal(fatal),
    );
    if fatal {
        flush_within(app, PANIC_FLUSH_TIMEOUT);
    }
}

struct ResetOnDrop;

impl Drop for ResetOnDrop {
    fn drop(&mut self) {
        PANIC_REPORTING.store(false, Ordering::SeqCst);
    }
}

fn panic_payload(info: &PanicHookInfo<'_>) -> String {
    payload_text(info.payload())
}

fn payload_text(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        return (*s).to_owned();
    }
    if let Some(s) = payload.downcast_ref::<String>() {
        return s.clone();
    }
    format!("{payload:?}")
}

fn flush_within(app: &AppHandle, timeout: Duration) {
    let handle = app.clone();
    let (done, wait) = mpsc::channel();
    let spawned = std::thread::Builder::new()
        .name("aptabase-flush".into())
        .spawn(move || {
            let _ = done.send(handle.flush_events_blocking());
        });
    if spawned.is_ok() {
        let _ = wait.recv_timeout(timeout);
    }
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
    track(
        app,
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
    fn panic_payload_reads_str_and_string() {
        assert_eq!(payload_text(&"boom"), "boom");
        assert_eq!(payload_text(&"boom".to_string()), "boom");
        assert_eq!(payload_text(&7u32), "Any { .. }");
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
