//! Runs the user configured command on startup and keeps its recent output (port of lib/process.js).

use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

const OUTPUT_LOG_LIMIT: usize = 100;

pub type OutputLog = Arc<Mutex<Vec<Value>>>;

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or_default()
}

fn push(log: &OutputLog, kind: &str, source: &str, content: Value) {
    let mut log = log.lock().unwrap();
    if let Some(last) = log.last_mut() {
        if kind == "output" && last["source"] == source && last["content"] == content {
            last["count"] = json!(last["count"].as_u64().unwrap_or(1) + 1);
            return;
        }
    }
    if log.len() >= OUTPUT_LOG_LIMIT {
        log.remove(0);
    }
    log.push(json!({ "time": now_ms(), "type": kind, "source": source, "content": content, "count": 1 }));
}

fn pipe(log: OutputLog, source: &'static str, stream: impl Read + Send + 'static) {
    std::thread::spawn(move || {
        for line in BufReader::new(stream).lines().map_while(Result::ok) {
            push(&log, "output", source, json!(line));
        }
    });
}

pub fn exec_on_startup(command: &str, args: Option<&str>, detached: bool, log: OutputLog) {
    if command.trim().is_empty() {
        return;
    }
    let args: Vec<&str> = args.unwrap_or_default().split('\n').map(str::trim).filter(|a| !a.is_empty()).collect();

    let mut cmd = Command::new(command);
    cmd.args(&args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    if detached {
        #[cfg(unix)]
        std::os::unix::process::CommandExt::process_group(&mut cmd, 0);
        #[cfg(windows)]
        std::os::windows::process::CommandExt::creation_flags(&mut cmd, 0x0000_0008 | 0x0000_0200); // DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP
    }
    #[cfg(windows)]
    if !detached {
        std::os::windows::process::CommandExt::creation_flags(&mut cmd, 0x0800_0000); // CREATE_NO_WINDOW
    }

    match cmd.spawn() {
        Ok(mut child) => {
            if let Some(out) = child.stdout.take() {
                pipe(log.clone(), "stdout", out);
            }
            if let Some(err) = child.stderr.take() {
                pipe(log.clone(), "stderr", err);
            }
            std::thread::spawn(move || {
                let code = child.wait().ok().and_then(|s| s.code());
                push(&log, "output", "close", json!(code));
            });
        }
        Err(e) => push(&log, "error", "exception", json!(e.to_string())),
    }
}
