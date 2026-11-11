//! Persistent user settings.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub width: f64,
    pub height: f64,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub maximized: bool,
    pub default_position: String,
    pub minimized_to_tray: bool,
    pub default_magnet_protocol_client: bool,
    pub exec_command_on_startup: Option<String>,
    pub exec_command_arguments_on_startup: Option<String>,
    pub exec_detached_command_on_startup: bool,
    pub auto_start_aria2: bool,
    pub last_check_updates_time: i64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            width: 950.0,
            height: 600.0,
            x: None,
            y: None,
            maximized: false,
            default_position: "last-position".into(),
            minimized_to_tray: true,
            default_magnet_protocol_client: false,
            exec_command_on_startup: None,
            exec_command_arguments_on_startup: None,
            exec_detached_command_on_startup: false,
            auto_start_aria2: false,
            last_check_updates_time: 0,
        }
    }
}

fn config_path(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("config.json"))
}

impl Config {
    pub fn load(app: &AppHandle) -> Self {
        let mut config: Config = config_path(app)
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        config.width = config.width.max(800.0);
        config.height = config.height.max(400.0);
        config
    }

    pub fn save(&self, app: &AppHandle) {
        let Some(path) = config_path(app) else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, content);
        }
    }

    /// The subset of settings exposed to the frontend (`render-sync-get-native-config`).
    pub fn native_config(&self) -> serde_json::Value {
        serde_json::json!({
            "defaultPosition": self.default_position,
            "minimizedToTray": self.minimized_to_tray,
            "execCommandOnStartup": self.exec_command_on_startup,
            "execCommandArgumentsOnStartup": self.exec_command_arguments_on_startup,
            "execDetachedCommandOnStartup": self.exec_detached_command_on_startup,
            "defaultMagnetProtocolClient": self.default_magnet_protocol_client,
            "autoStartAria2": self.auto_start_aria2,
        })
    }
}
