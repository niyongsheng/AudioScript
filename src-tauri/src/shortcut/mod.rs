use std::sync::Mutex;

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::config::OverlayMode;
use crate::i18n::{tf, tr, Key};
use crate::{overlay, prefs};

pub struct Binding {
    pub id: &'static str,
    pub default_accelerator: &'static str,
    pub action: fn(&AppHandle),
}

#[derive(Debug, Default)]
pub struct HotkeyState {
    failures: Mutex<Vec<ShortcutFailure>>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutFailure {
    pub id: String,
    pub accelerator: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyInfo {
    pub id: String,
    pub accelerator: String,
    pub accelerator_pretty: String,
    pub default_accelerator: String,
    pub customized: bool,
    pub registered: bool,
}

impl HotkeyState {
    pub fn failures(&self) -> Vec<ShortcutFailure> {
        self.failures
            .lock()
            .expect("shortcut mutex poisoned")
            .clone()
    }

    fn clear_failures(&self) {
        self.failures
            .lock()
            .expect("shortcut mutex poisoned")
            .clear();
    }

    fn push(&self, f: ShortcutFailure) {
        self.failures
            .lock()
            .expect("shortcut mutex poisoned")
            .push(f);
    }
}

pub fn bindings() -> Vec<Binding> {
    vec![
        Binding {
            id: "overlay.toggleMode",
            default_accelerator: "CmdOrControl+Shift+L",
            action: |app| {
                let current = app.state::<overlay::OverlayState>().mode();
                let next = match current {
                    OverlayMode::Locked => OverlayMode::Edit,
                    OverlayMode::Edit => OverlayMode::Locked,
                };
                match overlay::set_mode(app, next) {
                    Ok(m) => tracing::info!(mode = m.as_str(), "热键切换悬浮窗模式"),
                    Err(e) => tracing::error!("{e}"),
                }
            },
        },
        Binding {
            id: "overlay.toggleVisible",
            default_accelerator: "CmdOrControl+Shift+H",
            action: |app| match overlay::toggle(app) {
                Ok(visible) => tracing::info!(visible, "热键切换悬浮窗可见性"),
                Err(e) => tracing::error!("{e}"),
            },
        },
    ]
}

pub fn pretty(accelerator: &str) -> String {
    #[cfg(target_os = "macos")]
    let (cmd, ctrl, alt, shift) = ("⌘", "⌃", "⌥", "⇧");
    #[cfg(not(target_os = "macos"))]
    let (cmd, ctrl, alt, shift) = ("Ctrl", "Ctrl", "Alt", "Shift");

    accelerator
        .split('+')
        .map(|part| match part.trim() {
            "CmdOrControl" | "Cmd" | "Command" | "Super" | "Meta" => cmd.to_owned(),
            "Control" | "Ctrl" => ctrl.to_owned(),
            "Alt" | "Option" => alt.to_owned(),
            "Shift" => shift.to_owned(),
            other => other.strip_prefix("Key").unwrap_or(other).to_owned(),
        })
        .collect()
}

fn accelerator_from(saved: &std::collections::HashMap<String, String>, b: &Binding) -> String {
    saved
        .get(b.id)
        .filter(|s| !s.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| b.default_accelerator.to_owned())
}

pub fn accelerator_for(app: &AppHandle, b: &Binding) -> String {
    accelerator_from(&prefs::load(app).shortcuts, b)
}

pub fn list(app: &AppHandle) -> Vec<HotkeyInfo> {
    let saved = prefs::load(app).shortcuts;
    let gs = app.global_shortcut();

    bindings()
        .into_iter()
        .map(|b| {
            let customized = saved.get(b.id).is_some_and(|s| !s.trim().is_empty());
            let accel = accelerator_from(&saved, &b);
            HotkeyInfo {
                id: b.id.to_owned(),
                registered: gs.is_registered(accel.as_str()),
                accelerator_pretty: pretty(&accel),
                accelerator: accel,
                default_accelerator: b.default_accelerator.to_owned(),
                customized,
            }
        })
        .collect()
}

pub fn rebind(app: &AppHandle, id: &str, accelerator: &str) -> Result<Vec<HotkeyInfo>, String> {
    let accel = accelerator.trim();
    if accel.is_empty() {
        return Err(tr(Key::ShortcutEmpty).into());
    }

    let bindings = bindings();
    let b = bindings
        .iter()
        .find(|b| b.id == id)
        .ok_or_else(|| tf(Key::ShortcutUnknownId, &[id]))?;

    let old = accelerator_from(&prefs::load(app).shortcuts, b);

    accel
        .parse::<Shortcut>()
        .map_err(|e| tf(Key::ShortcutParseFailed, &[accel, &e.to_string()]))?;

    let gs = app.global_shortcut();

    if old != accel {
        let action = b.action;
        gs.on_shortcut(accel, move |app, _sc, event| {
            if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                action(app);
            }
        })
        .map_err(|e| tf(Key::ShortcutRegisterFailed, &[accel, &e.to_string()]))?;

        if let Err(e) = gs.unregister(old.as_str()) {
            tracing::warn!(old = %old, "旧热键注销失败，它可能仍然生效：{e}");
        }
    }

    let is_default = accel == b.default_accelerator;
    prefs::update(app, |p| {
        if is_default {
            p.shortcuts.remove(id);
        } else {
            p.shortcuts.insert(id.to_owned(), accel.to_owned());
        }
    })?;

    tracing::info!(id, old = %old, new = %accel, "热键已改绑");
    Ok(list(app))
}

pub fn init(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<HotkeyState>();
    state.clear_failures();
    let saved = prefs::load(app).shortcuts;

    for b in bindings() {
        let action = b.action;
        let id = b.id;
        let accel = accelerator_from(&saved, &b);

        let result = app
            .global_shortcut()
            .on_shortcut(accel.as_str(), move |app, _sc, event| {
                if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                    action(app);
                }
            });

        match result {
            Ok(()) => tracing::info!(id, accelerator = %accel, "热键已注册"),
            Err(e) => {
                tracing::error!(
                    id,
                    accelerator = %accel,
                    "热键注册失败（组合键可能已被其他应用占用）：{e}"
                );
                state.push(ShortcutFailure {
                    id: id.to_string(),
                    accelerator: pretty(&accel),
                });
            }
        }
    }

    let failed = state.failures();
    if !failed.is_empty() {
        tracing::error!(
            count = failed.len(),
            "有 {} 条全局热键注册失败，悬浮窗的逃生路径可能不完整",
            failed.len()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pretty_renders_readable_keycaps() {
        let out = pretty("CmdOrControl+Shift+H");
        #[cfg(target_os = "macos")]
        assert_eq!(out, "⌘⇧H");
        #[cfg(not(target_os = "macos"))]
        assert_eq!(out, "Ctrl+Shift+H");
    }

    #[test]
    fn pretty_strips_the_key_prefix() {
        assert!(
            !pretty("CmdOrControl+KeyL").contains("Key"),
            "不该露出内部写法"
        );
    }

    #[test]
    fn pretty_keeps_function_keys() {
        assert!(pretty("CmdOrControl+F5").ends_with("F5"));
    }

    #[test]
    fn pretty_does_not_mangle_unknown_parts() {
        assert!(pretty("CmdOrControl+Weird").ends_with("Weird"));
    }
}
