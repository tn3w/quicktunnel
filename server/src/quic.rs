use crate::{
    Registry, Tunnel, TunnelHandle, register_new_tunnel,
    tunnel_domain,
};
use quinn::crypto::rustls::QuicServerConfig;
use rcgen::generate_simple_self_signed;
use std::{net::SocketAddr, sync::Arc};

pub fn quic_port() -> u16 {
    std::env::var("QUIC_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(4433)
}

pub async fn serve_quic(
    registry: Registry,
    port: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cert = generate_simple_self_signed(vec![tunnel_domain()])?;
    let cert_der =
        rustls::pki_types::CertificateDer::from(cert.cert.der().to_vec());
    let key_der =
        rustls::pki_types::PrivatePkcs8KeyDer::from(
            cert.signing_key.serialize_der(),
        )
        .into();

    let server_crypto = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der], key_der)?;

    let server_config = quinn::ServerConfig::with_crypto(
        Arc::new(QuicServerConfig::try_from(server_crypto)?),
    );
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    let endpoint = quinn::Endpoint::server(server_config, addr)?;

    println!("QUIC server listening on {addr}");

    while let Some(incoming) = endpoint.accept().await {
        let registry = registry.clone();
        tokio::spawn(async move {
            if let Ok(conn) = incoming.await {
                handle_connection(conn, registry).await;
            }
        });
    }

    Ok(())
}

async fn read_control_port(
    recv: &mut quinn::RecvStream,
) -> Option<u16> {
    let mut buf = Vec::new();

    loop {
        let mut byte = [0u8; 1];
        match recv.read(&mut byte).await {
            Ok(Some(1)) if byte[0] == b'\n' => break,
            Ok(Some(1)) => {
                buf.push(byte[0]);
                if buf.len() > 10 {
                    return None;
                }
            }
            _ => return None,
        }
    }

    String::from_utf8(buf).ok()?.parse().ok()
}

async fn handle_connection(
    connection: quinn::Connection,
    registry: Registry,
) {
    let (mut control_send, mut control_recv) =
        match connection.accept_bi().await {
            Ok(pair) => pair,
            Err(_) => return,
        };

    let port = match read_control_port(&mut control_recv).await {
        Some(p) => p,
        None => return,
    };

    let token = register_new_tunnel(&registry);
    let url = format!("https://{}.{}", token, tunnel_domain());
    println!("QUIC tunnel connected: {url}");

    let tunnel = Tunnel {
        host: "127.0.0.1".to_string(),
        port,
        handle: TunnelHandle::Quic(connection.clone()),
    };

    registry
        .write()
        .unwrap()
        .insert(token.clone(), Some(tunnel));

    let banner = format!("{url}\n");
    let _ = control_send.write_all(banner.as_bytes()).await;
    let _ = control_send.finish();

    let mut buf = [0u8; 1];
    while let Ok(Some(_)) = control_recv.read(&mut buf).await {}

    println!("QUIC tunnel disconnected: {url}");
    registry.write().unwrap().remove(&token);
}
