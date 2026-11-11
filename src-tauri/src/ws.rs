//! Single WebSocket connection to the aria2 RPC, owned by the backend.
//! Each connection carries an id chosen by the frontend: events name it, and closing or sending
//! with another id never touches the current connection. Reconnecting is up to the frontend.
//! Messages sent before the socket is open are queued and flushed on open.

use std::sync::Mutex;

use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tauri::AppHandle;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio_tungstenite::tungstenite::Message;

use crate::emit;

struct Connection {
    id: String,
    tx: UnboundedSender<String>,
}

#[derive(Default)]
pub struct WebSocket {
    current: Mutex<Option<Connection>>,
}

impl WebSocket {
    /// Drops the current connection, whatever its id.
    pub fn reset(&self) {
        *self.current.lock().unwrap() = None;
    }

    /// Drops the connection only if it is still the one with this id.
    pub fn close(&self, id: &str) {
        let mut current = self.current.lock().unwrap();
        if current.as_ref().is_some_and(|c| c.id == id) {
            *current = None;
        }
    }

    pub fn send(&self, app: &AppHandle, id: &str, data: String) {
        let sent = self.current.lock().unwrap().as_ref().is_some_and(|c| c.id == id && c.tx.send(data.clone()).is_ok());
        if !sent {
            emit(app, "ws-message", json!([{ "success": false, "id": id, "request": data, "data": null }]));
        }
    }

    pub fn connect(&'static self, app: AppHandle, url: String, id: String) {
        let (tx, mut rx) = unbounded_channel::<String>();
        *self.current.lock().unwrap() = Some(Connection { id: id.clone(), tx });
        let is_current = {
            let id = id.clone();
            move || self.current.lock().unwrap().as_ref().is_some_and(|c| c.id == id)
        };

        tauri::async_runtime::spawn(async move {
            if let Ok((stream, _)) = tokio_tungstenite::connect_async(url.as_str()).await {
                if !is_current() {
                    return;
                }
                emit(&app, "ws-open", json!([{ "id": id }]));
                let (mut sink, mut source) = stream.split();
                loop {
                    tokio::select! {
                        outgoing = rx.recv() => match outgoing {
                            Some(text) => if sink.send(Message::text(text)).await.is_err() { break },
                            // the connection was dropped by close() or reset()
                            None => { let _ = sink.close().await; return; }
                        },
                        incoming = source.next() => match incoming {
                            Some(Ok(Message::Text(text))) => emit(&app, "ws-message",
                                json!([{ "success": true, "id": id, "data": text.as_str() }])),
                            Some(Ok(_)) => {}
                            _ => break,
                        },
                    }
                }
            }
            if is_current() {
                self.close(&id);
                emit(&app, "ws-close", json!([{ "id": id }]));
            }
        });
    }
}
