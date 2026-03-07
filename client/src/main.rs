use quinn::RecvStream;
use quinn::crypto::rustls::QuicClientConfig;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

fn server_addr() -> String {
    std::env::var("QT_SERVER").unwrap_or_else(|_| "t.tn3w.dev:4433".to_string())
}

fn server_name() -> String {
    std::env::var("QT_SERVER_NAME").unwrap_or_else(|_| "t.tn3w.dev".to_string())
}

fn build_client_config() -> quinn::ClientConfig {
    let crypto = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(SkipVerify))
        .with_no_client_auth();

    let quic_crypto = QuicClientConfig::try_from(crypto).expect("TLS 1.3 required");

    let mut transport_config = quinn::TransportConfig::default();
    transport_config.max_idle_timeout(Some(
        std::time::Duration::from_secs(86400).try_into().unwrap(),
    ));
    transport_config.keep_alive_interval(Some(std::time::Duration::from_secs(15)));

    let mut config = quinn::ClientConfig::new(Arc::new(quic_crypto));
    config.transport_config(Arc::new(transport_config));

    config
}

#[derive(Debug)]
struct SkipVerify;

impl rustls::client::danger::ServerCertVerifier for SkipVerify {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

async fn read_line(recv: &mut RecvStream) -> Result<String, Box<dyn std::error::Error>> {
    let mut buf = Vec::new();

    loop {
        let mut byte = [0u8; 1];
        match recv.read(&mut byte).await? {
            Some(1) if byte[0] == b'\n' => break,
            Some(1) => {
                buf.push(byte[0]);
                if buf.len() > 1024 {
                    return Err("line too long".into());
                }
            }
            _ => return Err("stream closed unexpectedly".into()),
        }
    }

    Ok(String::from_utf8(buf)?)
}

async fn proxy_stream(mut send: quinn::SendStream, mut recv: quinn::RecvStream, port: u16) {
    let mut request = Vec::new();
    let mut buf = [0u8; 65536];

    loop {
        match recv.read(&mut buf).await {
            Ok(Some(n)) => request.extend_from_slice(&buf[..n]),
            Ok(None) => break,
            Err(_) => return,
        }
    }

    let mut tcp = match TcpStream::connect(format!("127.0.0.1:{port}")).await {
        Ok(s) => s,
        Err(_) => return,
    };

    if tcp.write_all(&request).await.is_err() {
        return;
    }

    let mut response = Vec::new();
    loop {
        match tcp.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => response.extend_from_slice(&buf[..n]),
            Err(_) => break,
        }
    }

    let _ = send.write_all(&response).await;
    let _ = send.finish();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port: u16 = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or_else(|| {
            eprintln!("Usage: qt <port> [handle]");
            std::process::exit(1);
        });

    let handle: Option<String> = std::env::args().nth(2).map(|h| h.to_lowercase());

    if let Some(h) = &handle {
        let is_valid_len = h.len() >= 4 && h.len() <= 63 && h.len() != 6;
        let is_valid_chars = h
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit());

        if !is_valid_len || !is_valid_chars {
            eprintln!("Invalid handle: 4-63 chars; lowercase letters or digits");
            std::process::exit(1);
        }
    }

    let addr_str = server_addr();
    let name = server_name();

    let addrs: Vec<std::net::SocketAddr> = tokio::net::lookup_host(&addr_str).await?.collect();
    let addr = addrs
        .into_iter()
        .next()
        .ok_or("Failed to resolve server domain")?;

    let mut endpoint = quinn::Endpoint::client("0.0.0.0:0".parse()?)?;
    endpoint.set_default_client_config(build_client_config());

    let connection = endpoint.connect(addr, &name)?.await?;
    let endpoint_clone = endpoint.clone();

    let (mut control_send, mut control_recv) = connection.open_bi().await?;
    let control_msg = match &handle {
        Some(h) => format!("{port} {h}\n"),
        None => format!("{port}\n"),
    };
    control_send.write_all(control_msg.as_bytes()).await?;

    let url = read_line(&mut control_recv).await?;

    let inner = format!("  QuickTunnel  ▸  {}  ", url.trim());
    let border = "─".repeat(inner.chars().count());
    let banner = format!("\r\n┌{}┐\r\n│{}│\r\n└{}┘\r\n", border, inner, border);
    println!("{banner}");
    println!("Forwarding to 127.0.0.1:{port}");

    let conn_clone = connection.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        conn_clone.close(0u32.into(), b"client disconnect");
        endpoint_clone.wait_idle().await;
        std::process::exit(0);
    });

    loop {
        match connection.accept_bi().await {
            Ok((send, recv)) => {
                tokio::spawn(proxy_stream(send, recv, port));
            }
            Err(_) => {
                eprintln!("Connection closed");
                break;
            }
        }
    }

    Ok(())
}
