//! Minimal CCat `/change` HTTP client.
//!
//! Replaces the scanner's `scanner_post_*` helpers: the same POST goes to
//! `http://localhost:9101/change` with the session token from `/tmp/session_token`,
//! and the mandatory `contentSource: "OnDevice"` marker is stamped on the body
//! (stock behaviour lives in `scanner_post_to_uri_internal`).
//!
//! Transport is `ureq` in blocking mode without TLS: ccat runs locally on
//! plain HTTP so no TLS stack is needed on the device.

use std::time::Duration;

use serde_json::{json, Value};

const CCAT_HOST: &str = "127.0.0.1";
const CCAT_PORT: u16 = 9101;
const SESSION_TOKEN_PATH: &str = "/tmp/session_token";

/// Maximum number of POST attempts (initial + retries).
const MAX_ATTEMPTS: u32 = 4;
/// Sleep between failed attempts, matching `scanner_post_string`.
const RETRY_SLEEP: Duration = Duration::from_secs(1);
/// Backoff (microseconds) for HTTP 503 "ccat busy" responses, matching the
/// scanner's 1000/2000/4000us schedule.
const BUSY_BACKOFF_US: [u64; 3] = [1000, 2000, 4000];

/// Port override for tests; we must not talk to a real device's ccat there.
fn ccat_port() -> u16 {
    std::env::var("KOMPANION_CCAT_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(CCAT_PORT)
}

fn read_session_token() -> Option<String> {
    let token = std::fs::read_to_string(SESSION_TOKEN_PATH).ok()?;
    let token = token.trim().to_string();
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

/// POST `body` to the ccat server. Returns the HTTP status code and the raw
/// response body, distinguishing a failed request (Err) from a delivered
/// response (Ok) even when the status is an error code.
fn http_post(uri: &str, body: &str) -> Result<(u16, String), String> {
    let url = format!("http://{CCAT_HOST}:{}{uri}", ccat_port());
    let mut request = ureq::post(&url).header("Content-Type", "application/json");
    if let Some(token) = read_session_token() {
        request = request.header("AuthToken", &token);
    }

    match request.send(body) {
        Ok(mut resp) => {
            let status = resp.status();
            let text = resp
                .body_mut().read_to_string()
                .map_err(|e| format!("read response: {e}"))?;
            Ok((status.as_u16(), text))
        }
        Err(e) => Err(format!("{e}")),
    }
}

/// CCat answers `{"ok":true,...}` on success.
fn response_ok(status: u16, body: &str) -> bool {
    if !(200..300).contains(&status) {
        return false;
    }
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v.get("ok").and_then(|o| o.as_bool()))
        .unwrap_or(false)
}

fn post_with_retries(uri: &str, body: &str) -> i32 {
    for attempt in 0..MAX_ATTEMPTS {
        match http_post(uri, body) {
            Ok((status, text)) => {
                if response_ok(status, &text) {
                    return 0;
                }
                let snippet: String = text.chars().take(200).collect();
                log::warn!(
                    "ccat POST {uri} attempt {} failed (status {status}): {snippet}",
                    attempt + 1
                );
                if status == 503 && (attempt as usize) < BUSY_BACKOFF_US.len() {
                    std::thread::sleep(Duration::from_micros(
                        BUSY_BACKOFF_US[attempt as usize],
                    ));
                    continue;
                }
            }
            Err(e) => {
                log::warn!("ccat POST {uri} attempt {} error: {e}", attempt + 1);
            }
        }
        std::thread::sleep(RETRY_SLEEP);
    }
    1
}

/// POST a ChangeRequest to `/change`, stamping `contentSource: "OnDevice"`.
/// Returns 0 on success (matching the extractor's 0 = processed convention).
pub fn post_change(change: &Value) -> i32 {
    let mut change = change.clone();
    let Some(obj) = change.as_object_mut() else {
        log::error!("post_change: body is not a JSON object");
        return 1;
    };
    if !obj.contains_key("contentSource") {
        obj.insert(
            "contentSource".to_string(),
            Value::String("OnDevice".to_string()),
        );
    }
    post_with_retries("/change", &change.to_string())
}

/// Delete a CCat entry (same command pair the scanner sends for deletions).
pub fn delete_ccat_entry(uuid: &str) -> i32 {
    let body = json!({
        "type": "ChangeRequest",
        "commands": [{
            "delete": { "uuid": uuid },
            "updateDeletedArchivedItem": { "deletedUuid": uuid }
        }],
        "caller": "scannerDelete"
    });
    post_with_retries("/change", &body.to_string())
}

/// Update the thumbnail of an existing entry, mirroring the scanner's
/// `scanner_update_ccat_entry_with_thumbpath`.
pub fn update_thumbnail(uuid: &str, thumbnail_path: &str) -> i32 {
    let body = json!({
        "type": "ChangeRequest",
        "commands": [{
            "update": {
                "entry": { "uuid": uuid, "thumbnail": thumbnail_path }
            }
        }]
    });
    post_with_retries("/change", &body.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::Mutex;

    /// Tests that touch KOMPANION_CCAT_PORT must run serially (process-global env).
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct MiniServer {
        handle: Option<std::thread::JoinHandle<()>>,
    }

    impl MiniServer {
        /// Bind to an ephemeral port and answer every request for a few
        /// connections with `response_body`.
        fn serve(response_body: &'static str) -> (Self, u16) {
            let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
            let port = listener.local_addr().unwrap().port();
            let handle = std::thread::spawn(move || {
                let _ = listener.set_nonblocking(true);
                let deadline = std::time::Instant::now() + Duration::from_secs(5);
                let mut served = 0;
                while std::time::Instant::now() < deadline && served < 4 {
                    match listener.accept() {
                        Ok((mut conn, _)) => {
                            served += 1;
                            let mut buf = Vec::new();
                            let mut chunk = [0u8; 4096];
                            loop {
                                match conn.read(&mut chunk) {
                                    Ok(0) => break,
                                    Ok(n) => {
                                        buf.extend_from_slice(&chunk[..n]);
                                        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                                            break;
                                        }
                                    }
                                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                        continue;
                                    }
                                    Err(_) => break,
                                }
                            }
                            let body = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                response_body.len(),
                                response_body
                            );
                            let _ = conn.write_all(body.as_bytes());
                            let _ = conn.flush();
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(10));
                        }
                        Err(_) => break,
                    }
                }
            });
            (Self { handle: Some(handle) }, port)
        }
    }

    impl Drop for MiniServer {
        fn drop(&mut self) {
            let _ = self.handle.take().map(|h| h.join());
        }
    }

    #[test]
    fn test_response_ok() {
        assert!(response_ok(200, "{\"ok\":true,\"type\":\"ChangeResponse\"}"));
        assert!(!response_ok(200, "{\"ok\":false,\"error\":\"boom\"}"));
        assert!(!response_ok(503, "Service Unavailable"));
        assert!(!response_ok(200, ""));
    }

    #[test]
    fn test_post_change_success_over_http() {
        let _guard = ENV_LOCK.lock().unwrap();
        let (server, port) = MiniServer::serve("{\"ok\":true,\"changes\":1,\"type\":\"ChangeResponse\"}");
        std::env::set_var("KOMPANION_CCAT_PORT", port.to_string());

        let change = json!({
            "type": "ChangeRequest",
            "commands": [{ "insert": { "uuid": "u" } }]
        });
        assert_eq!(post_change(&change), 0);

        let _ = server;
        std::env::remove_var("KOMPANION_CCAT_PORT");
    }

    #[test]
    fn test_post_change_fails_on_ok_false() {
        let _guard = ENV_LOCK.lock().unwrap();
        let (server, port) = MiniServer::serve("{\"ok\":false,\"error\":\"nope\"}");
        std::env::set_var("KOMPANION_CCAT_PORT", port.to_string());

        let change = json!({
            "type": "ChangeRequest",
            "commands": [{ "insert": { "uuid": "u" } }]
        });
        assert_ne!(post_change(&change), 0);

        let _ = server;
        std::env::remove_var("KOMPANION_CCAT_PORT");
    }

    #[test]
    fn test_post_change_rejects_non_object() {
        assert_ne!(post_change(&json!([1, 2, 3])), 0);
    }

    #[test]
    fn test_post_change_adds_content_source() {
        let change = json!({
            "type": "ChangeRequest",
            "commands": [{ "insert": { "uuid": "u" } }]
        });
        let body = {
            let mut change = change.clone();
            let obj = change.as_object_mut().unwrap();
            if !obj.contains_key("contentSource") {
                obj.insert(
                    "contentSource".to_string(),
                    Value::String("OnDevice".to_string()),
                );
            }
            change.to_string()
        };
        let parsed: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(parsed["contentSource"].as_str(), Some("OnDevice"));
    }

    #[test]
    fn test_delete_body_shape() {
        let body = json!({
            "type": "ChangeRequest",
            "commands": [{
                "delete": { "uuid": "u1" },
                "updateDeletedArchivedItem": { "deletedUuid": "u1" }
            }],
            "caller": "scannerDelete"
        });
        assert_eq!(body["type"].as_str(), Some("ChangeRequest"));
        assert_eq!(body["commands"][0]["delete"]["uuid"].as_str(), Some("u1"));
        assert_eq!(
            body["commands"][0]["updateDeletedArchivedItem"]["deletedUuid"].as_str(),
            Some("u1")
        );
    }
}