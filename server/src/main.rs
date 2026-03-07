use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::any,
};
use russh::keys::{
    Algorithm, PrivateKey, ssh_key,
    ssh_key::rand_core::{OsRng, RngCore},
};
use russh::server::{Auth, Config, Handler, Server, Session};
use russh::{ChannelId, Preferred, kex};
use tokio::net::TcpListener;

mod errors;

pub type Registry = Arc<RwLock<HashMap<String, Option<Tunnel>>>>;

#[derive(Clone)]
pub enum TunnelHandle {
    Ssh(russh::server::Handle),
    Quic(quinn::Connection),
}

#[derive(Clone)]
pub struct Tunnel {
    pub host: String,
    pub port: u16,
    pub handle: TunnelHandle,
}

pub mod quic;

pub fn tunnel_domain() -> String {
    std::env::var("TUNNEL_DOMAIN").unwrap_or_else(|_| "t.tn3w.dev".to_string())
}

fn index_port() -> u16 {
    std::env::var("INDEX_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000)
}

fn proxy_port() -> u16 {
    std::env::var("PROXY_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080)
}

fn ssh_port() -> u16 {
    std::env::var("SSH_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(22)
}

fn index_enabled() -> bool {
    std::env::var("INDEX_ENABLED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(true)
}

fn generate_unique_token(registry: &Registry) -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";

    loop {
        let mut random_bytes = [0u8; 6];
        OsRng.fill_bytes(&mut random_bytes);

        let token: String = random_bytes
            .iter()
            .map(|&byte| CHARSET[byte as usize % CHARSET.len()] as char)
            .collect();

        if !registry.read().unwrap().contains_key(&token) {
            return token;
        }
    }
}

pub fn register_new_tunnel(registry: &Registry, req_handle: Option<String>) -> String {
    if let Some(mut handle) = req_handle {
        handle = handle.to_lowercase();
        let is_valid_len = handle.len() >= 4 && handle.len() <= 63 && handle.len() != 6;
        let is_valid_chars = handle
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit());

        if is_valid_len && is_valid_chars {
            let is_available = !registry.read().unwrap().contains_key(&handle);
            if is_available {
                registry.write().unwrap().insert(handle.clone(), None);
                return handle;
            }
        }
    }
    let token = generate_unique_token(registry);
    registry.write().unwrap().insert(token.clone(), None);
    token
}

pub enum TunnelStream {
    Ssh(russh::Channel<russh::server::Msg>),
    Quic(quinn::SendStream, quinn::RecvStream),
}

impl TunnelStream {
    pub async fn data(&mut self, data: &[u8]) -> Result<(), ()> {
        match self {
            Self::Ssh(ch) => ch.data(data).await.map_err(|_| ()),
            Self::Quic(send, _) => send.write_all(data).await.map_err(|_| ()),
        }
    }

    pub fn finish_send(&mut self) {
        if let Self::Quic(send, _) = self {
            let _ = send.finish();
        }
    }
}

fn extract_token_from_host(host: &str) -> Option<String> {
    let suffix = format!(".{}", tunnel_domain());

    host.strip_suffix(&suffix)
        .filter(|subdomain| !subdomain.is_empty() && !subdomain.contains('.'))
        .map(str::to_string)
}

fn decode_chunked_body(data: &[u8]) -> Result<Vec<u8>, ()> {
    let mut output = Vec::new();
    let mut position = 0;

    loop {
        let crlf_offset = data[position..]
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or(())?;

        let size_str = std::str::from_utf8(&data[position..position + crlf_offset])
            .map_err(|_| ())?
            .trim()
            .split(';')
            .next()
            .ok_or(())?
            .trim();

        let chunk_size = usize::from_str_radix(size_str, 16).map_err(|_| ())?;

        if chunk_size == 0 {
            return Ok(output);
        }

        position += crlf_offset + 2;

        if position + chunk_size > data.len() {
            return Err(());
        }

        output.extend_from_slice(&data[position..position + chunk_size]);
        position += chunk_size + 2;
    }
}

fn security_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();

    for (name, value) in [
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
        (
            "permissions-policy",
            "geolocation=(), microphone=(), camera=()",
        ),
        ("cross-origin-opener-policy", "same-origin"),
        ("cross-origin-resource-policy", "same-origin"),
        (
            "strict-transport-security",
            "max-age=31536000; includeSubDomains",
        ),
    ] {
        headers.insert(name, value.parse().unwrap());
    }

    headers
}

async fn serve_index() -> Response {
    let html = std::fs::read_to_string("./dist/index.html").unwrap_or_default();
    (security_headers(), html).into_response()
}

async fn serve_404() -> Response {
    let html = std::fs::read_to_string("./dist/404.html")
        .unwrap_or_else(|_| "<h1>404 Not Found</h1>".to_string());
    (StatusCode::NOT_FOUND, security_headers(), html).into_response()
}

fn resolve_tunnel_for_host(registry: &Registry, host: &str) -> Result<Tunnel, Box<Response>> {
    let tunnel_domain = tunnel_domain();
    let token = extract_token_from_host(host)
        .ok_or_else(|| Box::new(errors::tunnel_not_found_error(host, &tunnel_domain)))?;

    match registry.read().unwrap().get(&token).cloned() {
        Some(Some(tunnel)) => Ok(tunnel),
        Some(None) => Err(Box::new(errors::tunnel_not_connected_error(&tunnel_domain))),
        None => Err(Box::new(errors::tunnel_not_found_error(
            host,
            &tunnel_domain,
        ))),
    }
}

fn build_raw_http_request<B>(request: &Request<B>, body_bytes: &[u8]) -> Vec<u8> {
    let path_and_query = request
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");

    let host_header = request
        .headers()
        .get("host")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("localhost");

    let mut raw = format!(
        "{} {} HTTP/1.1\r\nhost: {}\r\n",
        request.method(),
        path_and_query,
        host_header,
    )
    .into_bytes();

    for (name, value) in request.headers().iter().filter(|(name, _)| *name != "host") {
        let Ok(value_str) = value.to_str() else {
            continue;
        };

        if name.as_str().len() + value_str.len() <= 8192 {
            raw.extend_from_slice(format!("{}: {}\r\n", name, value_str).as_bytes());
        }
    }

    raw.extend_from_slice(b"\r\n");
    raw.extend_from_slice(body_bytes);
    raw
}

async fn collect_tunnel_response(
    channel: &mut TunnelStream,
    tunnel_domain: &str,
) -> Result<Vec<u8>, Box<Response>> {
    const MAX_RESPONSE_BYTES: usize = 50 * 1024 * 1024;
    const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);

    let mut raw_response: Vec<u8> = Vec::with_capacity(65536);

    loop {
        if raw_response.len() > MAX_RESPONSE_BYTES {
            return Err(Box::new(errors::response_too_large_error(tunnel_domain)));
        }

        match channel {
            TunnelStream::Ssh(ch) => {
                match tokio::time::timeout(RESPONSE_TIMEOUT, ch.wait()).await {
                    Ok(Some(russh::ChannelMsg::Data { data })) => {
                        raw_response.extend_from_slice(&data)
                    }
                    Ok(Some(russh::ChannelMsg::Eof)) | Ok(None) => break,
                    Ok(_) => continue,
                    Err(_) => return Err(Box::new(errors::tunnel_timeout_error(tunnel_domain))),
                }
            }
            TunnelStream::Quic(_send, recv) => {
                let mut buf = [0; 65536];
                match tokio::time::timeout(RESPONSE_TIMEOUT, recv.read(&mut buf)).await {
                    Ok(Ok(Some(n))) => raw_response.extend_from_slice(&buf[..n]),
                    Ok(Ok(None)) => break,
                    Ok(Err(_)) => break,
                    Err(_) => return Err(Box::new(errors::tunnel_timeout_error(tunnel_domain))),
                }
            }
        }
    }

    if raw_response.is_empty() {
        return Err(Box::new(errors::empty_response_error(tunnel_domain)));
    }

    Ok(raw_response)
}

fn parse_tunnel_response(
    raw_response: Vec<u8>,
    tunnel_domain: &str,
) -> Result<Response, Box<Response>> {
    let header_end = raw_response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .filter(|&position| position > 0)
        .ok_or_else(|| Box::new(errors::malformed_response_error(tunnel_domain)))?;

    let header_str = std::str::from_utf8(&raw_response[..header_end])
        .map_err(|_| Box::new(errors::invalid_headers_error(tunnel_domain)))?;

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
        decode_chunked_body(response_body)
            .map_err(|_| Box::new(errors::chunked_decode_error(tunnel_domain)))?
    } else {
        response_body.to_vec()
    };

    builder
        .body(Body::from(final_body))
        .map_err(|_| Box::new(errors::response_construction_error(tunnel_domain)))
}

async fn proxy_request(
    State(registry): State<Registry>,
    request_headers: HeaderMap,
    request: Request,
) -> Response {
    let tunnel_domain = tunnel_domain();

    let host = request_headers
        .get("host")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    let tunnel = match resolve_tunnel_for_host(&registry, host) {
        Ok(tunnel) => tunnel,
        Err(error_response) => return *error_response,
    };

    let mut channel = match tunnel.handle {
        TunnelHandle::Ssh(handle) => match handle
            .channel_open_forwarded_tcpip(tunnel.host.clone(), tunnel.port as u32, "127.0.0.1", 0)
            .await
        {
            Ok(channel) => TunnelStream::Ssh(channel),
            Err(e) => {
                return errors::upstream_connection_failed_error(
                    Some(&e.to_string()),
                    &tunnel_domain,
                );
            }
        },
        TunnelHandle::Quic(conn) => match conn.open_bi().await {
            Ok((send, recv)) => TunnelStream::Quic(send, recv),
            Err(e) => {
                return errors::upstream_connection_failed_error(
                    Some(&e.to_string()),
                    &tunnel_domain,
                );
            }
        },
    };

    let (request_parts, request_body) = request.into_parts();
    let request = Request::from_parts(request_parts, ());

    const MAX_BODY_BYTES: usize = 10 * 1024 * 1024;

    let body_bytes = match axum::body::to_bytes(request_body, MAX_BODY_BYTES).await {
        Ok(bytes) => bytes,
        Err(_) => return errors::request_body_too_large_error(&tunnel_domain),
    };

    let raw_request = build_raw_http_request(&request, &body_bytes);

    if channel.data(raw_request.as_slice()).await.is_err() {
        return errors::tunnel_send_failed_error(&tunnel_domain);
    }

    channel.finish_send();

    let raw_response = match collect_tunnel_response(&mut channel, &tunnel_domain).await {
        Ok(response) => response,
        Err(error_response) => return *error_response,
    };

    parse_tunnel_response(raw_response, &tunnel_domain)
        .unwrap_or_else(|error_response| *error_response)
}

struct SshClientHandler {
    registry: Registry,
    token: Option<String>,
}

impl SshClientHandler {
    fn new(registry: Registry) -> Self {
        Self {
            registry,
            token: None,
        }
    }

    fn get_or_create_token(&mut self) -> &str {
        if self.token.is_none() {
            self.token = Some(register_new_tunnel(&self.registry, None));
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
        let token = self.get_or_create_token().to_string();
        let url = format!("https://{}.{}", token, tunnel_domain());
        let inner = format!("  QuickTunnel  ▸  {}  ", url);
        let border = "─".repeat(inner.chars().count());
        let banner = format!("\r\n┌{}┐\r\n│{}│\r\n└{}┘\r\n\r\n", border, inner, border);

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
        let token = self.get_or_create_token().to_string();
        let tunnel = Tunnel {
            host: address.to_string(),
            port: *port as u16,
            handle: TunnelHandle::Ssh(session.handle()),
        };
        let registry = self.registry.clone();

        async move {
            registry.write().unwrap().insert(token, Some(tunnel));
            Ok(true)
        }
    }

    async fn channel_close(
        &mut self,
        _channel: ChannelId,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
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

    std::fs::write(path, key.to_openssh(ssh_key::LineEnding::LF)?.as_bytes())?;

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

    let proxy_router = Router::new()
        .route("/", any(proxy_request))
        .route("/{*path}", any(proxy_request))
        .with_state(registry.clone());
    let proxy_listener = TcpListener::bind(format!("0.0.0.0:{}", proxy_port())).await?;

    let host_key = load_or_create_host_key(Path::new("/app/keys/ssh_host_ed25519_key"))?;
    let ssh_config = build_ssh_config(host_key);
    let mut ssh_server = TunnelServer {
        registry: registry.clone(),
    };

    let mut tasks = vec![
        tokio::spawn(async move {
            axum::serve(proxy_listener, proxy_router)
                .await
                .expect("Proxy server failed");
        }),
        tokio::spawn(async move {
            ssh_server
                .run_on_address(ssh_config, ("0.0.0.0", ssh_port()))
                .await
                .expect("SSH server failed");
        }),
        tokio::spawn({
            let registry = registry.clone();
            async move {
                quic::serve_quic(registry, quic::quic_port())
                    .await
                    .expect("QUIC server failed");
            }
        }),
    ];

    if index_enabled() {
        let index_router = Router::new()
            .route("/", any(serve_index))
            .fallback(serve_404);
        let index_listener = TcpListener::bind(format!("0.0.0.0:{}", index_port())).await?;

        tasks.push(tokio::spawn(async move {
            axum::serve(index_listener, index_router)
                .await
                .expect("Index server failed");
        }));
    }

    for task in tasks {
        task.await?;
    }

    Ok(())
}
