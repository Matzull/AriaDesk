//! Starts aria2c in the background when the user asks for it, and stops it when AriaDesk exits.
//! Unfinished downloads are kept in a session file in the app data directory, so they resume on the next start.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

const EXECUTABLE: &str = if cfg!(windows) { "aria2c.exe" } else { "aria2c" };
const STDERR_LINES: usize = 20;

struct Running {
    child: Child,
    port: u16,
    secret: String,
}

#[derive(Default)]
pub struct Aria2 {
    running: Mutex<Option<Running>>,
}

/// Looks for aria2c next to the AriaDesk executable, then in PATH.
fn find_executable() -> Option<PathBuf> {
    let beside = std::env::current_exe().ok().and_then(|exe| exe.parent().map(|dir| dir.join(EXECUTABLE)));
    let path = std::env::var_os("PATH").unwrap_or_default();
    beside.into_iter().chain(std::env::split_paths(&path).map(|dir| dir.join(EXECUTABLE))).find(|p| p.is_file())
}

/// The configuration file aria2 would load by itself, which `--conf-path` replaces.
fn user_config(app: &AppHandle) -> Option<String> {
    let home = app.path().home_dir().ok()?;
    let xdg = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".config"));
    [home.join(".aria2").join("aria2.conf"), xdg.join("aria2").join("aria2.conf")]
        .iter()
        .find_map(|p| std::fs::read_to_string(p).ok())
}

/// Writes the configuration readable only by the user, so the secret is not visible in the process list.
fn write_private(path: &Path, content: &str) -> std::io::Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        options.mode(0o600);
        if path.exists() {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        }
    }
    std::io::Write::write_all(&mut options.open(path)?, content.as_bytes())
}

/// Asks aria2 to shut down, which saves the session, and kills it if it does not exit in time.
fn shutdown(mut r: Running) {
    let params = if r.secret.is_empty() { json!([]) } else { json!([format!("token:{}", r.secret)]) };
    let body = json!({ "jsonrpc": "2.0", "id": "ariadesk", "method": "aria2.forceShutdown", "params": params });
    let url = format!("http://127.0.0.1:{}/jsonrpc", r.port);
    let _ = tauri::async_runtime::block_on(async {
        reqwest::Client::new().post(url).json(&body).timeout(Duration::from_secs(2)).send().await
    });
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        if r.child.try_wait().ok().flatten().is_some() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let _ = r.child.kill();
    let _ = r.child.wait();
}

impl Aria2 {
    /// Returns `{ "ok": true }`, or `{ "error": "not-found" | <message> }`.
    pub fn start(&self, app: &AppHandle, port: u16, secret: &str) -> Value {
        let mut running = self.running.lock().unwrap();
        if let Some(mut r) = running.take() {
            let alive = r.child.try_wait().ok().flatten().is_none();
            if alive && r.port == port && r.secret == secret {
                *running = Some(r);
                return json!({ "ok": true });
            }
            if alive {
                // started with another port or secret: restart it with the new ones
                shutdown(r);
            }
        }
        let Some(executable) = find_executable() else {
            return json!({ "error": "not-found" });
        };
        let Ok(data_dir) = app.path().app_data_dir() else {
            return json!({ "error": "no data directory" });
        };
        let _ = std::fs::create_dir_all(&data_dir);
        let session = data_dir.join("aria2.session");
        if !session.exists() {
            let _ = std::fs::write(&session, "");
        }
        let mut config = user_config(app).unwrap_or_default();
        let has_dir = config.lines().any(|l| l.trim_start().starts_with("dir="));
        if !secret.is_empty() {
            config.push_str(&format!("\nrpc-secret={secret}\n"));
        }
        let config_path = data_dir.join("aria2.conf");
        if let Err(e) = write_private(&config_path, &config) {
            return json!({ "error": e.to_string() });
        }
        let download_dir = app.path().download_dir().or_else(|_| app.path().home_dir()).unwrap_or_default();

        let mut cmd = Command::new(executable);
        cmd.arg(format!("--conf-path={}", config_path.display()))
            .arg("--enable-rpc")
            .arg(format!("--rpc-listen-port={port}"))
            .arg(format!("--input-file={}", session.display()))
            .arg(format!("--save-session={}", session.display()))
            .arg("--save-session-interval=30")
            .arg("--continue=true")
            // aria2 also stops by itself if AriaDesk dies without stopping it
            .arg(format!("--stop-with-process={}", std::process::id()))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        if !has_dir {
            cmd.arg(format!("--dir={}", download_dir.display()));
        }
        #[cfg(windows)]
        std::os::windows::process::CommandExt::creation_flags(&mut cmd, 0x0800_0000); // CREATE_NO_WINDOW

        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => return json!({ "error": e.to_string() }),
        };
        // stderr is read for as long as aria2 runs, so writing to it never fails or blocks;
        // the last lines explain an early exit
        let stderr_lines = Arc::new(Mutex::new(Vec::<String>::new()));
        let reader = child.stderr.take().map(|stderr| {
            let lines = stderr_lines.clone();
            std::thread::spawn(move || {
                for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                    let mut lines = lines.lock().unwrap();
                    if lines.len() == STDERR_LINES {
                        lines.remove(0);
                    }
                    lines.push(line);
                }
            })
        });
        // aria2 exits right away on bad options or when the port is taken
        std::thread::sleep(Duration::from_millis(400));
        if let Ok(Some(status)) = child.try_wait() {
            if let Some(reader) = reader {
                let _ = reader.join();
            }
            let lines = stderr_lines.lock().unwrap();
            let message = lines.iter().rev().find(|l| !l.trim().is_empty()).map(|l| l.trim().to_owned());
            return json!({ "error": message.unwrap_or_else(|| format!("aria2c exited ({status})")) });
        }
        *running = Some(Running { child, port, secret: secret.to_owned() });
        json!({ "ok": true })
    }

    pub fn stop(&self) {
        if let Some(r) = self.running.lock().unwrap().take() {
            shutdown(r);
        }
    }
}
