use std::sync::Arc;
use tokio::net::TcpListener;
use russh::server::{Config, Server};
use russh_keys::key::KeyPair;
use tunnel_server::{create_registry, create_registration_router, create_proxy_router, SshServer};

struct TunnelServer {
    registry: tunnel_server::TunnelRegistry,
}

impl Server for TunnelServer {
    type Handler = SshServer;

    fn new_client(&mut self, _peer_addr: Option<std::net::SocketAddr>) -> Self::Handler {
        SshServer::new(self.registry.clone())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = create_registry();

    let registration_router = create_registration_router(registry.clone());
    let registration_listener = TcpListener::bind("0.0.0.0:3000").await?;
    println!("Registration listening on :3000");

    let proxy_router = create_proxy_router(registry.clone());
    let proxy_listener = TcpListener::bind("0.0.0.0:8080").await?;
    println!("Proxy listening on :8080");

    let ssh_config = Arc::new(Config {
        inactivity_timeout: Some(std::time::Duration::from_secs(3600)),
        auth_rejection_time: std::time::Duration::from_secs(3),
        auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
        keys: vec![KeyPair::generate_ed25519().unwrap()],
        ..Default::default()
    });

    let mut tunnel_server = TunnelServer { registry };
    println!("SSH listening on :22");

    tokio::try_join!(
        tokio::spawn(async move {
            axum::serve(registration_listener, registration_router)
                .await
                .expect("Registration server failed");
        }),
        tokio::spawn(async move {
            axum::serve(proxy_listener, proxy_router)
                .await
                .expect("Proxy server failed");
        }),
        tokio::spawn(async move {
            tunnel_server
                .run_on_address(ssh_config, ("0.0.0.0", 22))
                .await
                .expect("SSH server failed");
        }),
    )?;

    Ok(())
}
