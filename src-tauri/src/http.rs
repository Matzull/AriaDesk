//! HTTP requests on behalf of the frontend, avoiding CORS restrictions.

use std::time::Duration;

use serde_json::{json, Map, Value};

pub async fn request(ctx: Value, user_agent: &str) -> Value {
    let url = ctx["url"].as_str().unwrap_or_default().to_owned();
    let method = ctx["method"].as_str().unwrap_or("GET").to_uppercase();
    let timeout = ctx["timeout"].as_u64().filter(|&t| t > 0);

    let mut config = Map::new();
    config.insert("method".into(), json!(method));
    config.insert("url".into(), json!(url));
    if let Some(t) = timeout {
        config.insert("timeout".into(), json!(t));
    }
    if method == "POST" {
        config.insert("data".into(), ctx["data"].clone());
    }
    config.insert("headers".into(), ctx["headers"].clone());

    let client = reqwest::Client::new();
    let mut req = client
        .request(reqwest::Method::from_bytes(method.as_bytes()).unwrap_or(reqwest::Method::GET), &url)
        .header(reqwest::header::USER_AGENT, user_agent);
    if let Some(t) = timeout {
        req = req.timeout(Duration::from_millis(t));
    }
    if let Some(headers) = ctx["headers"].as_object() {
        for (k, v) in headers {
            if let Some(v) = v.as_str() {
                req = req.header(k, v);
            }
        }
    }
    if method == "POST" {
        req = match &ctx["data"] {
            Value::String(s) => req.body(s.clone()),
            Value::Null => req,
            data => req.json(data),
        };
    }

    let (success, data, status, status_text) = match req.send().await {
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            let data = serde_json::from_str(&text).unwrap_or(if text.is_empty() { Value::Null } else { Value::String(text) });
            (status.is_success(), data, status.as_u16() as i64, status.canonical_reason().unwrap_or("").to_owned())
        }
        Err(_) => (false, Value::Null, -1, String::new()),
    };

    json!({
        "success": success,
        "response": { "data": data, "status": status, "statusText": status_text, "config": config }
    })
}
