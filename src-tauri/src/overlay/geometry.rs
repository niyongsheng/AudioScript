use std::time::Duration;

use tauri::{AppHandle, Manager};

use super::{Frame, LABEL};
use crate::config::{WindowGeometry, WindowPos, WindowSize, MIN_OVERLAY_HEIGHT, MIN_OVERLAY_WIDTH};

const POLL: Duration = Duration::from_millis(2_000);

const MIN_GRAB_W: i32 = 120;

const MIN_GRAB_H: i32 = 60;

const EDGE_MARGIN: u32 = 40;

pub fn restore(app: &AppHandle, saved: Option<WindowGeometry>) {
    let Some(g) = saved else { return };

    if Frame::get(app).is_err() {
        tracing::warn!("悬浮窗不存在，跳过几何恢复");
        return;
    }

    let safe = clamp_to_visible(app, g);
    if safe != g {
        tracing::warn!(
            saved_x = g.position.x,
            saved_y = g.position.y,
            "记录的位置不在任何屏幕上（多半是拔了显示器），已搬回可见区域"
        );
    }

    if let Err(e) = super::set_geometry(app, &safe) {
        tracing::error!("{e}");
        return;
    }

    tracing::info!(
        x = safe.position.x,
        y = safe.position.y,
        w = safe.size.width,
        h = safe.size.height,
        "悬浮窗几何已恢复"
    );
}

pub fn start_watch(app: &AppHandle) {
    if app.get_webview_window(LABEL).is_none() {
        tracing::error!("找不到悬浮窗，位置与尺寸不会被记住");
        return;
    }

    log_monitors(app);

    let app = app.clone();
    let spawned = std::thread::Builder::new()
        .name("overlay-geometry".into())
        .spawn(move || {
            let mut last = super::geometry(&app);
            loop {
                std::thread::sleep(POLL);
                let Some(now) = super::geometry(&app) else {
                    continue;
                };
                if Some(now) == last {
                    continue;
                }
                last = Some(now);
                save(&app, now);
            }
        });

    if let Err(e) = spawned {
        tracing::error!("无法启动几何监视线程：{e}");
    }
}

fn log_monitors(app: &AppHandle) {
    let monitors = app.available_monitors().unwrap_or_default();
    let primary = app.primary_monitor().ok().flatten();

    let desc: Vec<String> = monitors
        .iter()
        .map(|m| {
            let p = m.position();
            let s = m.size();
            let is_primary = primary
                .as_ref()
                .is_some_and(|pm| pm.position() == p && pm.size() == s);
            format!(
                "{}x{}@{},{} scale={}{}",
                s.width,
                s.height,
                p.x,
                p.y,
                m.scale_factor(),
                if is_primary { " (主)" } else { "" }
            )
        })
        .collect();

    tracing::info!(count = monitors.len(), "显示器布局：{}", desc.join(" | "));
}

fn save(app: &AppHandle, g: WindowGeometry) {
    if let Err(e) = crate::prefs::update(app, |p| p.overlay_geometry = Some(g)) {
        tracing::error!("悬浮窗位置保存失败，下次启动不会记住它：{e}");
    } else {
        tracing::debug!(
            x = g.position.x,
            y = g.position.y,
            w = g.size.width,
            h = g.size.height,
            "悬浮窗几何已保存"
        );
    }
}

fn fit_size(size: WindowSize, monitor: (u32, u32)) -> WindowSize {
    let cap = |want: u32, screen: u32, floor: u32| {
        want.min(screen.saturating_sub(EDGE_MARGIN)).max(floor)
    };
    WindowSize {
        width: cap(size.width, monitor.0, MIN_OVERLAY_WIDTH),
        height: cap(size.height, monitor.1, MIN_OVERLAY_HEIGHT),
    }
}

fn clamp_to_visible(app: &AppHandle, g: WindowGeometry) -> WindowGeometry {
    let monitors = app.available_monitors().unwrap_or_default();

    let host = monitors.iter().find(|m| is_grabbable(&g, m));

    let primary;
    let monitor: &tauri::Monitor = match host {
        Some(m) => m,
        None => {
            let Ok(Some(p)) = app.primary_monitor() else {
                tracing::warn!("拿不到任何显示器信息，无法校验悬浮窗位置");
                return g;
            };
            primary = p;
            &primary
        }
    };

    let (mx, my, mw, mh) = logical_monitor_rect(monitor);
    let size = fit_size(g.size, (mw, mh));

    if host.is_some() {
        return WindowGeometry {
            position: g.position,
            size,
        };
    }

    let (w, h) = (size.width as i32, size.height as i32);
    let x = mx + (mw as i32 - w) / 2;
    let y = my + mh as i32 - h - (mh as i32 / 6);

    WindowGeometry {
        position: WindowPos { x, y },
        size,
    }
}

pub(crate) fn logical_monitor_rect(m: &tauri::Monitor) -> (i32, i32, u32, u32) {
    let scale = m.scale_factor();
    let p = m.position();
    let s = m.size();
    (
        (p.x as f64 / scale).round() as i32,
        (p.y as f64 / scale).round() as i32,
        (s.width as f64 / scale).round() as u32,
        (s.height as f64 / scale).round() as u32,
    )
}

fn is_grabbable(g: &WindowGeometry, m: &tauri::Monitor) -> bool {
    rect_is_grabbable(
        (g.position.x, g.position.y, g.size.width, g.size.height),
        logical_monitor_rect(m),
    )
}

fn rect_is_grabbable(w: (i32, i32, u32, u32), m: (i32, i32, u32, u32)) -> bool {
    let (wx0, wy0) = (w.0, w.1);
    let (wx1, wy1) = (wx0 + w.2 as i32, wy0 + w.3 as i32);
    let (mx0, my0) = (m.0, m.1);
    let (mx1, my1) = (mx0 + m.2 as i32, my0 + m.3 as i32);

    let overlap_w = wx1.min(mx1) - wx0.max(mx0);
    let overlap_h = wy1.min(my1) - wy0.max(my0);

    overlap_w >= MIN_GRAB_W.min(w.2 as i32) && overlap_h >= MIN_GRAB_H.min(w.3 as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRIMARY: (i32, i32, u32, u32) = (0, 0, 2560, 1440);

    fn win(x: i32, y: i32) -> (i32, i32, u32, u32) {
        (x, y, 520, 160)
    }

    #[test]
    fn window_inside_monitor_is_grabbable() {
        assert!(rect_is_grabbable(win(100, 100), PRIMARY));
    }

    #[test]
    fn window_on_a_removed_monitor_is_not_grabbable() {
        assert!(!rect_is_grabbable(win(-1200, 400), PRIMARY));
    }

    #[test]
    fn barely_peeking_window_is_not_grabbable() {
        assert!(!rect_is_grabbable(win(-490, 400), PRIMARY));
    }

    #[test]
    fn mostly_onscreen_edge_still_counts() {
        assert!(rect_is_grabbable(win(-200, 400), PRIMARY));
    }

    #[test]
    fn window_below_a_rearranged_display_is_not_grabbable() {
        let builtin = (-1680, 0, 1680, 1050);
        let saved = (-1440, 1428, 1386, 648);
        assert!(!rect_is_grabbable(saved, PRIMARY));
        assert!(!rect_is_grabbable(saved, builtin));
    }

    #[test]
    fn second_monitor_to_the_right_is_valid() {
        let secondary = (2560, 0, 1920, 1080);
        assert!(rect_is_grabbable(win(2600, 200), secondary));
        assert!(!rect_is_grabbable(win(2600, 200), PRIMARY));
    }

    #[test]
    fn window_taller_than_grab_threshold_still_needs_overlap() {
        assert!(!rect_is_grabbable(win(100, 1500), PRIMARY));
    }

    fn size(w: u32, h: u32) -> WindowSize {
        WindowSize {
            width: w,
            height: h,
        }
    }

    #[test]
    fn oversize_window_is_shrunk_to_fit_the_screen() {
        let fitted = fit_size(size(4096, 160), (2560, 1440));
        assert!(
            fitted.width <= 2560 - EDGE_MARGIN,
            "宽度必须留出边缘余量，实际 {}",
            fitted.width
        );
        assert_eq!(fitted.height, 160, "高度本来就装得下，不该被动");
    }

    #[test]
    fn window_that_already_fits_is_untouched() {
        let fitted = fit_size(size(600, 200), (2560, 1440));
        assert_eq!(fitted, size(600, 200));
    }

    #[test]
    fn absurdly_small_monitor_still_yields_usable_size() {
        let fitted = fit_size(size(520, 160), (30, 20));
        assert_eq!(fitted.width, MIN_OVERLAY_WIDTH);
        assert_eq!(fitted.height, MIN_OVERLAY_HEIGHT);
    }

    #[test]
    fn shrinking_respects_the_configured_minimum() {
        let fitted = fit_size(size(MIN_OVERLAY_WIDTH, MIN_OVERLAY_HEIGHT), (2560, 1440));
        assert_eq!(fitted, size(MIN_OVERLAY_WIDTH, MIN_OVERLAY_HEIGHT));
    }
}
