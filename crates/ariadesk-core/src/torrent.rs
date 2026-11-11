use base64::Engine;
use serde::Serialize;
use sha1::{Digest, Sha1};

use crate::bencode::{self, Node};

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TorrentFile {
    pub path: String,
    pub name: String,
    pub length: i64,
    pub offset: i64,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TorrentInfo {
    pub name: String,
    pub info_hash: String,
    pub piece_length: i64,
    pub files: Vec<TorrentFile>,
    pub length: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private: Option<bool>,
    /// Creation time in unix milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub announce: Option<Vec<String>>,
}

fn utf8_or<'a, 'b>(node: &'b Node<'a>, key: &str) -> Option<&'b Node<'a>> {
    node.get(&format!("{key}.utf-8")).or_else(|| node.get(key))
}

fn parse_file(file: &Node, torrent_name: &str, offset: i64, sep: char) -> Option<TorrentFile> {
    let mut parts = vec![torrent_name.to_owned()];
    if let Some(path) = utf8_or(file, "path") {
        match path.as_list() {
            Some(list) => parts.extend(list.iter().map(|p| p.as_str().unwrap_or_default())),
            None => parts.push(path.as_str()?),
        }
    }
    let path = parts.iter().filter(|p| !p.is_empty()).cloned().collect::<Vec<_>>().join(&sep.to_string());
    Some(TorrentFile {
        path,
        name: parts.last().cloned().unwrap_or_default(),
        length: file.get("length").and_then(Node::as_int).unwrap_or_default(),
        offset,
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Decodes a base64 .torrent and extracts its name, info hash, file list and metadata (BEP 3).
/// File paths are joined with `sep`. Returns `None` when the data is not a valid torrent.
pub fn parse_bittorrent_info(base64_data: &str, sep: char) -> Option<TorrentInfo> {
    let data = base64::engine::general_purpose::STANDARD.decode(base64_data.trim()).ok()?;
    let torrent = bencode::decode(&data)?;
    let info = torrent.get("info")?;

    let name = utf8_or(info, "name")?.as_str()?;
    let piece_length = info.get("piece length")?.as_int().filter(|&n| n != 0)?;
    info.get("pieces")?.as_bytes().filter(|p| !p.is_empty())?;

    let mut files = Vec::new();
    let mut length = 0i64;

    match info.get("files").and_then(Node::as_list).filter(|l| !l.is_empty()) {
        Some(list) => {
            for file in list {
                let file_length = file.get("length").and_then(Node::as_int)?;
                utf8_or(file, "path")?;
                files.push(parse_file(file, &name, length, sep)?);
                length += file_length;
            }
        }
        None => {
            let single = info.get("length").and_then(Node::as_int).filter(|&n| n != 0)?;
            files.push(parse_file(info, &name, 0, sep)?);
            length = single;
        }
    }

    let announce = match torrent.get("announce-list").and_then(Node::as_list).filter(|l| !l.is_empty()) {
        Some(tiers) => {
            let mut urls: Vec<String> = Vec::new();
            for url in tiers.iter().flat_map(|t| t.as_list().unwrap_or_default()) {
                let url = url.as_str()?;
                if !urls.contains(&url) {
                    urls.push(url);
                }
            }
            Some(urls)
        }
        None => torrent.get("announce").and_then(Node::as_str).map(|a| vec![a]),
    };

    Some(TorrentInfo {
        name,
        info_hash: hex(&Sha1::digest(info.raw)),
        piece_length,
        files,
        length,
        private: info.get("private").map(|p| p.as_int().is_none_or(|i| i != 0)),
        created: torrent.get("creation date").and_then(Node::as_int).filter(|&d| d != 0).map(|d| d * 1000),
        comment: utf8_or(&torrent, "comment").and_then(Node::as_str),
        announce,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bencode::enc::{encode, s, E};

    fn b64(e: &E) -> (String, Vec<u8>) {
        let mut out = Vec::new();
        encode(e, &mut out);
        (base64::engine::general_purpose::STANDARD.encode(&out), out)
    }

    fn info_hash(info: &E) -> String {
        let mut out = Vec::new();
        encode(info, &mut out);
        hex(&Sha1::digest(&out))
    }

    fn single_info() -> E {
        E::D(vec![
            ("name", s("file.iso")),
            ("piece length", E::I(16384)),
            ("pieces", E::B(vec![0; 20])),
            ("length", E::I(1000)),
        ])
    }

    #[test]
    fn single_file() {
        let (data, _) = b64(&E::D(vec![
            ("announce", s("http://t/a")),
            ("creation date", E::I(10)),
            ("comment", s("hi")),
            ("info", single_info()),
        ]));
        let info = parse_bittorrent_info(&data, '/').unwrap();
        assert_eq!(info.name, "file.iso");
        assert_eq!(info.info_hash, info_hash(&single_info()));
        assert_eq!(info.length, 1000);
        assert_eq!(info.files, vec![TorrentFile { path: "file.iso".into(), name: "file.iso".into(), length: 1000, offset: 0 }]);
        assert_eq!(info.created, Some(10_000));
        assert_eq!(info.comment.as_deref(), Some("hi"));
        assert_eq!(info.announce, Some(vec!["http://t/a".into()]));
        assert_eq!(info.private, None);
    }

    #[test]
    fn multi_file() {
        let file = |len, path: &[&str]| E::D(vec![("length", E::I(len)), ("path", E::L(path.iter().map(|p| s(p)).collect()))]);
        let info = E::D(vec![
            ("name", s("dir")),
            ("name.utf-8", s("dír")),
            ("piece length", E::I(16384)),
            ("pieces", E::B(vec![1; 20])),
            ("private", E::I(1)),
            ("files", E::L(vec![file(10, &["a", "b.txt"]), file(5, &["c.txt"])])),
        ]);
        let (data, _) = b64(&E::D(vec![
            ("announce-list", E::L(vec![E::L(vec![s("u1"), s("u2")]), E::L(vec![s("u1"), s("u3")])])),
            ("info", info),
        ]));
        let parsed = parse_bittorrent_info(&data, '\\').unwrap();
        assert_eq!(parsed.name, "dír");
        assert_eq!(parsed.length, 15);
        assert_eq!(parsed.files[0].path, "dír\\a\\b.txt");
        assert_eq!(parsed.files[1].offset, 10);
        assert_eq!(parsed.private, Some(true));
        assert_eq!(parsed.announce, Some(vec!["u1".into(), "u2".into(), "u3".into()]));
    }

    #[test]
    fn invalid() {
        assert!(parse_bittorrent_info("not base64!", '/').is_none());
        let (data, _) = b64(&E::D(vec![("info", E::D(vec![("name", s("x"))]))]));
        assert!(parse_bittorrent_info(&data, '/').is_none());
    }
}
