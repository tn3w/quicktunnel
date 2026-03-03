<div align="center">

# 🔁 QuickTunnel

**SSH reverse tunneling — no install required**

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg?style=for-the-badge)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg?style=for-the-badge)](https://www.rust-lang.org)
[![Docker](https://img.shields.io/badge/docker-ready-blue.svg?style=for-the-badge)](Dockerfile)
[![SSH](https://img.shields.io/badge/transport-SSH-black.svg?style=for-the-badge)](#)

<br>

```bash
ssh -oStrictHostKeyChecking=no -NR 80:localhost:3000 t.tn3w.dev
```

**Your local server is now public. That's it.**

<sub>No downloads &nbsp;&bull;&nbsp; No accounts &nbsp;&bull;&nbsp; No configuration</sub>

<br>

[**Live Demo**](https://t.tn3w.dev) &nbsp;&bull;&nbsp; [**Issues**](https://github.com/tn3w/quicktunnel/issues) &nbsp;&bull;&nbsp; [**Self-Host**](#self-hosting)

</div>

<br>

## Why QuickTunnel

</div>

<table>
<tr>
<td width="33%" valign="top">

### Zero Install

SSH comes pre-installed on Linux, macOS, and Windows 10+. Nothing to download, nothing to trust, nothing to update.

<sub>No binary</sub>

</td>
<td width="33%" valign="top">

### Encrypted

All traffic flows through SSH, the same battle-tested protocol securing servers worldwide since 1995.

<sub>SSH TLS</sub>

</td>
<td width="33%" valign="top">

### Any Protocol

HTTP, WebSocket, gRPC — if your app speaks TCP, the tunnel carries it. No restrictions.

<sub>TCP native</sub>

</td>
</tr>
<tr>
<td width="33%" valign="top">

### No Account

No sign-up form, no email confirmation, no OAuth. Just a command. Anonymous by default.

<sub>Privacy first</sub>

</td>
<td width="33%" valign="top">

### Open Source

Fully auditable server code. Run your own instance if you need total control. No black boxes.

<sub>Apache-2.0</sub>

</td>
<td width="33%" valign="top">

### Sub-Second

The tunnel is live in under a second. No handshake dance, no dashboard to navigate.

<sub>&lt; 1s startup</sub>

</td>
</tr>
</table>

<br>

## How It Works

**STEP 01 — Run the command**

```bash
ssh -oStrictHostKeyChecking=no \
  -NR 80:localhost:3000 \
  t.tn3w.dev
```

SSH is pre-installed on all major operating systems. No extra software required.

**STEP 02 — Get your URL**

```
┌─────────────────────────────────────────────┐
│  QuickTunnel  ▸  https://abc123.t.tn3w.dev  │
└─────────────────────────────────────────────┘
```

The server responds instantly with a unique public URL. Share it with anyone.

**STEP 03 — Close to stop**

```bash
^C
# tunnel closed
# URL gone
```

Hit `Ctrl+C` to terminate. No dangling processes, no data retained.

<br>

## Usage

**Change the port to match your local server:**

```bash
# React / Next.js (3000)
ssh -oStrictHostKeyChecking=no -NR 80:localhost:3000 t.tn3w.dev

# Vite (5173)
ssh -oStrictHostKeyChecking=no -NR 80:localhost:5173 t.tn3w.dev

# API server (8080)
ssh -oStrictHostKeyChecking=no -NR 80:localhost:8080 t.tn3w.dev

# Flask / Django (5000)
ssh -oStrictHostKeyChecking=no -NR 80:localhost:5000 t.tn3w.dev

# Webhook receiver
ssh -oStrictHostKeyChecking=no -NR 80:localhost:4000 t.tn3w.dev
```

The pattern is always: `-NR 80:localhost:<YOUR_PORT> t.tn3w.dev`

<br>

## Comparison

| Feature | **QuickTunnel** | ngrok | cloudflared | localtunnel |
|:--------|:---:|:---:|:---:|:---:|
| No install required | **Yes** | Binary | Binary | npm |
| No account needed | **Yes** | Required | Yes | Yes |
| Open source server | **Yes** | No | Partial | Yes |
| Encrypted transport | **SSH** | TLS | QUIC/HTTP2 | TLS |
| WebSocket support | **Yes** | Yes | Yes | Partial |
| Custom subdomains | Planned | Paid | No | Unstable |

<br>

## Architecture

QuickTunnel is a single Rust binary (`main.rs`) running three servers:

<table>
<tr>
<td width="33%" valign="top">

**SSH Server** — `:22`

Accepts reverse tunnel connections via [`russh`](https://github.com/Eugeny/russh). On connect, generates a unique 6-character token, displays the public URL as an SSH banner, and registers the tunnel in a shared registry.

</td>
<td width="33%" valign="top">

**Proxy Server** — `:8080`

Handles all `*.t.tn3w.dev` HTTP requests. Extracts the token from the subdomain, looks up the tunnel in the registry, opens a forwarded TCP channel over SSH, and proxies the request/response.

</td>
<td width="33%" valign="top">

**Index Server** — `:3000`

Serves the landing page (`index.html`) with an interactive port picker and live command generation.

</td>
</tr>
</table>

**Request flow:**

```
Client → abc123.t.tn3w.dev → Proxy :8080 → Registry lookup
       → SSH channel → localhost:3000 → response back
```

**Key implementation details:**

- **Registry** — `Arc<RwLock<HashMap<String, Option<Tunnel>>>>` maps tokens to tunnel metadata. Auto-cleanup on SSH disconnect via `Drop`.
- **Token generation** — 6-character alphanumeric token using `OsRng` for cryptographically secure randomness. Collision-checked against the registry.
- **Chunked transfer encoding** — Proxy decodes chunked HTTP responses from upstream before forwarding to the client.
- **Limits** — Request body: 10 MB. Response body: 50 MB. Response timeout: 30 seconds.
- **Host key** — Ed25519, persisted to `/app/keys/ssh_host_ed25519_key`. Generated on first run.
- **KEX** — `mlkem768x25519-sha256` (post-quantum hybrid) via `russh::Preferred`.
- **Auth** — `auth_none`, `auth_password`, and `auth_publickey` all return `Accept`. No credentials required.

<br>

## Self-Hosting

**Docker**

```bash
docker build -t quicktunnel .
docker run \
  -p 22:22 \
  -p 80:8080 \
  -p 3000:3000 \
  -e TUNNEL_DOMAIN=yourdomain.com \
  quicktunnel
```

**Docker Compose**

```bash
docker-compose up -d
```

Edit `docker-compose.yml` to configure ports, domain, and key volume:

```yaml
environment:
  - TUNNEL_DOMAIN=yourdomain.com  # Change to your domain
```

**From Source**

```bash
cargo build --release
./target/release/quicktunnel
```

Requires Rust 1.70+ and OpenSSL dev libs.

**Port map:**

| Port | Service | Purpose |
|:----:|:--------|:--------|
| `22` | SSH Server | Accepts tunnel connections |
| `8080` | Proxy Server | Handles inbound HTTP (map to `80`/`443`) |
| `3000` | Index Server | Landing page |

**Custom domain setup:**

1. Set the `TUNNEL_DOMAIN` environment variable to your domain (e.g., `TUNNEL_DOMAIN=yourdomain.com`)
2. Add a DNS wildcard record: `*.yourdomain.com → your-server-ip`
3. (Optional) Place a TLS-terminating reverse proxy (e.g. Caddy, nginx) in front of `:8080`

The domain defaults to `t.tn3w.dev` if not specified.
