use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Manager, WindowEvent};

use crate::monitor;

pub const MAIN_LABEL: &str = "main";

#[derive(Debug, Default)]
pub struct LifecycleState {
    quitting: AtomicBool,
}

impl LifecycleState {
    fn is_quitting(&self) -> bool {
        self.quitting.load(Ordering::SeqCst)
    }

    fn mark_quitting(&self) {
        self.quitting.store(true, Ordering::SeqCst);
    }
}

#[cfg(target_os = "macos")]
fn set_dock_visible(app: &AppHandle, visible: bool) {
    let policy = if visible {
        tauri::ActivationPolicy::Regular
    } else {
        tauri::ActivationPolicy::Accessory
    };
    if let Err(e) = app.set_activation_policy(policy) {
        tracing::error!(visible, "切换激活策略失败：{e}");
    }
}

pub fn show_main(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    set_dock_visible(app, true);

    let Some(window) = app.get_webview_window(MAIN_LABEL) else {
        tracing::error!("找不到主窗口（label={MAIN_LABEL}）");
        return;
    };

    let _ = window.unminimize();
    if let Err(e) = window.show().and_then(|_| window.set_focus()) {
        tracing::error!("唤回主窗口失败：{e}");
    } else {
        monitor::on_window_shown(app);
    }
}

pub fn quit_app(app: &AppHandle) {
    tracing::info!("用户请求退出（唯一退出路径）");
    if let Some(state) = app.try_state::<LifecycleState>() {
        state.mark_quitting();
    }
    app.exit(0);
}

pub fn on_window_event(window: &tauri::Window, event: &WindowEvent) {
    let WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };

    let quitting = window
        .app_handle()
        .try_state::<LifecycleState>()
        .map(|s| s.is_quitting())
        .unwrap_or(false);

    if quitting {
        return;
    }

    api.prevent_close();

    if let Err(e) = window.hide() {
        tracing::error!(label = window.label(), "隐藏窗口失败：{e}");
    } else {
        if window.label() == MAIN_LABEL {
            monitor::on_window_hidden(window.app_handle());
        }
        #[cfg(target_os = "macos")]
        {
            if window.label() == MAIN_LABEL {
                set_dock_visible(window.app_handle(), false);
            }
        }
        tracing::debug!(label = window.label(), "窗口已收进托盘");
    }
}

pub fn on_run_event(app: &AppHandle, event: &tauri::RunEvent) -> bool {
    if let tauri::RunEvent::ExitRequested { .. } = event {
        let quitting = app
            .try_state::<LifecycleState>()
            .map(|s| s.is_quitting())
            .unwrap_or(false);
        if !quitting {
            tracing::debug!("拦下退出请求（后台常驻中）");
            return true;
        }
    }
    false
}
