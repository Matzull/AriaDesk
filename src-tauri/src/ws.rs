//! Single WebSocket connection to the aria2 RPC, owned by the backend (port of lib/websocket.js).
//! Messages sent before the socket is open are queued and flushed on open.

use std::sync::Mutex;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tauri::AppHandle;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio_tungstenite::tungstenite::Message;

use crate::emit;

struct Connection {
    url: String,
    reconnect_interval: u64,
    tx: UnboundedSender<String>,
}

#[derive(Default)]
pub struct WebSocket {
    generation: Mutex<u64>,
    current: Mutex<Option<Connection>>,
}

impl WebSocket {
    /// Drops the current connection and pending messages.
    pub fn reset(&self) {
        *self.generation.lock().unwrap() += 1;
        *self.current.lock().unwrap() = None;
    }

    pub fn connect(&'static self, app: AppHandle, url: String, options: &Value) {
        let reconnect_interval = options["reconnectInterval"].as_u64().unwrap_or(0);
        self.start(app, url, reconnect_interval);
    }

    pub fn reconnect(&'static self, app: AppHandle) {
        let current = self.current.lock().unwrap().as_ref().map(|c| (c.url.clone(), c.reconnect_interval));
        if let Some((url, interval)) = current {
            self.start(app, url, interval);
        }
    }

    pub fn send(&self, app: &AppHandle, url: &str, data: String) {
        let sent = self.current.lock().unwrap().as_ref().is_some_and(|c| c.tx.send(data.clone()).is_ok());
        if !sent {
            emit(app, "on-main-websocket-message", json!([{ "success": false, "url": url, "request": data, "data": null }]));
        }
    }

    fn start(&'static self, app: AppHandle, url: String, reconnect_interval: u64) {
        let (tx, mut rx) = unbounded_channel::<String>();
        let generation = {
            let mut g = self.generation.lock().unwrap();
            *g += 1;
            *g
        };
        *self.current.lock().unwrap() = Some(Connection { url: url.clone(), reconnect_interval, tx });
        let is_current = move || *self.generation.lock().unwrap() == generation;

        tauri::async_runtime::spawn(async move {
            if let Ok((stream, _)) = tokio_tungstenite::connect_async(url.as_str()).await {
                if !is_current() {
                    return;
                }
                emit(&app, "on-main-websocket-open", json!([{ "url": url }]));
                let (mut sink, mut source) = stream.split();
                loop {
                    tokio::select! {
                        outgoing = rx.recv() => match outgoing {
                            Some(text) => if sink.send(Message::text(text)).await.is_err() { break },
                            None => { let _ = sink.close().await; return; }
                        },
                        incoming = source.next() => match incoming {
                            Some(Ok(Message::Text(text))) => emit(&app, "on-main-websocket-message",
                                json!([{ "success": true, "url": url, "data": text.as_str() }])),
                            Some(Ok(_)) => {}
                            _ => break,
                        },
                    }
                }
            }
            if !is_current() {
                return;
            }
            let auto_reconnect = reconnect_interval > 0;
            emit(&app, "on-main-websocket-close", json!([{ "url": url, "autoReconnect": auto_reconnect }]));
            if auto_reconnect {
                emit(&app, "on-main-log-debug", json!([format!("[ws] next reconnection is pending in {reconnect_interval}ms")]));
                tokio::time::sleep(Duration::from_millis(reconnect_interval)).await;
                if is_current() {
                    self.start(app, url, reconnect_interval);
                }
            }
        });
    }
}
