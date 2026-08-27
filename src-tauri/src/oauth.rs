//! Codex OAuth (PKCE) flow + a tiny localhost callback HTTP server.
//!
//! Mirrors the macOS original's `CodexOAuthLogin` / `OAuthCallbackServer`
//! pair (see `Sources/CodexRunwayCore/CodexOAuthLogin.swift` and
//! `Sources/CodexRunwayCore/OAuthCallbackServer.swift`). The webview in the
//! popover loads `auth_url`; once the user signs in, OpenAI redirects to
//! `http://localhost:<port>/auth/callback?code=…&state=…`, our server
//! captures the URL, returns a friendly HTML page, and we exchange the
//! `code` + `verifier` for tokens.
//!
//! The server is a deliberately minimal HTTP/1.1 parser — it only handles a
//! single `GET` request line, which is exactly what OpenAI's redirect does.
//! This keeps the dependency surface flat (no `hyper`, no `axum`).

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::auth::CodexAuth;
use crate::error::{AppError, AppResult};

/// Public client id used by the official Codex CLI. Stable; matches macOS.
pub const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
pub const AUTH_ENDPOINT: &str = "https://auth.openai.com/oauth/authorize";
pub const TOKEN_ENDPOINT: &str = "https://auth.openai.com/oauth/token";
pub const SCOPES: &str = "openid profile email offline_access api.connectors.read api.connectors.invoke";
pub const ORIGINATOR: &str = "codex_cli_rs";
pub const PREFERRED_PORT: u16 = 1455;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthSession {
    pub login_id: String,
    pub auth_url: String,
    pub redirect_uri: String,
    pub code_verifier: String,
    pub state: String,
    pub port: u16,
    pub expires_at_unix: i64,
}

pub struct OAuthLogin;

impl OAuthLogin {
    /// Build a fresh PKCE session. Caller is responsible for actually
    /// starting the callback server and exchanging the code.
    pub fn start_session(port: u16, lifetime_secs: i64) -> AppResult<OAuthSession> {
        let code_verifier = random_base64url(32);
        let challenge = code_challenge(&code_verifier);
        let state = random_base64url(16);
        let login_id = random_base64url(8);
        let redirect_uri = format!("http://localhost:{port}/auth/callback");

        // Build auth URL with query params. Manual encoding is fine because
        // all our values are url-safe base64 / fixed strings.
        let auth_url = format!(
            "{AUTH_ENDPOINT}?response_type=code\
&client_id={CLIENT_ID}\
&redirect_uri={redirect}\
&scope={scopes}\
&code_challenge={challenge}\
&code_challenge_method=S256\
&state={state}\
&id_token_add_organizations=true\
&codex_cli_simplified_flow=true\
&originator={ORIGINATOR}",
            redirect = url_encode(&redirect_uri),
            scopes = url_encode(SCOPES),
            challenge = challenge,
            state = state,
        );

        let now = chrono::Utc::now().timestamp();
        Ok(OAuthSession {
            login_id,
            auth_url,
            redirect_uri,
            code_verifier,
            state,
            port,
            expires_at_unix: now + lifetime_secs,
        })
    }

    /// Extract the `code` query param from a callback URL after verifying
    /// `state`. Returns the authorization code on success.
    pub fn authorization_code(callback_url: &str, expected_state: &str) -> AppResult<String> {
        let (path, query) = split_path_query(callback_url)
            .ok_or_else(|| AppError::Auth("malformed callback URL".to_string()))?;
        if !path.ends_with("/auth/callback") {
            return Err(AppError::Auth(format!("unexpected callback path: {path}")));
        }
        let params: std::collections::HashMap<String, String> = parse_form_encoded(&query);
        if let Some(err) = params.get("error") {
            return Err(AppError::Auth(format!("OAuth error: {err}")));
        }
        let state = params
            .get("state")
            .ok_or_else(|| AppError::Auth("missing state".to_string()))?;
        if state != expected_state {
            return Err(AppError::Auth("OAuth state mismatch".to_string()));
        }
        let code = params
            .get("code")
            .filter(|c| !c.is_empty())
            .ok_or_else(|| AppError::Auth("missing code".to_string()))?;
        Ok(code.clone())
    }

    /// POST the code + verifier to the token endpoint and decode the
    /// response into a `CodexAuth`.
    pub async fn exchange_code(
        session: &OAuthSession,
        code: &str,
    ) -> AppResult<CodexAuth> {
        let body = format!(
            "grant_type=authorization_code\
&client_id={CLIENT_ID}\
&code={code}\
&redirect_uri={redirect}\
&code_verifier={verifier}",
            code = url_encode(code),
            redirect = url_encode(&session.redirect_uri),
            verifier = url_encode(&session.code_verifier),
        );
        let resp = reqwest::Client::new()
            .post(TOKEN_ENDPOINT)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .timeout(Duration::from_secs(25))
            .body(body)
            .send()
            .await
            .map_err(AppError::Http)?;
        let status = resp.status();
        let bytes = resp.bytes().await.map_err(AppError::Http)?;
        if !status.is_success() {
            return Err(AppError::Auth(format!(
                "token endpoint {}: {}",
                status.as_u16(),
                String::from_utf8_lossy(&bytes).chars().take(200).collect::<String>()
            )));
        }
        let parsed: TokenResponse = serde_json::from_slice(&bytes)
            .map_err(|e| AppError::Auth(format!("decode token response: {e}")))?;
        let account_id = crate::account::subject_id_from_idtoken(parsed.id_token.as_deref())
            .or_else(|| crate::account::subject_id_from_idtoken(Some(&parsed.access_token)));
        let plan_type = decode_plan_type(parsed.id_token.as_deref())
            .or_else(|| decode_plan_type(Some(&parsed.access_token)));
        let mut auth = CodexAuth {
            auth_mode: Some("chatgpt".to_string()),
            tokens: Some(crate::auth::Tokens {
                access_token: parsed.access_token,
                refresh_token: parsed.refresh_token.unwrap_or_default(),
                account_id: account_id.clone(),
                id_token: parsed.id_token,
            }),
            last_refresh: Some(now_iso8601()),
            plan_type,
            ..Default::default()
        };
        // If we couldn't read account_id from JWT, leave it None; the upsert
        // path will fall back to subject id (also from JWT) or `imported`.
        if auth.account_id().is_none() {
            auth.tokens.as_mut().unwrap().account_id = account_id;
        }
        if !auth.can_refresh_oauth() && auth.access_token().map(|s| s.is_empty()).unwrap_or(true) {
            return Err(AppError::Auth("token endpoint returned no usable credentials".to_string()));
        }
        Ok(auth)
    }
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    id_token: Option<String>,
    access_token: String,
    refresh_token: Option<String>,
}

// ---------------------------------------------------------------------------
// Callback HTTP server
// ---------------------------------------------------------------------------

/// One-shot localhost listener that captures a single OAuth callback URL.
pub struct CallbackServer {
    port: u16,
    captured: Arc<Mutex<Option<String>>>,
    listener: Option<TcpListener>,
}

impl CallbackServer {
    /// Bind the preferred port. Returns `Err` if the port is taken.
    pub fn bind(port: u16) -> AppResult<Self> {
        let listener = TcpListener::bind(("127.0.0.1", port))
            .map_err(|e| AppError::Auth(format!("bind :{port}: {e}")))?;
        Ok(Self {
            port,
            captured: Arc::new(Mutex::new(None)),
            listener: Some(listener),
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// Block on the listener until a request arrives or the timeout elapses.
    /// On success returns the full URL the browser requested.
    pub fn wait_for_callback(mut self, timeout: Duration) -> AppResult<String> {
        let listener = self.listener.take().expect("listener already taken");
        listener
            .set_nonblocking(false)
            .map_err(|e| AppError::Auth(format!("set_nonblocking: {e}")))?;

        let captured = Arc::clone(&self.captured);
        let deadline = Instant::now() + timeout;

        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(AppError::Auth("OAuth callback timed out".to_string()));
            }
            // set a per-accept timeout via SO_RCVTIMEO so the loop can
            // re-check the deadline.
            set_recv_timeout(&listener, remaining)?;
            let (mut stream, _) = match listener.accept() {
                Ok(pair) => pair,
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut
                    || e.kind() == std::io::ErrorKind::WouldBlock => continue,
                Err(e) => {
                    return Err(AppError::Auth(format!("accept: {e}")));
                }
            };
            let _ = set_stream_timeout(&stream, Duration::from_secs(5));
            // Read just enough of the request to find the request line.
            let mut buf = [0u8; 4096];
            let n = match stream.read(&mut buf) {
                Ok(n) => n,
                Err(e) => {
                    let _ = stream.write_all(serve_error_page("read failed"));
                    return Err(AppError::Auth(format!("read request: {e}")));
                }
            };
            let request = String::from_utf8_lossy(&buf[..n]).to_string();
            if let Some(line) = request.lines().next() {
                if let Some(target) = parse_request_target(line) {
                    let url = format!("http://localhost:{}{}", self.port, target);
                    {
                        let mut g = captured.lock().unwrap();
                        *g = Some(url.clone());
                    }
                    let _ = stream.write_all(serve_success_page());
                    let _ = stream.flush();
                    return Ok(url);
                }
            }
            let _ = stream.write_all(serve_error_page("malformed request"));
        }
    }
}

fn set_recv_timeout(listener: &TcpListener, dur: Duration) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        // Build a `timeval` for SO_RCVTIMEO. tv_sec is i64 on Linux, i32 on
        // macOS; we cast through the platform-correct type via libc.
        let secs = dur.as_secs() as libc::time_t;
        let usecs = dur.subsec_micros() as libc::suseconds_t;
        let timeout = libc::timeval {
            tv_sec: secs,
            tv_usec: usecs,
        };
        let fd = listener.as_raw_fd();
        let rv = unsafe {
            libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_RCVTIMEO,
                &timeout as *const _ as *const _,
                std::mem::size_of::<libc::timeval>() as libc::socklen_t,
            )
        };
        if rv != 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (listener, dur);
    }
    Ok(())
}

#[cfg(unix)]
use std::os::unix::io::AsRawFd;

fn set_stream_timeout(stream: &TcpStream, dur: Duration) -> std::io::Result<()> {
    stream.set_read_timeout(Some(dur))?;
    stream.set_write_timeout(Some(dur))?;
    Ok(())
}

fn parse_request_target(line: &str) -> Option<String> {
    // "GET /path?query HTTP/1.1"
    let mut parts = line.splitn(3, ' ');
    let method = parts.next()?;
    if method != "GET" {
        return None;
    }
    parts.next().map(|s| s.to_string())
}

fn serve_success_page() -> &'static [u8] {
    use std::sync::OnceLock;
    static CACHE: OnceLock<Vec<u8>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let body = "<!doctype html><html><head><meta charset=\"utf-8\"><title>Codex Runway</title>\
<style>body{font-family:system-ui,Segoe UI,sans-serif;padding:2rem;max-width:32rem;margin:auto;\
color:#1c1c1e;background:#fafafa}h1{font-size:1.1rem;margin:0 0 .5rem}\
p{color:#555;line-height:1.5}</style></head>\
<body><h1>Sign-in complete</h1>\
<p>You can close this tab and return to Codex Runway. The window will refresh on its own.</p>\
</body></html>";
        build_http_response(200, "OK", "text/html; charset=utf-8", body)
    })
    .as_slice()
}

fn serve_error_page(reason: &str) -> &'static [u8] {
    let body = format!(
        "<!doctype html><body style=\"font-family:system-ui;padding:2rem\">\
<h2>Sign-in failed</h2><p>{}</p></body>",
        html_escape(reason)
    );
    let boxed: &'static mut Vec<u8> =
        Box::leak(Box::new(build_http_response(400, "Bad Request", "text/html; charset=utf-8", &body)));
    boxed.as_slice()
}

fn build_http_response(status: u16, reason: &str, content_type: &str, body: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 {status} {reason}\r\n\
Content-Type: {content_type}\r\n\
Content-Length: {len}\r\n\
Connection: close\r\n\
\r\n\
{body}",
        len = body.len()
    )
    .into_bytes()
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn random_base64url(byte_len: usize) -> String {
    let mut buf = vec![0u8; byte_len];
    rand::rngs::OsRng.fill_bytes(&mut buf);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&buf)
}

fn code_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
}

fn url_encode(s: &str) -> String {
    // Minimal percent-encoder. Codex OAuth values are all URL-safe, so we
    // only need to escape a handful of characters.
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            other => {
                out.push_str(&format!("%{:02X}", other));
            }
        }
    }
    out
}

fn split_path_query(url: &str) -> Option<(&str, &str)> {
    let scheme_end = url.find("://")?;
    let after_scheme = &url[scheme_end + 3..];
    let slash = after_scheme.find('/')?;
    let path_and_query = &after_scheme[slash..];
    let query_start = path_and_query.find('?')?;
    Some((&path_and_query[..query_start], &path_and_query[query_start + 1..]))
}

fn parse_form_encoded(s: &str) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    for pair in s.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = match pair.split_once('=') {
            Some((k, v)) => (k, v),
            None => (pair, ""),
        };
        out.insert(url_decode(k), url_decode(v));
    }
    out
}

fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                if let (Some(a), Some(b)) = (hex_digit(bytes[i + 1]), hex_digit(bytes[i + 2])) {
                    out.push(a * 16 + b);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn now_iso8601() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn decode_plan_type(jwt: Option<&str>) -> Option<String> {
    let jwt = jwt?;
    let payload = jwt.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    v.get("https://api.openai.com/auth")
        .and_then(|c| c.get("chatgpt_plan_type"))
        .and_then(|p| p.as_str())
        .map(String::from)
        .or_else(|| {
            v.get("chatgpt_plan_type")
                .and_then(|p| p.as_str())
                .map(String::from)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpStream;

    #[test]
    fn pkce_challenge_matches_sha256() {
        let verifier = "test-verifier-value-1234567890";
        let challenge = code_challenge(verifier);
        // Re-derive the same challenge from the same verifier and compare.
        let digest = Sha256::digest(verifier.as_bytes());
        let expected =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);
        assert_eq!(challenge, expected);
        // Spot-check that S256 produced something different from the input.
        assert_ne!(challenge, verifier);
        assert!(!challenge.contains('='));
        assert!(!challenge.contains('+'));
    }

    #[test]
    fn state_mismatch_rejected() {
        let url = "http://localhost:1455/auth/callback?code=abc&state=other";
        let err = OAuthLogin::authorization_code(url, "expected").unwrap_err();
        match err {
            AppError::Auth(msg) => assert!(msg.contains("state"), "{msg}"),
            _ => panic!("expected Auth error"),
        }
    }

    #[test]
    fn code_extraction() {
        let url = "http://localhost:1455/auth/callback?code=the-code&state=s";
        let code = OAuthLogin::authorization_code(url, "s").unwrap();
        assert_eq!(code, "the-code");
    }

    #[test]
    fn oauth_error_surfaces() {
        let url = "http://localhost:1455/auth/callback?error=access_denied&state=s";
        let err = OAuthLogin::authorization_code(url, "s").unwrap_err();
        match err {
            AppError::Auth(msg) => assert!(msg.contains("access_denied"), "{msg}"),
            _ => panic!("expected Auth error"),
        }
    }

    #[test]
    fn url_form_round_trip() {
        let encoded = url_encode("hello world/a+b=c");
        let decoded = url_decode(&encoded);
        assert_eq!(decoded, "hello world/a+b=c");
    }

    #[test]
    fn callback_server_handles_request() {
        // Bind a free port via port 0 trick.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let server = CallbackServer::bind(port).unwrap();
        // Connect from another thread, send a request, close.
        let handle = thread::spawn(move || {
            let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
            s.write_all(b"GET /auth/callback?code=abc&state=ok HTTP/1.1\r\nHost: localhost\r\n\r\n")
                .unwrap();
            let mut buf = [0u8; 1024];
            let _ = s.read(&mut buf);
            String::from_utf8_lossy(&buf).into_owned()
        });
        let url = server
            .wait_for_callback(Duration::from_secs(5))
            .unwrap();
        let _ = handle.join().unwrap();
        assert!(url.contains("/auth/callback"));
        assert!(url.contains("code=abc"));
    }
}
