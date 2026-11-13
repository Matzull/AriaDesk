//! Moves the data of versions that used the identifier io.github.ariadesk (config, aria2 session,
//! web view storage) to the directories of the current identifier.

use tauri::{AppHandle, Manager};

const OLD_IDENTIFIER: &str = "io.github.ariadesk";

pub fn run(app: &AppHandle) {
    let path = app.path();
    let dirs = [path.app_config_dir(), path.app_data_dir(), path.app_local_data_dir(), path.app_cache_dir()];
    for dir in dirs.into_iter().flatten() {
        let old = dir.with_file_name(OLD_IDENTIFIER);
        if old != dir && old.is_dir() && !dir.exists() {
            let _ = std::fs::rename(&old, &dir);
        }
    }
}
