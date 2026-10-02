//! Local HTTP server receiving Claude Code hook events (127.0.0.1 only).

use std::io::Read;
use std::sync::Arc;
use std::time::Instant;

use tauri::{AppHandle, Manager};
use tiny_http::{Header, Method, Response, Server};

use crate::AppState;
use crate::claude_settings::HOOK_PATH;
use crate::sessions::HookEvent;

/// Hook payloads embed tool inputs (whole files for `Write`): cap what we read.
const MAX_BODY: u64 = 16 * 1024 * 1024;

/// Starts listening on `port`. The returned handle stops the server with `unblock()`.
pub fn start(app: AppHandle, port: u16, token: String) -> Result<Arc<Server>, String> {
    let server = Arc::new(Server::http(("127.0.0.1", port)).map_err(|e| e.to_string())?);
    log::info!("hook server listening on 127.0.0.1:{port}");
    let expected = format!("Bearer {token}");

    let listener = Arc::clone(&server);
    std::thread::spawn(move || {
        // Ends when `unblock()` is called (port change).
        for mut request in listener.incoming_requests() {
            let status = match (request.method(), request.url()) {
                (Method::Get, "/winotch/health") => 200,
                (Method::Post, HOOK_PATH) if !authorized(&request, &expected) => 401,
                (Method::Post, HOOK_PATH) => {
                    let mut body = Vec::new();
                    let read = request.as_reader().take(MAX_BODY).read_to_end(&mut body);
                    match read
                        .ok()
                        .and_then(|_| serde_json::from_slice::<HookEvent>(&body).ok())
                    {
                        Some(event) => {
                            handle(&app, &event);
                            200
                        }
                        None => 400,
                    }
                }
                _ => 404,
            };
            // An empty 2xx body means "no decision": Claude Code carries on as usual.
            let _ = request.respond(Response::empty(status));
        }
        log::info!("hook server on port {port} stopped");
    });
    Ok(server)
}

fn authorized(request: &tiny_http::Request, expected: &str) -> bool {
    request
        .headers()
        .iter()
        .find(|h: &&Header| h.field.equiv("Authorization"))
        .is_some_and(|h| h.value.as_str() == expected)
}

fn handle(app: &AppHandle, event: &HookEvent) {
    let state = app.state::<AppState>();
    let changed = state.sessions.lock().unwrap().apply(event, Instant::now());
    if changed {
        crate::emit_sessions(app);
    }
}
