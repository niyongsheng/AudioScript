pub mod geometry;
pub mod native;
#[cfg(target_os = "macos")]
pub mod panel;

#[cfg(target_os = "macos")]
pub mod watchdog;

use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

use crate::config::{OverlayConfig, OverlayMode, MAX_OVERLAY_HEIGHT, MIN_OVERLAY_HEIGHT};
use crate::i18n::{tf, tr, Key};

pub const LABEL: &str = "overlay";

pub const ENV_OVERLAY_LEVEL: &str = "AUDIOSCRIPT_OVERLAY_LEVEL";

pub const EVENT_OVERLAY_STATE: &str = "overlay://state";

pub const EVENT_OVERLAY_VISIBILITY: &str = "overlay://visibility";

#[derive(Debug, Default)]
pub struct OverlayState {
    config: Mutex<OverlayConfig>,
    applied_click_through: Mutex<Option<bool>>,
}

impl OverlayState {
    pub fn config(&self) -> OverlayConfig {
        self.config
            .lock()
            .expect("overlay config mutex poisoned")
            .clone()
    }

    pub fn mode(&self) -> OverlayMode {
        self.config().mode
    }

    pub fn mode_is_click_through(&self) -> bool {
        self.mode().is_click_through()
    }

    fn replace(&self, cfg: OverlayConfig) {
        *self.config.lock().expect("overlay config mutex poisoned") = cfg;
    }

    fn set_mode(&self, mode: OverlayMode) {
        self.config
            .lock()
            .expect("overlay config mutex poisoned")
            .mode = mode;
    }

    fn set_enable(&self, enable: bool) {
        self.config
            .lock()
            .expect("overlay config mutex poisoned")
            .enable = enable;
    }

    fn needs_click_through_write(&self, want: bool) -> bool {
        let mut cur = self
            .applied_click_through
            .lock()
            .expect("overlay click-through mutex poisoned");
        if *cur == Some(want) {
            return false;
        }
        *cur = Some(want);
        true
    }
}

pub(crate) enum Frame {
    #[cfg(target_os = "macos")]
    Panel(tauri_nspanel::PanelHandle<tauri::Wry>),
    Window(Box<WebviewWindow>),
}

impl Frame {
    pub(crate) fn get(app: &AppHandle) -> Result<Self, String> {
        #[cfg(target_os = "macos")]
        {
            use tauri_nspanel::ManagerExt;
            if let Ok(p) = app.get_webview_panel(LABEL) {
                return Ok(Self::Panel(p));
            }
        }

        app.get_webview_window(LABEL)
            .map(|w| Self::Window(Box::new(w)))
            .ok_or_else(|| tf(Key::OverlayNotFound, &[LABEL]))
    }

    fn show(&self) -> Result<(), String> {
        match self {
            #[cfg(target_os = "macos")]
            Self::Panel(p) => {
                p.show();
                Ok(())
            }
            Self::Window(w) => w
                .show()
                .map_err(|e| tf(Key::OverlayShowFailed, &[&e.to_string()])),
        }
    }

    fn hide(&self) -> Result<(), String> {
        match self {
            #[cfg(target_os = "macos")]
            Self::Panel(p) => {
                p.hide();
                Ok(())
            }
            Self::Window(w) => w
                .hide()
                .map_err(|e| tf(Key::OverlayHideFailed, &[&e.to_string()])),
        }
    }

    fn is_visible(&self) -> bool {
        match self {
            #[cfg(target_os = "macos")]
            Self::Panel(p) => p.is_visible(),
            Self::Window(w) => w.is_visible().unwrap_or(false),
        }
    }

    fn set_click_through(&self, on: bool) -> Result<(), String> {
        match self {
            #[cfg(target_os = "macos")]
            Self::Panel(p) => {
                p.set_ignores_mouse_events(on);
                Ok(())
            }
            Self::Window(w) => w
                .set_ignore_cursor_events(on)
                .map_err(|e| tf(Key::OverlayClickThroughFailed, &[&e.to_string()])),
        }
    }

    fn label(&self) -> &str {
        match self {
            #[cfg(target_os = "macos")]
            Self::Panel(p) => p.label(),
            Self::Window(w) => w.label(),
        }
    }
}

pub(crate) fn window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(LABEL)
}

pub(crate) fn geometry(app: &AppHandle) -> Option<crate::config::WindowGeometry> {
    use crate::config::{WindowGeometry, WindowPos, WindowSize};

    let w = window(app)?;
    let pos = w.outer_position().ok()?;
    let size = w.outer_size().ok()?;
    let scale = w.scale_factor().ok()?;

    Some(WindowGeometry {
        position: WindowPos {
            x: (pos.x as f64 / scale).round() as i32,
            y: (pos.y as f64 / scale).round() as i32,
        },
        size: WindowSize {
            width: (size.width as f64 / scale).round() as u32,
            height: (size.height as f64 / scale).round() as u32,
        },
    })
}

pub(crate) fn set_geometry(
    app: &AppHandle,
    g: &crate::config::WindowGeometry,
) -> Result<(), String> {
    let w = window(app).ok_or_else(|| tr(Key::OverlayWindowUnavailable))?;
    w.set_size(tauri::LogicalSize::new(
        g.size.width as f64,
        g.size.height as f64,
    ))
    .map_err(|e| tf(Key::OverlaySizeFailed, &[&e.to_string()]))?;
    w.set_position(tauri::LogicalPosition::new(
        g.position.x as f64,
        g.position.y as f64,
    ))
    .map_err(|e| tf(Key::OverlayPositionFailed, &[&e.to_string()]))?;
    Ok(())
}

pub fn toggle(app: &AppHandle) -> Result<bool, String> {
    let frame = Frame::get(app)?;
    let visible = frame.is_visible();

    if visible {
        frame.hide()?;
    } else {
        frame.show()?;
        apply_fullscreen_overlay(app, &frame);
    }

    let now = !visible;
    set_enable(app, now);
    broadcast_visibility(app, now);
    crate::telemetry::track(
        app,
        "overlay_toggled",
        Some(serde_json::json!({ "visible": if now { 1 } else { 0 } })),
    );

    Ok(now)
}

fn set_enable(app: &AppHandle, enable: bool) {
    let state = app.state::<OverlayState>();
    state.set_enable(enable);
    if let Err(e) = crate::prefs::update(app, |p| p.overlay.enable = enable) {
        tracing::error!("悬浮窗可见性保存失败：{e}（重启后会回到上次落盘的状态）");
    }
    crate::tray::sync_visibility(app, enable);
    broadcast(app, &state);
}

pub fn is_visible(app: &AppHandle) -> bool {
    Frame::get(app).map(|f| f.is_visible()).unwrap_or(false)
}

fn broadcast_visibility(app: &AppHandle, visible: bool) {
    if let Err(e) = app.emit(EVENT_OVERLAY_VISIBILITY, visible) {
        tracing::debug!("广播悬浮窗可见性失败（可能没有窗口在监听）：{e}");
    }
}

fn apply_mode(app: &AppHandle, mode: OverlayMode) -> Result<(), String> {
    let frame = Frame::get(app)?;
    let state = app.state::<OverlayState>();

    let want = mode.is_click_through();
    if state.needs_click_through_write(want) {
        if let Err(e) = frame.set_click_through(want) {
            *state
                .applied_click_through
                .lock()
                .expect("overlay click-through mutex poisoned") = None;
            return Err(format!("{e}（mode={}）", mode.as_str()));
        }
    }

    state.set_mode(mode);
    Ok(())
}

pub fn set_mode(app: &AppHandle, mode: OverlayMode) -> Result<OverlayMode, String> {
    apply_mode(app, mode)?;
    if let Err(e) = crate::prefs::update(app, |p| p.overlay.mode = mode) {
        tracing::error!("悬浮窗交互模式保存失败：{e}（重启后会回到上次落盘的模式）");
    }
    crate::tray::sync_mode(app, mode);
    broadcast(app, &app.state::<OverlayState>());
    Ok(mode)
}

pub fn config(app: &AppHandle) -> OverlayConfig {
    app.state::<OverlayState>().config()
}

pub fn set_config(app: &AppHandle, cfg: OverlayConfig) -> Result<OverlayConfig, String> {
    let cfg = cfg.sanitized();
    let state = app.state::<OverlayState>();
    let previous = state.config();

    if cfg.mode != state.mode() {
        apply_mode(app, cfg.mode)?;
    }

    #[cfg(target_os = "macos")]
    if !cfg.always_on_top {
        tracing::warn!(
            "always_on_top=false 在 macOS 上不被采纳：跨全屏置顶依赖强制的 NSScreenSaverWindowLevel"
        );
    }

    state.replace(cfg.clone());
    crate::tray::sync_mode(app, cfg.mode);
    if previous.font_size != cfg.font_size || previous.max_lines != cfg.max_lines {
        fit_height_to_lines(app, &cfg);
    }
    if let Err(e) = crate::prefs::update(app, |p| p.overlay = cfg.clone()) {
        tracing::error!("悬浮窗配置保存失败，本次调整不会被记住：{e}");
    }
    broadcast(app, &state);
    Ok(cfg)
}

const OVERLAY_LINE_HEIGHT: f64 = 1.35;
const OVERLAY_VERTICAL_PADDING: f64 = 16.0;

fn lines_height(font_size: u8, max_lines: u16) -> f64 {
    font_size as f64 * OVERLAY_LINE_HEIGHT * max_lines as f64 + OVERLAY_VERTICAL_PADDING
}

fn fit_height_to_lines(app: &AppHandle, cfg: &OverlayConfig) {
    let Some(win) = window(app) else {
        return;
    };
    let Some(g) = geometry(app) else {
        tracing::warn!("读取悬浮窗尺寸失败，跳过按行数调整高度");
        return;
    };

    let min_h = MIN_OVERLAY_HEIGHT as i32;
    let want = (lines_height(cfg.font_size, cfg.max_lines).round() as i32)
        .clamp(min_h, MAX_OVERLAY_HEIGHT as i32);

    let monitor = win.current_monitor().ok().flatten();
    let (monitor_y, monitor_h) = monitor
        .as_ref()
        .map(|m| {
            let (_, y, _, h) = geometry::logical_monitor_rect(m);
            (y, h as i32)
        })
        .unwrap_or((0, MAX_OVERLAY_HEIGHT as i32));

    let grow_up = g.position.y + g.size.height as i32 / 2 > monitor_y + monitor_h / 2;
    let available = if grow_up {
        g.position.y + g.size.height as i32 - monitor_y
    } else {
        monitor_y + monitor_h - g.position.y
    };
    let height = want.min(available.max(min_h)).max(min_h) as u32;

    if height == g.size.height {
        return;
    }

    if grow_up {
        let y = g.position.y + g.size.height as i32 - height as i32;
        if let Err(e) = win.set_position(tauri::LogicalPosition::new(g.position.x as f64, y as f64))
        {
            tracing::warn!("调整悬浮窗位置失败：{e}");
        }
    }
    match win.set_size(tauri::LogicalSize::new(g.size.width as f64, height as f64)) {
        Ok(()) => tracing::info!(
            font_size = cfg.font_size,
            max_lines = cfg.max_lines,
            height,
            "悬浮窗高度已按设置调整"
        ),
        Err(e) => tracing::warn!("按行数调整悬浮窗高度失败：{e}"),
    }
}

fn broadcast(app: &AppHandle, state: &OverlayState) {
    if let Err(e) = app.emit(EVENT_OVERLAY_STATE, state.config()) {
        tracing::debug!("广播悬浮窗状态失败（可能没有窗口在监听）：{e}");
    }
}

pub fn init(
    app: &AppHandle,
    cfg: OverlayConfig,
    geometry: Option<crate::config::WindowGeometry>,
) -> Result<(), String> {
    let frame = Frame::get(app)?;
    let had_geometry = geometry.is_some();

    geometry::restore(app, geometry);

    let state = app.state::<OverlayState>();
    state.replace(cfg.sanitized());
    let mode = state.mode();

    if !had_geometry {
        fit_height_to_lines(app, &state.config());
    }

    frame.set_click_through(mode.is_click_through())?;
    state.needs_click_through_write(mode.is_click_through());

    apply_fullscreen_overlay(app, &frame);

    broadcast(app, &state);

    let applied = state.config();
    tracing::info!(
        font_size = applied.font_size,
        opacity = applied.opacity,
        max_lines = applied.max_lines,
        font_color = %applied.font_color,
        bg_color = %applied.bg_color,
        mode = applied.mode.as_str(),
        "悬浮窗外观已应用"
    );

    if applied.enable {
        if let Err(e) = frame.show() {
            tracing::error!("悬浮窗启动显示失败：{e}（可在悬浮窗页手动显示）");
        }
    }

    #[cfg(target_os = "macos")]
    watchdog::start(app.clone());

    geometry::start_watch(app);

    Ok(())
}

pub fn on_page_loaded(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    {
        let Some(window) = app.get_webview_window(LABEL) else {
            tracing::error!("page-load 回调里找不到悬浮窗，无法换类");
            return;
        };

        let handle = app.clone();
        let window_for_convert = window.clone();
        if let Err(e) =
            window.run_on_main_thread(move || match panel::convert(&window_for_convert) {
                Ok(_) => {
                    tracing::info!("悬浮窗已换类为 NSPanel（跨全屏的前提）");
                    if let Ok(frame) = Frame::get(&handle) {
                        apply_fullscreen_overlay(&handle, &frame);
                    }
                }
                Err(e) => tracing::error!(
                    "悬浮窗换类失败：{e}\n\
                 → 后果：其他应用全屏时悬浮窗不可见（看视频读字幕的主场景会失效）"
                ),
            })
        {
            tracing::error!("无法投递换类任务到主线程：{e}");
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
    }
}

fn apply_fullscreen_overlay(app: &AppHandle, frame: &Frame) {
    #[cfg(target_os = "macos")]
    {
        let Frame::Panel(p) = frame else {
            tracing::debug!("悬浮窗尚未换类，跳过全屏属性设置");
            return;
        };

        let panel = p.clone();
        let handle = app.clone();
        if let Err(e) = app.run_on_main_thread(move || {
            native::macos::apply_overlay_panel_attributes(&panel);
            match debug_state(&handle) {
                Ok(s) => tracing::info!(state = %s, "悬浮窗原生属性已应用"),
                Err(e) => tracing::warn!("属性已设置但状态回读失败：{e}"),
            }
        }) {
            tracing::error!("无法投递原生属性设置到主线程：{e}");
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, frame);
    }
}

pub fn debug_state(app: &AppHandle) -> Result<serde_json::Value, String> {
    let frame = Frame::get(app)?;

    let mut out = serde_json::json!({
        "label": frame.label(),
        "visible": frame.is_visible(),
        "clickThrough": app.state::<OverlayState>().mode_is_click_through(),
    });

    if let Some(g) = geometry(app) {
        out["geometry"] = serde_json::json!({
            "x": g.position.x,
            "y": g.position.y,
            "width": g.size.width,
            "height": g.size.height,
        });
    }

    #[cfg(target_os = "macos")]
    {
        out["convertedToPanel"] = serde_json::json!(matches!(frame, Frame::Panel(_)));

        if let Frame::Panel(p) = &frame {
            out["level"] = serde_json::json!(native::macos::current_level(p));
            out["reallyPainted"] = serde_json::json!(native::macos::really_painted(p));
            out["onActiveSpace"] = serde_json::json!(native::macos::on_active_space(p));
            out["styleMask"] = serde_json::json!(format!("{:?}", p.as_panel().styleMask()));
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_height_matches_the_css_budget() {
        assert!((lines_height(22, 6) - (22.0 * 1.35 * 6.0 + 16.0)).abs() < 1e-9);
        assert!(lines_height(48, 20) > lines_height(22, 6));
    }
}
