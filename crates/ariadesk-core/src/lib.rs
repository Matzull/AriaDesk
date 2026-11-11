//! Platform independent logic shared by the desktop app and the web build (via wasm).

pub mod bencode;
mod torrent;

pub use torrent::{parse_bittorrent_info, TorrentFile, TorrentInfo};

pub fn supported_file_extension(ext: &str) -> Option<&'static str> {
    match ext.to_ascii_lowercase().as_str() {
        ".torrent" => Some("torrent"),
        ".meta4" | ".metalink" => Some("metalink"),
        _ => None,
    }
}

#[cfg(feature = "wasm")]
mod wasm {
    use wasm_bindgen::prelude::*;

    /// Returns the torrent info as a JSON string, or `undefined` when the data is invalid.
    #[wasm_bindgen(js_name = parseBittorrentInfo)]
    pub fn parse_bittorrent_info(base64: &str, sep: &str) -> Option<String> {
        let sep = sep.chars().next().unwrap_or('/');
        let info = super::parse_bittorrent_info(base64, sep)?;
        serde_json::to_string(&info).ok()
    }

    #[wasm_bindgen(js_name = supportedFileExtension)]
    pub fn supported_file_extension(ext: &str) -> Option<String> {
        super::supported_file_extension(ext).map(str::to_owned)
    }
}
