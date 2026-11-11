//! Tray icon and macOS application menu.

use serde_json::Value;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};

pub const TRAY_ID: &str = "main";
pub const MENU_SHOW: &str = "tray-show";
pub const MENU_EXIT: &str = "exit";
pub const MENU_ABOUT: &str = "app-about";

fn label<'a>(labels: &'a Value, key: &str, default: &'a str) -> &'a str {
    labels[key].as_str().filter(|s| !s.is_empty()).unwrap_or(default)
}

pub fn toggle_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            show_main_window(app);
        }
    }
}

pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn tray_menu<R: Runtime>(app: &AppHandle<R>, labels: &Value) -> tauri::Result<Menu<R>> {
    if labels.is_null() {
        return Menu::with_items(app, &[&MenuItem::with_id(app, MENU_EXIT, "Quit", true, None::<&str>)?]);
    }
    Menu::with_items(app, &[
        &MenuItem::with_id(app, MENU_SHOW, label(labels, "show", "Show AriaDesk"), true, None::<&str>)?,
        &PredefinedMenuItem::separator(app)?,
        &MenuItem::with_id(app, MENU_EXIT, label(labels, "exit", "Exit"), true, None::<&str>)?,
    ])
}

/// Creates the tray icon on Windows and Linux. Returns whether a tray is available.
pub fn init_tray<R: Runtime>(app: &AppHandle<R>) -> bool {
    if cfg!(target_os = "macos") {
        return false;
    }
    let Ok(menu) = tray_menu(app, &Value::Null) else { return false };
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("AriaDesk")
        .menu(&menu)
        .show_menu_on_left_click(cfg!(target_os = "linux"))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::DoubleClick { .. } = event {
                toggle_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app).is_ok()
}

pub fn set_tray_menu<R: Runtime>(app: &AppHandle<R>, labels: &Value) {
    if let (Some(tray), Ok(menu)) = (app.tray_by_id(TRAY_ID), tray_menu(app, labels)) {
        let _ = tray.set_menu(Some(menu));
    }
}

pub fn set_tray_tooltip<R: Runtime>(app: &AppHandle<R>, tooltip: &str) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_tooltip(Some(tooltip));
    }
}

/// The application menu only exists on macOS.
pub fn set_app_menu<R: Runtime>(app: &AppHandle<R>, labels: &Value) -> tauri::Result<()> {
    if !cfg!(target_os = "macos") {
        return Ok(());
    }
    let l = |k, d| Some(label(labels, k, d));
    let menu = Menu::with_items(app, &[
        &Submenu::with_items(app, "AriaDesk", true, &[
            &MenuItem::with_id(app, MENU_ABOUT, label(labels, "about", "About AriaDesk"), true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::services(app, l("services", "Services"))?,
            &PredefinedMenuItem::hide(app, l("hide", "Hide AriaDesk"))?,
            &PredefinedMenuItem::hide_others(app, l("hideOthers", "Hide Others"))?,
            &PredefinedMenuItem::show_all(app, l("showAll", "Show All"))?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, MENU_EXIT, label(labels, "quit", "Quit AriaDesk"), true, Some("CmdOrCtrl+Q"))?,
        ])?,
        &Submenu::with_items(app, label(labels, "edit", "Edit"), true, &[
            &PredefinedMenuItem::undo(app, l("undo", "Undo"))?,
            &PredefinedMenuItem::redo(app, l("redo", "Redo"))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, l("cut", "Cut"))?,
            &PredefinedMenuItem::copy(app, l("copy", "Copy"))?,
            &PredefinedMenuItem::paste(app, l("paste", "Paste"))?,
            &PredefinedMenuItem::select_all(app, l("selectAll", "Select All"))?,
        ])?,
        &Submenu::with_items(app, label(labels, "window", "Window"), true, &[
            &PredefinedMenuItem::minimize(app, l("minimize", "Minimize"))?,
            &PredefinedMenuItem::maximize(app, l("zoom", "Zoom"))?,
        ])?,
    ])?;
    app.set_menu(menu)?;
    Ok(())
}
