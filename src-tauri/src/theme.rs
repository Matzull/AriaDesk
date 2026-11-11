//! Window theme. On Linux, "follow the system" cannot be left to the window (a `None` theme turns
//! GTK's dark preference off), so the system preference is read from the XDG desktop portal,
//! falling back to the GTK setting found at startup, and portal changes are applied live.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Manager, Theme};

/// Whether the user chose to follow the system theme (as opposed to forcing light or dark).
static FOLLOW_SYSTEM: AtomicBool = AtomicBool::new(true);

/// Applies "dark", "light" or anything else for the system theme.
pub fn set(app: &AppHandle, mode: &str) {
    let theme = match mode {
        "dark" => Some(Theme::Dark),
        "light" => Some(Theme::Light),
        _ => None,
    };
    FOLLOW_SYSTEM.store(theme.is_none(), Ordering::Relaxed);
    apply(app, theme.or_else(system));
}

fn apply(app: &AppHandle, theme: Option<Theme>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_theme(theme);
    }
}

#[cfg(not(target_os = "linux"))]
fn system() -> Option<Theme> {
    None
}

#[cfg(not(target_os = "linux"))]
pub fn init(_app: &AppHandle) {}

#[cfg(target_os = "linux")]
pub use linux::{init, system};

#[cfg(target_os = "linux")]
mod linux {
    use std::sync::OnceLock;

    use gio::prelude::*;
    use gtk::prelude::*;
    use tauri::{AppHandle, Theme};

    const PORTAL: &str = "org.freedesktop.portal.Desktop";
    const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";
    const SETTINGS: &str = "org.freedesktop.portal.Settings";

    /// GTK's dark preference before AriaDesk changes it.
    static GTK_PREFERS_DARK: OnceLock<bool> = OnceLock::new();

    fn to_theme(dark: bool) -> Theme {
        if dark { Theme::Dark } else { Theme::Light }
    }

    /// The portal's color-scheme: 1 prefers dark, 2 prefers light, 0 has no preference.
    fn color_scheme(value: &gio::glib::Variant) -> Option<u32> {
        let mut value = value.clone();
        // ReadOne returns (v); the older Read returns (v) holding another v
        while let Some(inner) = value.as_variant().or_else(|| value.try_child_value(0)) {
            value = inner;
        }
        value.get::<u32>()
    }

    fn portal_prefers_dark() -> Option<bool> {
        let bus = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE).ok()?;
        let reply = ["ReadOne", "Read"].iter().find_map(|method| {
            bus.call_sync(
                Some(PORTAL),
                PORTAL_PATH,
                SETTINGS,
                method,
                Some(&("org.freedesktop.appearance", "color-scheme").to_variant()),
                None,
                gio::DBusCallFlags::NONE,
                1000,
                gio::Cancellable::NONE,
            )
            .ok()
        })?;
        match color_scheme(&reply)? {
            1 => Some(true),
            2 => Some(false),
            _ => None,
        }
    }

    pub fn system() -> Option<Theme> {
        let dark = portal_prefers_dark().or_else(|| GTK_PREFERS_DARK.get().copied())?;
        Some(to_theme(dark))
    }

    /// Records the GTK preference and follows portal changes. Must run on the main thread.
    pub fn init(app: &AppHandle) {
        if let Some(settings) = gtk::Settings::default() {
            let _ = GTK_PREFERS_DARK.set(settings.is_gtk_application_prefer_dark_theme());
        }
        let Ok(bus) = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE) else { return };
        let app = app.clone();
        #[allow(deprecated)] // the replacement needs a newer gio than the one Tauri uses
        bus.signal_subscribe(
            Some(PORTAL),
            Some(SETTINGS),
            Some("SettingChanged"),
            Some(PORTAL_PATH),
            None,
            gio::DBusSignalFlags::NONE,
            move |_, _, _, _, _, params| {
                let namespace = params.try_child_value(0).and_then(|v| v.get::<String>());
                let key = params.try_child_value(1).and_then(|v| v.get::<String>());
                if namespace.as_deref() != Some("org.freedesktop.appearance") || key.as_deref() != Some("color-scheme") {
                    return;
                }
                if super::FOLLOW_SYSTEM.load(std::sync::atomic::Ordering::Relaxed) {
                    let theme = params.try_child_value(2).as_ref().and_then(color_scheme).and_then(|n| match n {
                        1 => Some(to_theme(true)),
                        2 => Some(to_theme(false)),
                        _ => None,
                    });
                    super::apply(&app, theme.or_else(system));
                }
            },
        );
    }
}
