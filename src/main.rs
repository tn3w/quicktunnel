use std::collections::HashMap;
use std::error::Error;
use std::os::fd::FromRawFd;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, Method, StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use futures_util::StreamExt;
use hyper_util::rt::TokioIo;
use russh::keys::ssh_key::LineEnding;
use russh::keys::{Algorithm, PrivateKey};
use russh::server::{Auth, Config, Handle, Handler, Session};
use tokio::time::{Instant, sleep_until, timeout};

const MAX_TUNNELS: usize = 64;
const TUNNELS_PER_IP: usize = 3;
const BYTES_PER_SECOND: f64 = 1_000_000.0;
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);
const WARNING_COOKIE: &str = "__Host-tunnel-warning=seen";

type Forward = (Handle, String, u32);

struct Tunnel {
    ip: String,
    forward: Option<Forward>,
}

#[derive(Clone)]
struct App {
    domain: String,
    host_key: String,
    ssh: Arc<Config>,
    tunnels: Arc<Mutex<HashMap<String, Tunnel>>>,
    next_send: Arc<Mutex<Instant>>,
}

struct Client {
    app: App,
    token: Option<String>,
}

impl Drop for Client {
    fn drop(&mut self) {
        if let Some(token) = &self.token {
            self.app.tunnels.lock().unwrap().remove(token);
        }
    }
}

impl Handler for Client {
    type Error = russh::Error;

    async fn authentication_banner(&mut self) -> Result<Option<String>, Self::Error> {
        let domain = &self.app.domain;
        let Some(token) = &self.token else {
            return Ok(Some("\r\n  Server is full, try again later.\r\n\r\n".into()));
        };
        Ok(Some(format!(
            "\r\n  Your local server is live at https://{token}.{domain}\r\n\r\n  \
             Share the link, keep this open while you need it, Ctrl+C to stop.\r\n\r\n"
        )))
    }

    async fn auth_none(&mut self, _user: &str) -> Result<Auth, Self::Error> {
        Ok(if self.token.is_some() { Auth::Accept } else { Auth::reject() })
    }

    async fn tcpip_forward(
        &mut self,
        address: &str,
        port: &mut u32,
        session: &mut Session,
    ) -> Result<bool, Self::Error> {
        let Some(token) = &self.token else {
            return Ok(false);
        };
        let mut tunnels = self.app.tunnels.lock().unwrap();
        let Some(tunnel) = tunnels.get_mut(token) else {
            return Ok(false);
        };
        tunnel.forward = Some((session.handle(), address.into(), *port));
        Ok(true)
    }
}

fn random_token() -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    rand::random::<[u8; 4]>()
        .iter()
        .map(|byte| CHARSET[*byte as usize % CHARSET.len()] as char)
        .collect()
}

fn reserve_token(app: &App, ip: String) -> Option<String> {
    let mut tunnels = app.tunnels.lock().unwrap();
    let from_ip = tunnels.values().filter(|tunnel| tunnel.ip == ip).count();
    if tunnels.len() >= MAX_TUNNELS || from_ip >= TUNNELS_PER_IP {
        return None;
    }
    let token = std::iter::repeat_with(random_token)
        .find(|token| !tunnels.contains_key(token))?;
    tunnels.insert(token.clone(), Tunnel { ip, forward: None });
    Some(token)
}

fn header_value<'a>(headers: &'a HeaderMap, name: &str) -> &'a str {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
}

fn client_ip(headers: &HeaderMap) -> String {
    let forwarded = header_value(headers, "x-forwarded-for");
    let ip = match header_value(headers, "cf-connecting-ip") {
        "" => forwarded.split(',').next().unwrap_or_default(),
        ip => ip,
    };
    ip.trim().to_string()
}

fn accept_ssh(app: App, mut request: Request) -> Response {
    let ip = client_ip(request.headers());
    let upgrade = hyper::upgrade::on(&mut request);

    tokio::spawn(async move {
        let Ok(stream) = upgrade.await else {
            return;
        };
        let token = reserve_token(&app, ip);
        let client = Client { app: app.clone(), token };
        if let Ok(session) = russh::server::run_stream(app.ssh, TokioIo::new(stream), client).await
        {
            let _ = session.await;
        }
    });

    Response::builder()
        .status(StatusCode::SWITCHING_PROTOCOLS)
        .header(header::UPGRADE, "websocket")
        .header(header::CONNECTION, "Upgrade")
        .body(Body::empty())
        .unwrap()
}

fn page(status: StatusCode, html: String) -> Response {
    let headers = [(header::CACHE_CONTROL, "no-store"), (header::X_FRAME_OPTIONS, "DENY")];
    (status, headers, Html(html)).into_response()
}

fn script(app: &App, port: u16) -> Response {
    let script = include_str!("tunnel.sh")
        .replace("{host}", &format!("t.{}", app.domain))
        .replace("{key}", &app.host_key)
        .replace("{port}", &port.to_string());
    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], script).into_response()
}

fn entry(app: App, request: Request) -> Response {
    if request.headers().contains_key(header::UPGRADE) {
        return accept_ssh(app, request);
    }
    let path = request.uri().path().trim_start_matches('/');
    if path.is_empty() {
        let html = include_str!("index.html").replace("{domain}", &app.domain);
        return page(StatusCode::OK, html);
    }
    match path.parse::<u16>() {
        Ok(port) if port > 0 => script(&app, port),
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

fn needs_warning(request: &Request) -> bool {
    let headers = request.headers();
    request.method() == Method::GET
        && header_value(headers, "user-agent").starts_with("Mozilla/")
        && header_value(headers, "accept").contains("text/html")
        && !header_value(headers, "cookie").contains(WARNING_COOKIE)
}

async fn pace(app: &App, bytes: usize) {
    let duration = Duration::from_secs_f64(bytes as f64 / BYTES_PER_SECOND);
    let start = {
        let mut next_send = app.next_send.lock().unwrap();
        let start = (*next_send).max(Instant::now());
        *next_send = start + duration;
        start
    };
    sleep_until(start).await;
}

async fn send_through(
    app: App,
    (handle, address, port): Forward,
    mut request: Request,
) -> Result<Response, Box<dyn Error + Send + Sync>> {
    let channel = handle
        .channel_open_forwarded_tcpip(address, port, "127.0.0.1", 0)
        .await?;
    let io = TokioIo::new(channel.into_stream());
    let (mut sender, connection) = hyper::client::conn::http1::handshake(io).await?;
    tokio::spawn(connection.with_upgrades());

    let visitor_upgrade = hyper::upgrade::on(&mut request);
    let mut response = sender.send_request(request).await?;

    if response.status() == StatusCode::SWITCHING_PROTOCOLS {
        let local_upgrade = hyper::upgrade::on(&mut response);
        tokio::spawn(async move {
            let (Ok(visitor), Ok(local)) = tokio::join!(visitor_upgrade, local_upgrade) else {
                return;
            };
            let (mut visitor, mut local) = (TokioIo::new(visitor), TokioIo::new(local));
            let _ = tokio::io::copy_bidirectional(&mut visitor, &mut local).await;
        });
    }

    Ok(response.map(|body| {
        let chunks = Body::new(body).into_data_stream().then(move |chunk| {
            let app = app.clone();
            async move {
                if let Ok(data) = &chunk {
                    pace(&app, data.len()).await;
                }
                chunk
            }
        });
        Body::from_stream(chunks)
    }))
}

async fn proxy(app: App, token: &str, request: Request) -> Response {
    let forward = app
        .tunnels
        .lock()
        .unwrap()
        .get(token)
        .and_then(|tunnel| tunnel.forward.clone());
    let Some(forward) = forward else {
        let html = include_str!("index.html").replace("{domain}", &app.domain);
        return page(StatusCode::NOT_FOUND, html);
    };
    if needs_warning(&request) {
        let html = include_str!("warning.html").replace("{domain}", &app.domain);
        return page(StatusCode::OK, html);
    }
    match timeout(RESPONSE_TIMEOUT, send_through(app, forward, request)).await {
        Ok(Ok(response)) => response,
        _ => (StatusCode::BAD_GATEWAY, "The tunnel's local server did not answer.\n")
            .into_response(),
    }
}

async fn route(State(app): State<App>, request: Request) -> Response {
    let host = header_value(request.headers(), "host");
    let host = host.split(':').next().unwrap_or_default().to_lowercase();
    if host == format!("t.{}", app.domain) {
        return entry(app, request);
    }
    match host.strip_suffix(&format!(".{}", app.domain)) {
        Some(token) => proxy(app.clone(), token, request).await,
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

fn load_host_key() -> Result<PrivateKey, Box<dyn Error>> {
    let directory = std::env::var("STATE_DIRECTORY").unwrap_or_else(|_| ".".into());
    let path = PathBuf::from(directory).join("host_key");
    if path.exists() {
        return Ok(russh::keys::load_secret_key(path, None)?);
    }
    let key = PrivateKey::random(&mut rand::rng(), Algorithm::Ed25519)?;
    std::fs::write(path, key.to_openssh(LineEnding::LF)?)?;
    Ok(key)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let key = load_host_key()?;
    let app = App {
        domain: std::env::var("DOMAIN").unwrap_or_else(|_| "tn3w.dev".into()),
        host_key: key.public_key().to_openssh()?,
        ssh: Arc::new(Config {
            keys: vec![key],
            keepalive_interval: Some(Duration::from_secs(30)),
            auth_rejection_time_initial: Some(Duration::ZERO),
            ..Default::default()
        }),
        tunnels: Default::default(),
        next_send: Arc::new(Mutex::new(Instant::now())),
    };
    let router = axum::Router::new().fallback(route).with_state(app);

    if std::env::var_os("LISTEN_FDS").is_some() {
        let listener = unsafe { std::os::unix::net::UnixListener::from_raw_fd(3) };
        listener.set_nonblocking(true)?;
        let listener = tokio::net::UnixListener::from_std(listener)?;
        return Ok(axum::serve(listener, router).await?);
    }
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    Ok(axum::serve(listener, router).await?)
}
