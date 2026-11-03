use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

use serde_json::Value;
use tauri::{AppHandle, Manager};
use tauri_plugin_aptabase::EventTracker;

pub const APP_KEY: &str = "A-US-2179981546";

static STARTED_AT: OnceLock<Instant> = OnceLock::new();
static EXIT_REPORTED: AtomicBool = AtomicBool::new(false);

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
