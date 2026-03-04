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
use russh::server::{Auth, Config, Handle, Handler, Server, Session};
use russh::{ChannelId, ChannelMsg, Preferred, kex};
use tokio::net::TcpListener;

type Registry = Arc<RwLock<HashMap<String, Option<Tunnel>>>>;

#[derive(Clone)]
struct Tunnel {
    host: String,
    port: u16,
    handle: Handle,
}

fn tunnel_domain() -> String {
    std::env::var("TUNNEL_DOMAIN").unwrap_or_else(|_| "t.tn3w.dev".to_string())
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

fn register_new_tunnel(registry: &Registry) -> String {
    let token = generate_unique_token(registry);
    registry.write().unwrap().insert(token.clone(), None);
    token
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

fn bad_gateway(message: &str) -> Response {
    (StatusCode::BAD_GATEWAY, message.to_string()).into_response()
}

fn index_security_headers() -> HeaderMap {
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
    (index_security_headers(), html).into_response()
}

fn resolve_tunnel_for_host(registry: &Registry, host: &str) -> Result<Tunnel, Box<Response>> {
    let token = extract_token_from_host(host)
        .ok_or_else(|| Box::new(bad_gateway("Invalid subdomain format")))?;

    match registry.read().unwrap().get(&token).cloned() {
        Some(Some(tunnel)) => Ok(tunnel),
        Some(None) => Err(Box::new(bad_gateway("Tunnel not yet connected"))),
        None => Err(Box::new(bad_gateway("Tunnel not found"))),
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
    channel: &mut russh::Channel<russh::server::Msg>,
) -> Result<Vec<u8>, Box<Response>> {
    const MAX_RESPONSE_BYTES: usize = 50 * 1024 * 1024;
    const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);

    let mut raw_response: Vec<u8> = Vec::with_capacity(65536);

    loop {
        if raw_response.len() > MAX_RESPONSE_BYTES {
            return Err(Box::new(bad_gateway("Response too large")));
        }

        match tokio::time::timeout(RESPONSE_TIMEOUT, channel.wait()).await {
            Ok(Some(ChannelMsg::Data { ref data })) => raw_response.extend_from_slice(data),
            Ok(Some(ChannelMsg::Eof)) | Ok(None) => break,
            Ok(_) => continue,
            Err(_) => return Err(Box::new(bad_gateway("Tunnel response timed out"))),
        }
    }

    if raw_response.is_empty() {
        return Err(Box::new(bad_gateway("Empty response from tunnel")));
    }

    Ok(raw_response)
}

fn parse_tunnel_response(raw_response: Vec<u8>) -> Result<Response, Box<Response>> {
    let header_end = raw_response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .filter(|&position| position > 0)
        .ok_or_else(|| Box::new(bad_gateway("Malformed HTTP response")))?;

    let header_str = std::str::from_utf8(&raw_response[..header_end])
        .map_err(|_| Box::new(bad_gateway("Invalid response headers")))?;

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
            .map_err(|_| Box::new(bad_gateway("Failed to decode chunked response")))?
    } else {
        response_body.to_vec()
    };

    builder
        .body(Body::from(final_body))
        .map_err(|_| Box::new(bad_gateway("Failed to construct response")))
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

    let tunnel = match resolve_tunnel_for_host(&registry, host) {
        Ok(tunnel) => tunnel,
        Err(error_response) => return *error_response,
    };

    let mut channel = match tunnel
        .handle
        .channel_open_forwarded_tcpip(tunnel.host, tunnel.port as u32, "127.0.0.1", 0)
        .await
    {
        Ok(channel) => channel,
        Err(_) => return bad_gateway("Failed to open tunnel channel"),
    };

    let (request_parts, request_body) = request.into_parts();
    let request = Request::from_parts(request_parts, ());

    const MAX_BODY_BYTES: usize = 10 * 1024 * 1024;

    let body_bytes = match axum::body::to_bytes(request_body, MAX_BODY_BYTES).await {
        Ok(bytes) => bytes,
        Err(_) => return bad_gateway("Request body too large"),
    };

    let raw_request = build_raw_http_request(&request, &body_bytes);

    if channel.data(raw_request.as_slice()).await.is_err() {
        return bad_gateway("Failed to send request through tunnel");
    }

    let raw_response = match collect_tunnel_response(&mut channel).await {
        Ok(response) => response,
        Err(error_response) => return *error_response,
    };

    parse_tunnel_response(raw_response).unwrap_or_else(|error_response| *error_response)
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
            self.token = Some(register_new_tunnel(&self.registry));
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
            handle: session.handle(),
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
