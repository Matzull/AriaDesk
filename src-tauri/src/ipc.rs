//! Channel based IPC. The frontend (ui/src/lib/native.js) forwards fire-and-forget messages
//! to `ipc_send` and requests to `ipc_invoke`; the backend answers with events (see `emit`).

use std::path::Path;
use std::sync::atomic::Ordering;

use serde_json::{json, Value};
use tauri::{AppHandle, Manager, Theme};
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

use crate::{emit, notify_new_task, ui, websocket, AppState, NEW_TASK_LOCATION};

fn arg_str(args: &[Value], i: usize) -> String {
    args.get(i).and_then(Value::as_str).unwrap_or_default().to_owned()
}

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).cloned().unwrap_or(Value::Null)
}

fn update_config(app: &AppHandle, f: impl FnOnce(&mut crate::config::Config)) {
    let state = app.state::<AppState>();
    let mut config = state.config.lock().unwrap();
    f(&mut config);
    config.save(app);
}

fn magnet_is_default(app: &AppHandle) -> bool {
    app.deep_link().is_registered("magnet").unwrap_or(false)
}

#[tauri::command]
pub fn ipc_send(app: AppHandle, channel: String, args: Vec<Value>) {
    let state = app.state::<AppState>();
    let window = app.get_webview_window("main");
    match channel.as_str() {
        "ready" => {
            websocket().reset();
            if !state.minimal {
                if let Some(w) = &window {
                    let _ = w.show();
                }
            }
        }
        "route-changed" => {
            if arg_str(&args, 0).starts_with(NEW_TASK_LOCATION) {
                if let Some(pending) = state.pending.lock().unwrap().take() {
                    notify_new_task(&app, pending, true);
                }
            }
        }
        "set-theme" => {
            let theme = match arg_str(&args, 0).as_str() {
                "dark" => Some(Theme::Dark),
                "light" => Some(Theme::Light),
                _ => None,
            };
            if let Some(w) = &window {
                let _ = w.set_theme(theme);
            }
        }
        "reload" => {
            if let Some(w) = &window {
                let _ = w.eval("window.location.reload()");
            }
        }
        "notify" => {
            let ctx = arg(&args, 0);
            let _ = app
                .notification()
                .builder()
                .title(ctx["title"].as_str().unwrap_or("AriaDesk"))
                .body(ctx["body"].as_str().unwrap_or_default())
                .show();
        }
        "set-menu-labels" => {
            let labels = arg(&args, 0);
            let _ = ui::set_app_menu(&app, &labels);
            ui::set_tray_menu(&app, &labels);
        }
        "set-tray-tooltip" => ui::set_tray_tooltip(&app, &arg_str(&args, 0)),
        "set-config" => {
            let value = arg(&args, 1);
            let text = value.as_str().filter(|s| !s.is_empty()).map(str::to_owned);
            let flag = value.as_bool().unwrap_or(false);
            match arg_str(&args, 0).as_str() {
                "defaultPosition" => update_config(&app, |c| c.default_position = text.unwrap_or_default()),
                "minimizedToTray" => update_config(&app, |c| c.minimized_to_tray = flag),
                "execCommandOnStartup" => update_config(&app, |c| c.exec_command_on_startup = text),
                "execCommandArgumentsOnStartup" => update_config(&app, |c| c.exec_command_arguments_on_startup = text),
                "execDetachedCommandOnStartup" => update_config(&app, |c| c.exec_detached_command_on_startup = flag),
                key => emit(&app, "log", json!([format!("[ipc] unknown config key {key}")])),
            }
        }
        "ws-connect" => websocket().connect(app.clone(), arg_str(&args, 0), arg_str(&args, 1)),
        "ws-close" => websocket().close(&arg_str(&args, 0)),
        "ws-send" => {
            let ctx = arg(&args, 0);
            let data = match &ctx["data"] {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            websocket().send(&app, ctx["id"].as_str().unwrap_or_default(), data);
        }
        "open-url" => {
            let _ = app.opener().open_url(arg_str(&args, 0), None::<&str>);
        }
        "open-dir" => {
            let dir = arg_str(&args, 0);
            let full = Path::new(&dir).join(arg_str(&args, 1));
            if full.exists() {
                let _ = app.opener().reveal_item_in_dir(full);
            } else if Path::new(&dir).is_dir() {
                let _ = app.opener().open_path(dir, None::<&str>);
            }
        }
        "devtools" => {
            if let Some(w) = &window {
                if state.dev_mode.load(Ordering::Relaxed) {
                    w.open_devtools();
                }
            }
        }
        _ => emit(&app, "log", json!([format!("[ipc] unknown channel {channel}")])),
    }
}

#[tauri::command]
pub async fn ipc_invoke(app: AppHandle, channel: String, args: Vec<Value>) -> Value {
    let state = app.state::<AppState>();
    match channel.as_str() {
        "set-magnet-handler" => {
            let enabled = arg(&args, 0).as_bool().unwrap_or(false);
            update_config(&app, |c| c.default_magnet_protocol_client = enabled);
            let dl = app.deep_link();
            let registered = if enabled { dl.register("magnet") } else { dl.unregister("magnet") }.is_ok();
            json!({ "enabled": enabled, "isDefault": magnet_is_default(&app), "registered": registered })
        }
        "magnet-status" => json!({
            "enabled": state.config.lock().unwrap().default_magnet_protocol_client,
            "isDefault": magnet_is_default(&app),
        }),
        "open-default-apps-settings" => {
            let url = if cfg!(windows) {
                "ms-settings:defaultapps"
            } else if cfg!(target_os = "macos") {
                "x-apple.systempreferences:com.apple.preference.general"
            } else {
                return json!(false);
            };
            json!(app.opener().open_url(url, None::<&str>).is_ok())
        }
        "startup-output" => {
            if state.dev_mode.load(Ordering::Relaxed) {
                json!(*state.startup_output.lock().unwrap())
            } else {
                json!([])
            }
        }
        "http" => {
            let user_agent = format!("AriaDesk/{}", app.package_info().version);
            crate::http::request(arg(&args, 0), &user_agent).await
        }
        _ => {
            emit(&app, "log", json!([format!("[ipc] unknown channel {channel}")]));
            Value::Null
        }
    }
}
