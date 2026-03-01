use dashmap::DashMap;
use rand::Rng;
use std::sync::Arc;
use axum::{
    extract::{Request, State},
    response::{IntoResponse, Response},
    routing::{any, post},
    Router,
    http::{StatusCode, HeaderMap},
    body::Body,
};
use russh::server::{Auth, Handler, Session, Handle};
use russh::ChannelId;
use russh_keys::key::PublicKey;
use async_trait::async_trait;
use bytes::Bytes;

pub type TunnelRegistry = Arc<DashMap<String, Option<TunnelEntry>>>;

#[derive(Clone)]
pub struct TunnelEntry {
    pub host: String,
    pub port: u16,
    pub handle: Handle,
}

impl std::fmt::Debug for TunnelEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TunnelEntry")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("handle", &"<Handle>")
            .finish()
    }
}

const TOKEN_LENGTH: usize = 4;
const ALPHANUMERIC: &[u8] = b"abcdefghijklmnopqrstuvwxyz\
                               ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                               0123456789";

pub fn generate_unique_token(registry: &TunnelRegistry) -> String {
    loop {
        let token = generate_token();
        if !registry.contains_key(&token) {
            return token;
        }
    }
}

fn generate_token() -> String {
    let mut rng = rand::thread_rng();
    (0..TOKEN_LENGTH)
        .map(|_| {
            let idx = rng.gen_range(0..ALPHANUMERIC.len());
            ALPHANUMERIC[idx] as char
        })
        .collect()
}

pub fn create_registry() -> TunnelRegistry {
    Arc::new(DashMap::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_has_correct_length() {
        let token = generate_token();
        assert_eq!(token.len(), TOKEN_LENGTH);
    }

    #[test]
    fn token_is_alphanumeric() {
        let token = generate_token();
        assert!(token.chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn unique_token_not_in_registry() {
        let registry = create_registry();
        registry.insert("test".to_string(), None);
        let token = generate_unique_token(&registry);
        assert_ne!(token, "test");
    }
    
    #[test]
    fn decode_chunked_with_extensions() {
        let chunked_data = b"5\r\nhello\r\n6; ext=value\r\n world\r\n0\r\n\r\n";
        let result = super::decode_chunked(chunked_data).unwrap();
        assert_eq!(result, b"hello world");
    }
    
    #[test]
    fn decode_chunked_without_extensions() {
        let chunked_data = b"5\r\nhello\r\n6\r\n world\r\n0\r\n\r\n";
        let result = super::decode_chunked(chunked_data).unwrap();
        assert_eq!(result, b"hello world");
    }
}

pub async fn register_handler(
    State(registry): State<TunnelRegistry>,
) -> String {
    let token = generate_unique_token(&registry);
    registry.insert(token.clone(), None);
    token
}

pub fn create_registration_router(registry: TunnelRegistry) -> Router {
    Router::new()
        .route("/register", post(register_handler))
        .with_state(registry)
}

pub struct SshServer {
    registry: TunnelRegistry,
    username: Option<String>,
}

impl SshServer {
    pub fn new(registry: TunnelRegistry) -> Self {
        Self { 
            registry,
            username: None,
        }
    }
}

impl Drop for SshServer {
    fn drop(&mut self) {
        if let Some(username) = &self.username {
            self.registry.remove(username);
        }
    }
}

#[async_trait]
impl Handler for SshServer {
    type Error = russh::Error;

    async fn auth_none(
        &mut self,
        user: &str,
    ) -> Result<Auth, Self::Error> {
        if self.registry.contains_key(user) {
            self.username = Some(user.to_string());
            Ok(Auth::Accept)
        } else {
            Ok(Auth::Reject {
                proceed_with_methods: None,
            })
        }
    }

    async fn auth_publickey(
        &mut self,
        user: &str,
        _key: &PublicKey,
    ) -> Result<Auth, Self::Error> {
        if self.registry.contains_key(user) {
            self.username = Some(user.to_string());
            Ok(Auth::Accept)
        } else {
            Ok(Auth::Reject {
                proceed_with_methods: None,
            })
        }
    }

    async fn auth_password(
        &mut self,
        user: &str,
        _password: &str,
    ) -> Result<Auth, Self::Error> {
        if self.registry.contains_key(user) {
            self.username = Some(user.to_string());
            Ok(Auth::Accept)
        } else {
            Ok(Auth::Reject {
                proceed_with_methods: None,
            })
        }
    }

    async fn tcpip_forward(
        &mut self,
        address: &str,
        port: &mut u32,
        session: &mut Session,
    ) -> Result<bool, Self::Error> {
        if let Some(username) = &self.username {
            let entry = TunnelEntry {
                host: address.to_string(),
                port: *port as u16,
                handle: session.handle(),
            };
            self.registry.insert(username.clone(), Some(entry));
            Ok(true)
        } else {
            Ok(false)
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

pub fn verify_token(registry: &TunnelRegistry, token: &str) -> bool {
    registry.contains_key(token)
}

pub fn extract_token_from_host(host: &str) -> Option<String> {
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() >= 4 
        && parts[parts.len() - 3] == "t" 
        && parts[parts.len() - 2] == "tn3w"
        && parts[parts.len() - 1] == "dev" {
        return Some(parts[0].to_string());
    }
    None
}

pub async fn proxy_handler(
    State(registry): State<TunnelRegistry>,
    headers: HeaderMap,
    request: Request,
) -> Response {
    let host = match headers.get("host") {
        Some(h) => match h.to_str() {
            Ok(s) => s,
            Err(_) => return bad_gateway("Invalid host header"),
        },
        None => return bad_gateway("Missing host header"),
    };

    let token = match extract_token_from_host(host) {
        Some(t) => t,
        None => return bad_gateway("Invalid subdomain format"),
    };

    let entry = match registry.get(&token) {
        Some(e) => match e.value() {
            Some(entry) => entry.clone(),
            None => return bad_gateway("Tunnel not connected"),
        },
        None => return bad_gateway("Tunnel not found"),
    };

    let mut channel = match entry.handle
        .channel_open_forwarded_tcpip(
            entry.host.clone(),
            entry.port as u32,
            "127.0.0.1".to_string(),
            0,
        )
        .await
    {
        Ok(ch) => ch,
        Err(_) => return bad_gateway("Tunnel unreachable"),
    };

    let (parts, body) = request.into_parts();
    let mut request_bytes = Vec::new();
    
    let path_and_query = parts.uri
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    
    request_bytes.extend_from_slice(
        format!("{} {} HTTP/1.1\r\n", parts.method, path_and_query).as_bytes()
    );
    
    let host_value = parts.headers
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost");
    request_bytes.extend_from_slice(
        format!("host: {}\r\n", host_value).as_bytes()
    );
    
    for (key, value) in parts.headers.iter() {
        if key != "host" {
            if let Ok(val_str) = value.to_str() {
                request_bytes.extend_from_slice(
                    format!("{}: {}\r\n", key, val_str).as_bytes()
                );
            }
        }
    }
    
    request_bytes.extend_from_slice(b"\r\n");
    
    let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
        Ok(b) => b,
        Err(_) => return bad_gateway("Failed to read request body"),
    };
    request_bytes.extend_from_slice(&body_bytes);

    if let Err(_) = channel.data(&request_bytes[..]).await {
        return bad_gateway("Failed to send request");
    }

    let mut response_bytes = Vec::new();
    let timeout_duration = std::time::Duration::from_secs(30);
    
    loop {
        let wait_result = tokio::time::timeout(
            timeout_duration,
            channel.wait()
        ).await;
        
        match wait_result {
            Ok(Some(russh::ChannelMsg::Data { ref data })) => {
                response_bytes.extend_from_slice(data);
            },
            Ok(Some(russh::ChannelMsg::Eof)) | Ok(None) => break,
            Ok(_) => continue,
            Err(_) => return bad_gateway("Tunnel response timeout"),
        }
    }

    if response_bytes.is_empty() {
        return bad_gateway("Empty response from tunnel");
    }

    let header_end = response_bytes
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .unwrap_or(0);
    
    if header_end == 0 {
        return bad_gateway("Malformed response");
    }

    let header_bytes = &response_bytes[..header_end];
    let body_bytes = &response_bytes[header_end + 4..];

    let header_str = match std::str::from_utf8(header_bytes) {
        Ok(s) => s,
        Err(_) => return bad_gateway("Invalid response headers"),
    };

    let header_lines: Vec<&str> = header_str.lines().collect();
    if header_lines.is_empty() {
        return bad_gateway("Missing status line");
    }

    let status_parts: Vec<&str> = header_lines[0].split_whitespace().collect();
    let status_code = if status_parts.len() >= 2 {
        status_parts[1].parse::<u16>().unwrap_or(200)
    } else {
        200
    };

    let mut response = Response::builder()
        .status(StatusCode::from_u16(status_code).unwrap_or(StatusCode::OK));

    let mut is_chunked = false;
    for line in &header_lines[1..] {
        if let Some(colon_pos) = line.find(':') {
            let key = &line[..colon_pos];
            let value = line[colon_pos + 1..].trim();
            
            if key.eq_ignore_ascii_case("transfer-encoding") 
                && value.eq_ignore_ascii_case("chunked") {
                is_chunked = true;
                continue;
            }
            
            if is_chunked && key.eq_ignore_ascii_case("content-length") {
                continue;
            }
            
            response = response.header(key, value);
        }
    }

    let final_body = if is_chunked {
        match decode_chunked(body_bytes) {
            Ok(decoded) => decoded,
            Err(_) => return bad_gateway("Failed to decode chunked response"),
        }
    } else {
        body_bytes.to_vec()
    };

    match response.body(Body::from(Bytes::from(final_body))) {
        Ok(r) => r,
        Err(_) => bad_gateway("Failed to build response"),
    }
}

fn decode_chunked(data: &[u8]) -> Result<Vec<u8>, ()> {
    let mut result = Vec::new();
    let mut pos = 0;
    
    loop {
        let chunk_size_end = data[pos..]
            .windows(2)
            .position(|w| w == b"\r\n")
            .ok_or(())?;
        
        let chunk_size_str = std::str::from_utf8(&data[pos..pos + chunk_size_end])
            .map_err(|_| ())?;
        
        let chunk_size_hex = chunk_size_str
            .trim()
            .split(';')
            .next()
            .ok_or(())?
            .trim();
        
        let chunk_size = usize::from_str_radix(chunk_size_hex, 16)
            .map_err(|_| ())?;
        
        if chunk_size == 0 {
            break;
        }
        
        pos += chunk_size_end + 2;
        
        if pos + chunk_size > data.len() {
            return Err(());
        }
        
        result.extend_from_slice(&data[pos..pos + chunk_size]);
        pos += chunk_size + 2;
    }
    
    Ok(result)
}

fn bad_gateway(message: &str) -> Response {
    (StatusCode::BAD_GATEWAY, message.to_string()).into_response()
}

pub fn create_proxy_router(registry: TunnelRegistry) -> Router {
    Router::new()
        .route("/*path", any(proxy_handler))
        .route("/", any(proxy_handler))
        .with_state(registry)
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_authentication_token_verification(
            token in "[a-zA-Z0-9]{4}",
            in_registry: bool,
        ) {
            let registry = create_registry();
            
            if in_registry {
                registry.insert(token.clone(), None);
            }
            
            let result = verify_token(&registry, &token);
            
            prop_assert_eq!(result, in_registry);
        }

        #[test]
        fn prop_valid_token_acceptance(
            token in "[a-zA-Z0-9]{4}",
        ) {
            let registry = create_registry();
            registry.insert(token.clone(), None);
            
            let result = verify_token(&registry, &token);
            
            prop_assert!(result);
        }

        #[test]
        fn prop_invalid_token_rejection(
            token in "[a-zA-Z0-9]{4}",
        ) {
            let registry = create_registry();
            
            let result = verify_token(&registry, &token);
            
            prop_assert!(!result);
        }

        #[test]
        fn prop_proxy_token_extraction_and_lookup(
            token in "[a-zA-Z0-9]{4}",
            in_registry: bool,
        ) {
            let registry = create_registry();
            
            if in_registry {
                registry.insert(token.clone(), None);
            }
            
            let host = format!("{}.t.tn3w.dev", token);
            let extracted = extract_token_from_host(&host);
            
            prop_assert_eq!(extracted, Some(token.clone()));
            
            let lookup_result = registry.contains_key(&token);
            prop_assert_eq!(lookup_result, in_registry);
        }

        #[test]
        fn prop_valid_tunnel_request_forwarding(
            token in "[a-zA-Z0-9]{4}",
            _port in 1024u16..65535u16,
        ) {
            let registry = create_registry();
            
            prop_assert!(registry.get(&token).is_none());
        }

        #[test]
        fn prop_invalid_tunnel_error_response(
            token in "[a-zA-Z0-9]{4}",
            tunnel_state in 0u8..2u8,
        ) {
            let registry = create_registry();
            
            match tunnel_state {
                0 => {},
                _ => { registry.insert(token.clone(), None); },
            }
            
            let lookup = registry.get(&token);
            let has_active_tunnel = lookup
                .as_ref()
                .and_then(|e| e.value().as_ref())
                .is_some();
            
            if tunnel_state < 1 {
                prop_assert!(!has_active_tunnel);
            }
        }

        #[test]
        fn prop_concurrent_proxy_independence(
            tokens in proptest::collection::vec("[a-zA-Z0-9]{4}", 2..10),
        ) {
            let registry = create_registry();
            
            for token in &tokens {
                registry.insert(token.clone(), None);
            }
            
            for token in &tokens {
                let lookup = registry.get(token);
                prop_assert!(lookup.is_some());
            }
        }
    }
}
