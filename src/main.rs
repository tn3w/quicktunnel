use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use russh::keys::{ssh_key, ssh_key::rand_core::OsRng, Algorithm, PrivateKey};
use russh::server::{Auth, Config, Handle, Handler, Server, Session};
use russh::{kex, ChannelId, ChannelMsg, Preferred};
use tokio::net::TcpListener;

type Registry = Arc<RwLock<HashMap<String, Option<Tunnel>>>>;

#[derive(Clone)]
struct Tunnel {
    host: String,
    port: u16,
    handle: Handle,
}

fn get_tunnel_domain() -> String {
    std::env::var("TUNNEL_DOMAIN").unwrap_or_else(|_| "t.tn3w.dev".to_string())
}

fn generate_token(registry: &Registry) -> String {
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";

    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64 ^ d.as_secs().wrapping_mul(0x9e3779b97f4a7c15))
        .unwrap_or(0xdeadbeefcafebabe);

    let mut state = seed;
    let mut next_char = || -> char {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        CHARS[state as usize % CHARS.len()] as char
    };

    loop {
        let token: String = (0..6).map(|_| next_char()).collect();

        if !registry.read().unwrap().contains_key(&token) {
            return token;
        }
    }
}

fn register_tunnel(registry: &Registry) -> String {
    let token = generate_token(registry);
    registry.write().unwrap().insert(token.clone(), None);
    token
}

fn token_from_host(host: &str) -> Option<String> {
    let domain = get_tunnel_domain();
    let suffix = format!(".{}", domain);

    host.strip_suffix(&suffix)
        .filter(|token| !token.is_empty() && !token.contains('.'))
        .map(|token| token.to_string())
}

fn decode_chunked_body(data: &[u8]) -> Result<Vec<u8>, ()> {
    let mut output = Vec::new();
    let mut pos = 0;

    loop {
        let crlf = data[pos..]
            .windows(2)
            .position(|w| w == b"\r\n")
            .ok_or(())?;

        let size_hex = std::str::from_utf8(&data[pos..pos + crlf])
            .map_err(|_| ())?
            .trim()
            .split(';')
            .next()
            .ok_or(())?
            .trim();

        let chunk_size = usize::from_str_radix(size_hex, 16).map_err(|_| ())?;

        if chunk_size == 0 {
            break;
        }

        pos += crlf + 2;

        if pos + chunk_size > data.len() {
            return Err(());
        }

        output.extend_from_slice(&data[pos..pos + chunk_size]);
        pos += chunk_size + 2;
    }

    Ok(output)
}

fn bad_gateway(message: &str) -> Response {
    (StatusCode::BAD_GATEWAY, message.to_string()).into_response()
}

fn security_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();

    for (key, value) in [
        ("content-type", "text/html; charset=utf-8"),
        ("x-content-type-options", "nosniff"),
        ("x-frame-options", "DENY"),
        ("x-xss-protection", "1; mode=block"),
        (
            "content-security-policy",
            "default-src 'self'; script-src 'self' 'unsafe-inline'; \
             style-src 'self' 'unsafe-inline'; img-src 'self' data:; \
             connect-src 'self'; form-action 'self'; frame-ancestors 'none'; \
             base-uri 'self'",
        ),
        ("referrer-policy", "strict-origin-when-cross-origin"),
        ("permissions-policy", "geolocation=(), microphone=(), camera=()"),
        ("cross-origin-opener-policy", "same-origin"),
        ("cross-origin-resource-policy", "same-origin"),
        ("strict-transport-security", "max-age=31536000; includeSubDomains"),
    ] {
        headers.insert(key, value.parse().unwrap());
    }

    headers
}

async fn serve_index() -> Response {
    let html = std::fs::read_to_string("index.html").unwrap_or_default();
    (security_headers(), html).into_response()
}

async fn proxy_request(
    State(registry): State<Registry>,
    request_headers: HeaderMap,
    request: Request,
) -> Response {
    let host = request_headers
        .get("host")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    let token = match token_from_host(host) {
        Some(token) => token,
        None => return bad_gateway("Invalid subdomain format"),
    };

    let tunnel = match registry.read().unwrap().get(&token).cloned() {
        Some(Some(tunnel)) => tunnel,
        Some(None) => return bad_gateway("Tunnel not yet connected"),
        None => return bad_gateway("Tunnel not found"),
    };

    let mut channel = match tunnel
        .handle
        .channel_open_forwarded_tcpip(tunnel.host, tunnel.port as u32, "127.0.0.1", 0)
        .await
    {
        Ok(channel) => channel,
        Err(_) => return bad_gateway("Failed to open tunnel channel"),
    };

    let (parts, body) = request.into_parts();

    const MAX_BODY: usize = 10 * 1024 * 1024;

    let body_bytes = match axum::body::to_bytes(body, MAX_BODY).await {
        Ok(bytes) => bytes,
        Err(_) => return bad_gateway("Request body too large"),
    };

    let path_and_query = parts
        .uri
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");

    let host_header = parts
        .headers
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost");

    let mut raw_request =
        format!("{} {} HTTP/1.1\r\nhost: {}\r\n", parts.method, path_and_query, host_header)
            .into_bytes();

    for (key, value) in parts.headers.iter().filter(|(k, _)| *k != "host") {
        let Ok(value_str) = value.to_str() else {
            continue;
        };

        if key.as_str().len() + value_str.len() <= 8192 {
            raw_request.extend_from_slice(format!("{}: {}\r\n", key, value_str).as_bytes());
        }
    }

    raw_request.extend_from_slice(b"\r\n");
    raw_request.extend_from_slice(&body_bytes);

    if channel.data(raw_request.as_slice()).await.is_err() {
        return bad_gateway("Failed to send request through tunnel");
    }

    let mut raw_response: Vec<u8> = Vec::with_capacity(65536);

    const MAX_RESPONSE: usize = 50 * 1024 * 1024;
    const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);

    loop {
        if raw_response.len() > MAX_RESPONSE {
            return bad_gateway("Response too large");
        }

        match tokio::time::timeout(RESPONSE_TIMEOUT, channel.wait()).await {
            Ok(Some(ChannelMsg::Data { ref data })) => raw_response.extend_from_slice(data),
            Ok(Some(ChannelMsg::Eof)) | Ok(None) => break,
            Ok(_) => continue,
            Err(_) => return bad_gateway("Tunnel response timed out"),
        }
    }

    if raw_response.is_empty() {
        return bad_gateway("Empty response from tunnel");
    }

    let header_end = match raw_response.windows(4).position(|w| w == b"\r\n\r\n") {
        Some(pos) if pos > 0 => pos,
        _ => return bad_gateway("Malformed HTTP response"),
    };

    let header_str = match std::str::from_utf8(&raw_response[..header_end]) {
        Ok(text) => text,
        Err(_) => return bad_gateway("Invalid response headers"),
    };

    let response_body = &raw_response[header_end + 4..];
    let mut header_lines = header_str.lines();

    let status_code = header_lines
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .and_then(|code| StatusCode::from_u16(code).ok())
        .unwrap_or(StatusCode::OK);

    let mut builder = Response::builder().status(status_code);
    let mut is_chunked = false;

    for line in header_lines {
        let Some(colon) = line.find(':') else {
            continue;
        };

        let header_name = &line[..colon];
        let header_value = line[colon + 1..].trim();

        if header_name.eq_ignore_ascii_case("transfer-encoding")
            && header_value.eq_ignore_ascii_case("chunked")
        {
            is_chunked = true;
            continue;
        }

        if is_chunked && header_name.eq_ignore_ascii_case("content-length") {
            continue;
        }

        builder = builder.header(header_name, header_value);
    }

    let final_body = if is_chunked {
        match decode_chunked_body(response_body) {
            Ok(decoded) => decoded,
            Err(_) => return bad_gateway("Failed to decode chunked response"),
        }
    } else {
        response_body.to_vec()
    };

    builder
        .body(Body::from(final_body))
        .unwrap_or_else(|_| bad_gateway("Failed to construct response"))
}

struct SshClientHandler {
    registry: Registry,
    token: Option<String>,
}

impl SshClientHandler {
    fn new(registry: Registry) -> Self {
        Self { registry, token: None }
    }

    fn ensure_token(&mut self) -> &str {
        if self.token.is_none() {
            self.token = Some(register_tunnel(&self.registry));
        }
        self.token.as_deref().unwrap()
    }
}

impl Drop for SshClientHandler {
    fn drop(&mut self) {
        if let Some(token) = &self.token {
            self.registry.write().unwrap().remove(token);
        }
    }
}

impl Handler for SshClientHandler {
    type Error = russh::Error;

    fn authentication_banner(
        &mut self,
    ) -> impl Future<Output = Result<Option<String>, Self::Error>> + Send {
        let token = self.ensure_token().to_string();
        let domain = get_tunnel_domain();
        let url = format!("https://{}.{}", token, domain);
        let inner = format!("  QuickTunnel  ▸  {}  ", url);
        let width = inner.chars().count();
        let line = "─".repeat(width);
        let banner = format!(
            "\r\n┌{}┐\r\n│{}│\r\n└{}┘\r\n\r\n",
            line, inner, line
        );
        async move { Ok(Some(banner)) }
    }

    async fn auth_none(&mut self, _user: &str) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn auth_password(&mut self, _user: &str, _password: &str) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn auth_publickey(
        &mut self,
        _user: &str,
        _public_key: &ssh_key::PublicKey,
    ) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    fn tcpip_forward(
        &mut self,
        address: &str,
        port: &mut u32,
        session: &mut Session,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        let token = self.ensure_token().to_string();
        let tunnel = Tunnel {
            host: address.to_string(),
            port: *port as u16,
            handle: session.handle(),
        };
        let registry = self.registry.clone();

        async move {
            registry.write().unwrap().insert(token, Some(tunnel));
            Ok(true)
        }
    }

    async fn channel_close(&mut self, _channel: ChannelId, _session: &mut Session) -> Result<(), Self::Error> {
        Ok(())
    }
}

struct TunnelServer {
    registry: Registry,
}

impl Server for TunnelServer {
    type Handler = SshClientHandler;

    fn new_client(&mut self, _peer_addr: Option<std::net::SocketAddr>) -> Self::Handler {
        SshClientHandler::new(self.registry.clone())
    }
}

fn load_or_create_host_key(path: &Path) -> Result<PrivateKey, Box<dyn std::error::Error>> {
    if path.exists() {
        return Ok(russh::keys::load_secret_key(path, None)?);
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let key = PrivateKey::random(&mut OsRng, Algorithm::Ed25519)?;
    let pem = key.to_openssh(ssh_key::LineEnding::LF)?;
    std::fs::write(path, pem.as_bytes())?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }

    Ok(key)
}

fn build_ssh_config(host_key: PrivateKey) -> Arc<Config> {
    Arc::new(Config {
        inactivity_timeout: Some(Duration::from_secs(300)),
        keepalive_interval: Some(Duration::from_secs(60)),
        keepalive_max: 3,
        auth_rejection_time: Duration::from_secs(3),
        auth_rejection_time_initial: Some(Duration::ZERO),
        max_auth_attempts: 3,
        window_size: 2097152,
        maximum_packet_size: 32768,
        channel_buffer_size: 100,
        event_buffer_size: 20,
        keys: vec![host_key],
        preferred: Preferred {
            kex: Cow::Borrowed(&[kex::MLKEM768X25519_SHA256]),
            ..Preferred::default()
        },
        nodelay: true,
        ..Default::default()
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry: Registry = Arc::new(RwLock::new(HashMap::new()));

    let index_router = Router::new().route("/", any(serve_index));
    let index_listener = TcpListener::bind("0.0.0.0:3000").await?;

    let proxy_router = Router::new()
        .route("/", any(proxy_request))
        .route("/{*path}", any(proxy_request))
        .with_state(registry.clone());
    let proxy_listener = TcpListener::bind("0.0.0.0:8080").await?;

    let host_key = load_or_create_host_key(Path::new("/app/keys/ssh_host_ed25519_key"))?;
    let ssh_config = build_ssh_config(host_key);
    let mut ssh_server = TunnelServer { registry };

    tokio::try_join!(
        tokio::spawn(async move {
            axum::serve(index_listener, index_router)
                .await
                .expect("Index server failed");
        }),
        tokio::spawn(async move {
            axum::serve(proxy_listener, proxy_router)
                .await
                .expect("Proxy server failed");
        }),
        tokio::spawn(async move {
            ssh_server
                .run_on_address(ssh_config, ("0.0.0.0", 22))
                .await
                .expect("SSH server failed");
        }),
    )?;

    Ok(())
}
