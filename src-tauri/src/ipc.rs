//! Channel based IPC between the frontend and the backend.
//! The frontend shim forwards `ipcRenderer.send` to `ipc_send` and `ipcRenderer.invoke` to `ipc_invoke`.

use std::path::Path;
use std::sync::atomic::Ordering;

use serde_json::{json, Value};
use tauri::{AppHandle, Manager, Theme};
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

use crate::{emit, notify_new_task, open_new_task, ui, websocket, AppState, Pending, NEW_TASK_LOCATION};

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
        "on-render-electron-service-inited" => {
            websocket().reset();
            if !state.minimal {
                if let Some(w) = &window {
                    let _ = w.show();
                }
            }
        }
        "on-render-view-content-loaded" => {
            if arg_str(&args, 0).starts_with(NEW_TASK_LOCATION) {
                if let Some(pending) = state.pending.lock().unwrap().take() {
                    notify_new_task(&app, pending, true);
                }
            }
        }
        "on-render-new-drop-file" => {
            if let Some(path) = args.first().and_then(|a| a["filePath"].as_str()) {
                open_new_task(&app, Pending::File(path.to_owned()));
            }
        }
        "on-render-new-drop-text" => {
            if let Some(text) = args.first().and_then(|a| a["text"].as_str()) {
                open_new_task(&app, Pending::Text(text.to_owned()));
            }
        }
        "render-set-native-theme" => {
            let theme = match arg_str(&args, 0).as_str() {
                "dark" => Some(Theme::Dark),
                "light" => Some(Theme::Light),
                _ => None,
            };
            if let Some(w) = &window {
                let _ = w.set_theme(theme);
            }
        }
        "render-set-titlebar-color" => {} // the native title bar is always used
        "render-reload-native-window" => {
            if let Some(w) = &window {
                let _ = w.eval("window.location.reload()");
            }
        }
        "render-show-textbox-context-menu" => {
            if let Some(w) = &window {
                let labels = state.labels.lock().unwrap().clone();
                let _ = ui::popup_textbox_menu(w, &labels, &arg(&args, 0));
            }
        }
        "render-show-system-notification" => {
            let ctx = arg(&args, 0);
            let _ = app
                .notification()
                .builder()
                .title(ctx["title"].as_str().unwrap_or("AriaDesk"))
                .body(ctx["body"].as_str().unwrap_or_default())
                .show();
        }
        "render-update-app-menu-label" => {
            let labels = arg(&args, 0);
            let _ = ui::set_app_menu(&app, &labels);
            *state.labels.lock().unwrap() = labels;
        }
        "render-update-tray-menu-label" => ui::set_tray_menu(&app, &arg(&args, 0)),
        "render-update-tray-tip" => ui::set_tray_tooltip(&app, &arg_str(&args, 0)),
        "render-set-native-config-default-position" => update_config(&app, |c| c.default_position = arg_str(&args, 0)),
        "render-set-native-config-minimized-to-tray" => {
            update_config(&app, |c| c.minimized_to_tray = arg(&args, 0).as_bool().unwrap_or(false))
        }
        "render-set-native-config-exec-command-on-startup" => {
            update_config(&app, |c| c.exec_command_on_startup = args.first().and_then(Value::as_str).map(str::to_owned))
        }
        "render-set-native-config-exec-command-arguments-on-startup" => update_config(&app, |c| {
            c.exec_command_arguments_on_startup = args.first().and_then(Value::as_str).map(str::to_owned)
        }),
        "render-set-native-config-exec-detached-command-on-startup" => {
            update_config(&app, |c| c.exec_detached_command_on_startup = arg(&args, 0).as_bool().unwrap_or(false))
        }
        "render-set-native-config-last-check-updates-time" => {
            update_config(&app, |c| c.last_check_updates_time = arg(&args, 0).as_i64().unwrap_or(0))
        }
        "render-connect-websocket" => websocket().connect(app.clone(), arg_str(&args, 0), &arg(&args, 1)),
        "render-reconnect-websocket" => websocket().reconnect(app.clone()),
        "render-send-websocket-message" => {
            let ctx = arg(&args, 0);
            let data = match &ctx["data"] {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            websocket().send(&app, ctx["url"].as_str().unwrap_or_default(), data);
        }
        "render-open-external-url" => {
            let _ = app.opener().open_url(arg_str(&args, 0), None::<&str>);
        }
        "render-open-local-directory" => {
            let dir = arg_str(&args, 0);
            let full = Path::new(&dir).join(arg_str(&args, 1));
            if full.exists() {
                let _ = app.opener().reveal_item_in_dir(full);
            } else if Path::new(&dir).is_dir() {
                let _ = app.opener().open_path(dir, None::<&str>);
            }
        }
        "render-show-dev-tools" => {
            if let Some(w) = &window {
                w.open_devtools();
            }
        }
        _ => emit(&app, "on-main-log-warn", json!([format!("[ipc] unknown channel {channel}")])),
    }
}

#[tauri::command]
pub async fn ipc_invoke(app: AppHandle, channel: String, args: Vec<Value>) -> Value {
    let state = app.state::<AppState>();
    match channel.as_str() {
        "render-set-native-config-default-magnet-protocol-client" => {
            let enabled = arg(&args, 0).as_bool().unwrap_or(false);
            update_config(&app, |c| c.default_magnet_protocol_client = enabled);
            let dl = app.deep_link();
            let registered = if enabled { dl.register("magnet") } else { dl.unregister("magnet") }.is_ok();
            json!({ "enabled": enabled, "isDefault": magnet_is_default(&app), "registered": registered })
        }
        "render-get-system-magnet-protocol-status" => json!({
            "enabled": state.config.lock().unwrap().default_magnet_protocol_client,
            "isDefault": magnet_is_default(&app),
        }),
        "render-open-system-default-apps-setting" => {
            let url = if cfg!(windows) {
                "ms-settings:defaultapps"
            } else if cfg!(target_os = "macos") {
                "x-apple.systempreferences:com.apple.preference.general"
            } else {
                return json!(false);
            };
            json!(app.opener().open_url(url, None::<&str>).is_ok())
        }
        "render-get-native-config-last-check-updates-time" => json!(state.config.lock().unwrap().last_check_updates_time),
        "render-get-startup-command-process-output" => {
            if state.dev_mode.load(Ordering::Relaxed) {
                json!(*state.startup_output.lock().unwrap())
            } else {
                json!([])
            }
        }
        "render-request-http" => {
            let user_agent = format!("AriaDesk/{}", app.package_info().version);
            crate::http::request(arg(&args, 0), &user_agent).await
        }
        "render-get-localfs-file-buffer" => match std::fs::read(arg_str(&args, 0)) {
            Ok(bytes) => json!(bytes),
            Err(_) => Value::Null,
        },
        "render-get-localfs-dir-exists" => json!(Path::new(&arg_str(&args, 0)).is_dir()),
        "render-show-open-file-dialog" => {
            let mut dialog = app.dialog().file();
            for filter in arg(&args, 0).as_array().into_iter().flatten() {
                let exts: Vec<&str> = filter["extensions"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
                dialog = dialog.add_filter(filter["name"].as_str().unwrap_or_default(), &exts);
            }
            let (tx, rx) = tokio::sync::oneshot::channel();
            dialog.pick_file(move |path| {
                let _ = tx.send(path.and_then(|p| p.into_path().ok()));
            });
            match rx.await.ok().flatten() {
                Some(path) => json!({ "canceled": false, "filePaths": [path] }),
                None => json!({ "canceled": true, "filePaths": [] }),
            }
        }
        _ => {
            emit(&app, "on-main-log-warn", json!([format!("[ipc] unknown channel {channel}")]));
            Value::Null
        }
    }
}
