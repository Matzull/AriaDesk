mod aria2;
mod args;
mod config;
#[cfg(target_os = "linux")]
mod desktop_entry;
mod http;
mod ipc;
mod process;
mod theme;
mod ui;
mod ws;

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};

use base64::Engine;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager, RunEvent, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_deep_link::DeepLinkExt;

use config::Config;

pub const NEW_TASK_LOCATION: &str = "/new";
pub const SETTINGS_LOCATION: &str = "/settings/about";

static WEBSOCKET: LazyLock<ws::WebSocket> = LazyLock::new(ws::WebSocket::default);

/// A new task waiting for the "new task" page to finish loading.
pub enum Pending {
    File(String),
    Text(String),
}

pub struct AppState {
    pub config: Mutex<Config>,
    pub dev_mode: AtomicBool,
    pub confirm_exit: AtomicBool,
    pub tray_enabled: AtomicBool,
    pub minimal: bool,
    pub startup_output: process::OutputLog,
    pub pending: Mutex<Option<Pending>>,
    pub aria2: aria2::Aria2,
}

impl AppState {
    fn close_to_hide(&self) -> bool {
        (self.tray_enabled.load(Ordering::Relaxed) || cfg!(target_os = "macos"))
            && self.config.lock().unwrap().minimized_to_tray
            && !self.confirm_exit.load(Ordering::Relaxed)
    }
}

/// Sends an event to the frontend; the payload is the array of callback arguments.
pub fn emit(app: &AppHandle, channel: &str, args: Value) {
    let _ = app.emit_to("main", channel, args);
}

pub fn websocket() -> &'static ws::WebSocket {
    &WEBSOCKET
}

fn main_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("main")
}

fn current_location(app: &AppHandle) -> String {
    main_window(app)
        .and_then(|w| w.url().ok())
        .and_then(|u| u.fragment().map(|f| f.trim_start_matches('!').to_owned()))
        .unwrap_or_default()
}

pub fn navigate_to(app: &AppHandle, location: &str) {
    emit(app, "navigate", json!([location]));
}

pub fn notify_new_task(app: &AppHandle, pending: Pending, is_async: bool) {
    match pending {
        Pending::File(path) => {
            let p = Path::new(&path);
            let ext = p.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
            let Some(kind) = ariadesk_core::supported_file_extension(&ext) else {
                emit(app, "error", json!(["The selected file type is invalid!"]));
                return;
            };
            let result = match std::fs::read(p) {
                Ok(bytes) => json!({
                    "type": kind,
                    "fileName": p.file_name().map(|n| n.to_string_lossy()).unwrap_or_default(),
                    "base64Content": base64::engine::general_purpose::STANDARD.encode(bytes),
                    "async": is_async,
                }),
                Err(e) => json!({ "exception": e.to_string() }),
            };
            emit(app, "new-task-file", json!([result]));
        }
        Pending::Text(text) => {
            emit(app, "new-task-text", json!([{ "text": text, "append": !is_async, "async": is_async }]));
        }
    }
}

/// Delivers a new task to the "new task" page, navigating there first if needed.
pub fn open_new_task(app: &AppHandle, pending: Pending) {
    if current_location(app).starts_with(NEW_TASK_LOCATION) {
        notify_new_task(app, pending, false);
    } else {
        *app.state::<AppState>().pending.lock().unwrap() = Some(pending);
        navigate_to(app, NEW_TASK_LOCATION);
    }
}

fn pending_from_arg(arg: Option<String>) -> Option<Pending> {
    let arg = arg?;
    if args::is_supported_file(&arg) {
        Some(Pending::File(arg))
    } else if args::is_url(&arg) {
        Some(Pending::Text(arg))
    } else {
        None
    }
}

fn handle_url(app: &AppHandle, url: &url::Url) {
    let pending = match url.scheme() {
        "file" => url.to_file_path().ok().map(|p| Pending::File(p.to_string_lossy().into_owned())),
        _ => Some(Pending::Text(url.to_string())),
    };
    if let Some(pending) = pending {
        ui::show_main_window(app);
        open_new_task(app, pending);
    }
}

fn on_second_instance(app: &AppHandle, argv: Vec<String>) {
    ui::show_main_window(app);
    let parsed = args::parse(argv);
    if parsed.development {
        app.state::<AppState>().dev_mode.store(true, Ordering::Relaxed);
        emit(app, "dev-mode", json!([true]));
    }
    // magnet links in a second instance are delivered by the deep-link plugin
    let file_or_url = parsed.file_or_url.filter(|a| !a.starts_with("magnet:"));
    if let Some(pending) = pending_from_arg(file_or_url) {
        open_new_task(app, pending);
    }
}

fn boot_script(app: &AppHandle, initial_route: Option<&str>) -> String {
    let state = app.state::<AppState>();
    let boot = json!({
        "platform": "tauri",
        "os": std::env::consts::OS,
        "sep": std::path::MAIN_SEPARATOR.to_string(),
        "initialRoute": initial_route,
        "globals": {
            "version": app.package_info().version.to_string(),
            "buildCommit": env!("ARIADESK_BUILD_COMMIT"),
            "isDevMode": state.dev_mode.load(Ordering::Relaxed),
        },
        "nativeConfig": state.config.lock().unwrap().native_config(),
        "runtime": [
            { "name": "Tauri", "value": tauri::VERSION },
            { "name": "WebView", "value": tauri::webview_version().unwrap_or_default() },
            { "name": "OS", "value": format!("{} {}", std::env::consts::OS, std::env::consts::ARCH) },
        ],
    });
    format!("window.__ARIADESK_BOOT__ = {boot};")
}

fn create_main_window(app: &AppHandle, initial_route: Option<&str>) -> tauri::Result<()> {
    let state = app.state::<AppState>();
    let config = state.config.lock().unwrap().clone();

    let window = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("AriaDesk")
        .inner_size(config.width, config.height)
        .min_inner_size(800.0, 400.0)
        .visible(false)
        .initialization_script(boot_script(app, initial_route))
        .build()?;

    match (config.default_position.as_str(), config.x, config.y) {
        ("last-position", Some(x), Some(y)) if x >= 0 && y >= 0 => {
            let in_screen = window.available_monitors().unwrap_or_default().iter().any(|m| {
                let (p, s) = (m.position(), m.size());
                x >= p.x && x <= p.x + s.width as i32 && y >= p.y && y <= p.y + s.height as i32
            });
            if in_screen {
                let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
            }
        }
        ("screen-center", _, _) => {
            let _ = window.center();
        }
        _ => {}
    }
    if config.maximized {
        let _ = window.maximize();
    }

    let handle = app.clone();
    window.on_window_event(move |event| {
        let state = handle.state::<AppState>();
        match event {
            WindowEvent::CloseRequested { api, .. } => {
                if state.close_to_hide() {
                    api.prevent_close();
                    if let Some(w) = main_window(&handle) {
                        let _ = w.hide();
                    }
                }
            }
            WindowEvent::Resized(size) => {
                let Some(w) = main_window(&handle) else { return };
                let mut config = state.config.lock().unwrap();
                config.maximized = w.is_maximized().unwrap_or(false);
                if !config.maximized && size.width > 0 && size.height > 0 {
                    let logical = size.to_logical::<f64>(w.scale_factor().unwrap_or(1.0));
                    config.width = logical.width;
                    config.height = logical.height;
                }
            }
            WindowEvent::Moved(pos) => {
                let mut config = state.config.lock().unwrap();
                if !config.maximized {
                    config.x = Some(pos.x);
                    config.y = Some(pos.y);
                }
            }
            WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) => {
                if let Some(path) = paths.first() {
                    open_new_task(&handle, Pending::File(path.to_string_lossy().into_owned()));
                }
            }
            _ => {}
        }
    });
    Ok(())
}

fn setup(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let parsed = args::parse(std::env::args());
    let config = Config::load(app);
    let output = process::OutputLog::default();

    if let Some(command) = config.exec_command_on_startup.as_deref() {
        process::exec_on_startup(
            command,
            config.exec_command_arguments_on_startup.as_deref(),
            config.exec_detached_command_on_startup,
            output.clone(),
        );
    }
    if config.default_magnet_protocol_client {
        let _ = app.deep_link().register("magnet");
    }

    let pending = pending_from_arg(parsed.file_or_url);
    let initial_route = pending.as_ref().map(|_| NEW_TASK_LOCATION);

    app.manage(AppState {
        config: Mutex::new(config),
        dev_mode: AtomicBool::new(cfg!(debug_assertions) || parsed.development),
        confirm_exit: AtomicBool::new(false),
        tray_enabled: AtomicBool::new(false),
        minimal: parsed.minimal,
        startup_output: output,
        pending: Mutex::new(pending),
        aria2: aria2::Aria2::default(),
    });

    theme::init(app);
    #[cfg(target_os = "linux")]
    desktop_entry::ensure(app);
    create_main_window(app, initial_route)?;
    let tray = ui::init_tray(app);
    app.state::<AppState>().tray_enabled.store(tray, Ordering::Relaxed);
    ui::set_app_menu(app, &Value::Null)?;

    let handle = app.clone();
    app.deep_link().on_open_url(move |event| {
        for url in event.urls() {
            handle_url(&handle, &url);
        }
    });
    Ok(())
}

fn on_menu_event(app: &AppHandle, id: &str) {
    match id {
        ui::MENU_SHOW => ui::show_main_window(app),
        ui::MENU_EXIT => {
            app.state::<AppState>().confirm_exit.store(true, Ordering::Relaxed);
            app.exit(0);
        }
        ui::MENU_ABOUT => navigate_to(app, SETTINGS_LOCATION),
        _ => {}
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| on_second_instance(app, argv)))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| setup(app.handle()))
        .on_menu_event(|app, event| on_menu_event(app, event.id().as_ref()))
        .invoke_handler(tauri::generate_handler![ipc::ipc_send, ipc::ipc_invoke])
        .build(tauri::generate_context!())
        .expect("error while building AriaDesk")
        .run(|app, event| match event {
            RunEvent::Exit => {
                let state = app.state::<AppState>();
                state.aria2.stop();
                let config = state.config.lock().unwrap();
                config.save(app);
            }
            #[cfg(target_os = "macos")]
            RunEvent::Reopen { .. } => ui::show_main_window(app),
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            RunEvent::Opened { urls } => {
                // magnet links are delivered through the deep-link plugin
                for url in urls.iter().filter(|u| u.scheme() == "file") {
                    handle_url(app, url);
                }
            }
            _ => {}
        });
}
