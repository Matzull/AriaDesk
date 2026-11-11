//! Command line handling: `ariadesk [file/url] [-d|--development] [-c|--classic] [-m|--minimal]`

use std::path::Path;

#[derive(Debug, Default)]
pub struct Args {
    pub development: bool,
    pub minimal: bool,
    pub file_or_url: Option<String>,
}

pub fn parse<I: IntoIterator<Item = String>>(argv: I) -> Args {
    let mut args = Args::default();
    for arg in argv.into_iter().skip(1) {
        match arg.as_str() {
            "-d" | "--development" => args.development = true,
            "-m" | "--minimal" => args.minimal = true,
            "new" => {}
            a if a.starts_with('-') => {}
            _ => args.file_or_url = Some(arg),
        }
    }
    args
}

pub fn is_supported_file(arg: &str) -> bool {
    let path = Path::new(arg);
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    ariadesk_core::supported_file_extension(&ext).is_some() && path.exists()
}

pub fn is_url(arg: &str) -> bool {
    url::Url::parse(arg).is_ok()
}
