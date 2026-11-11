//! Window icon on Linux. Wayland compositors take it from the .desktop file named after the
//! window's app id (the binary name), so when AriaDesk runs without a package that installed one
//! (development builds, AppImage, a copied binary) a user entry and the icon are written to
//! ~/.local/share. Entries written by AriaDesk carry a marker; any other entry is left alone.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

const APP_ID: &str = "ariadesk";
const MARKER: &str = "X-AriaDesk-Generated=true";
const ICON: &[u8] = include_bytes!("../icons/128x128@2x.png");

/// Whether a package installed a desktop entry for AriaDesk.
fn packaged() -> bool {
    let dirs = std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    std::env::split_paths(&dirs)
        .any(|dir| ["ariadesk.desktop", "AriaDesk.desktop"].iter().any(|name| dir.join("applications").join(name).is_file()))
}

/// The command that starts this AriaDesk: the AppImage itself rather than its mounted copy.
fn executable() -> Option<PathBuf> {
    std::env::var_os("APPIMAGE").map(PathBuf::from).or_else(|| std::env::current_exe().ok())
}

fn write_if_changed(path: &Path, content: &[u8]) -> std::io::Result<()> {
    if std::fs::read(path).is_ok_and(|current| current == content) {
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, content)
}

pub fn ensure(app: &AppHandle) {
    if packaged() {
        return;
    }
    let (Ok(data), Some(exe)) = (app.path().data_dir(), executable()) else { return };
    let entry_path = data.join("applications").join(format!("{APP_ID}.desktop"));
    if std::fs::read_to_string(&entry_path).is_ok_and(|content| !content.contains(MARKER)) {
        return;
    }
    // an absolute path: desktop shells cache failed theme lookups made before the icon existed
    let icon_path = data.join("icons/hicolor/256x256/apps").join(format!("{APP_ID}.png"));
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName=AriaDesk\nComment=Desktop client for aria2\n\
         Exec=\"{}\" %U\nIcon={}\nStartupWMClass={APP_ID}\nCategories=Network;FileTransfer;\n\
         Terminal=false\nNoDisplay={}\n{MARKER}\n",
        exe.display(),
        icon_path.display(),
        cfg!(debug_assertions),
    );
    let _ = write_if_changed(&icon_path, ICON);
    let _ = write_if_changed(&entry_path, entry.as_bytes());
}
