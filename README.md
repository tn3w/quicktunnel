<div align="center">

# 🔁 QuickTunnel

**Fast reverse tunneling — via QUIC protocol or SSH**

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg?style=for-the-badge)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg?style=for-the-badge)](https://www.rust-lang.org)
[![Docker](https://img.shields.io/badge/docker-ready-blue.svg?style=for-the-badge)](Dockerfile)
[![Protocol](https://img.shields.io/badge/transport-QUIC%20%7C%20SSH-black.svg?style=for-the-badge)](#)

<br>

<pre align="center">
ssh -oStrictHostKeyChecking=no -NR 80:localhost:3000 t.tn3w.dev
</pre>

**Your local server is now public. That's it.**

<sub>Multiple clients &nbsp;&bull;&nbsp; No accounts &nbsp;&bull;&nbsp; No configuration</sub>

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

### Two ways to connect

All traffic flows through SSH, the same battle-tested protocol securing servers worldwide since 1995. Or, use the optimized `qt` CLI for blazing-fast QUIC speeds.

<sub>Flexible & Secure</sub>

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

**STEP 01 — Run command**

Using the fast QUIC client:

```bash
qt 3000
```

_Or_, using standard SSH (no install required):

```bash
ssh -oStrictHostKeyChecking=no -NR 80:localhost:3000 t.tn3w.dev
```

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

### Using the QUIC `qt` client

Download or build the CLI from `client/`.

```bash
qt 3000
```

You can configure the server connection details by setting `QT_SERVER` (default `127.0.0.1:4433`) and `QT_SERVER_NAME` (default `t.tn3w.dev`).

### Using standard SSH

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

**Create a shortcut (optional):**

```bash
# bash / sh
grep -qxF 'qtnl(){ ssh -oStrictHostKeyChecking=no -NR 80:localhost:"$1" t.tn3w.dev; }' ~/.bashrc || echo 'qtnl(){ ssh -oStrictHostKeyChecking=no -NR 80:localhost:"$1" t.tn3w.dev; }' >> ~/.bashrc

# zsh
grep -qxF 'qtnl(){ ssh -oStrictHostKeyChecking=no -NR 80:localhost:"$1" t.tn3w.dev; }' ~/.zshrc || echo 'qtnl(){ ssh -oStrictHostKeyChecking=no -NR 80:localhost:"$1" t.tn3w.dev; }' >> ~/.zshrc

# fish
grep -qx 'function qtnl; ssh -oStrictHostKeyChecking=no -NR 80:localhost:$argv[1] t.tn3w.dev; end' ~/.config/fish/config.fish || echo 'function qtnl; ssh -oStrictHostKeyChecking=no -NR 80:localhost:$argv[1] t.tn3w.dev; end' >> ~/.config/fish/config.fish

# csh / tcsh
grep -q 'alias qtnl' ~/.cshrc || echo 'alias qtnl ssh -oStrictHostKeyChecking=no -NR 80:localhost:\!^ t.tn3w.dev' >> ~/.cshrc

# powershell
if(!(Select-String -Quiet 'qtnl' $PROFILE 2>$null)){ Add-Content $PROFILE 'function qtnl($p){ ssh -oStrictHostKeyChecking=no -NR 80:localhost:$p t.tn3w.dev }' }
```

Then use: `qtnl 3000` instead of the full SSH command.

**Quick HTTP server + tunnel (no local server needed):**

If you don't have a local server running, you can start one and tunnel it in a single command:

##### Linux:

```bash
# Python
python3 -m http.server 8080 & ssh -oStrictHostKeyChecking=no -NR 80:localhost:8080 t.tn3w.dev

# Node.js
npx serve . -l 8080 & ssh -oStrictHostKeyChecking=no -NR 80:localhost:8080 t.tn3w.dev

# Pure bash (no dependencies)
(p=8080; while true; do { echo -e "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n"; cat index.html; } | nc -l -q1 $p; done) & ssh -oStrictHostKeyChecking=no -NR 80:localhost:8080 t.tn3w.dev
```

##### macOS:

```bash
# Python
python3 -m http.server 8080 & ssh -oStrictHostKeyChecking=no -NR 80:localhost:8080 t.tn3w.dev

# Node.js
npx serve . -l 8080 & ssh -oStrictHostKeyChecking=no -NR 80:localhost:8080 t.tn3w.dev

# Ruby
ruby -run -e httpd . -p 8080 & ssh -oStrictHostKeyChecking=no -NR 80:localhost:8080 t.tn3w.dev
```

##### Windows:

```powershell
# Python
Start-Process python3 -ArgumentList "-m", "http.server", "8080" -NoNewWindow; ssh -oStrictHostKeyChecking=no -NR 80:localhost:8080 t.tn3w.dev

# Node.js
Start-Process npx -ArgumentList "serve", ".", "-l", "8080" -NoNewWindow; ssh -oStrictHostKeyChecking=no -NR 80:localhost:8080 t.tn3w.dev

# Pure PowerShell (no dependencies)
Start-Job -ScriptBlock { $p=8080; $l=[Net.HttpListener]::new(); $l.Prefixes.Add("http://+:$p/"); $l.Start(); while($true){$c=$l.GetContext(); $f=Join-Path $pwd $c.Request.Url.LocalPath.TrimStart('/'); $b=if(Test-Path $f){[IO.File]::ReadAllBytes($f)}else{$c.Response.StatusCode=404;@()}; $c.Response.OutputStream.Write($b,0,$b.Length); $c.Response.Close()} }; ssh -oStrictHostKeyChecking=no -NR 80:localhost:8080 t.tn3w.dev
```

These commands start a simple HTTP server on port 8080 and immediately tunnel it. Perfect for quickly sharing static files.

<br>

## Comparison

| Feature             | **QuickTunnel** |  ngrok   | cloudflared | localtunnel |
| :------------------ | :-------------: | :------: | :---------: | :---------: |
| No install required |     **Yes**     |  Binary  |   Binary    |     npm     |
| No account needed   |     **Yes**     | Required |     Yes     |     Yes     |
| Open source server  |     **Yes**     |    No    |   Partial   |     Yes     |
| Encrypted transport | **QUIC / SSH**  |   TLS    | QUIC/HTTP2  |     TLS     |
| WebSocket support   |     **Yes**     |   Yes    |     Yes     |   Partial   |
| Custom subdomains   |     Planned     |   Paid   |     No      |  Unstable   |

<br>

## Architecture

QuickTunnel is a single Rust binary running multiple servers:

<table>
<tr>
<td width="50%" valign="top">

**QUIC Server** — `:4433`

Accepts ultra-fast UDP connections from the `qt` client using QUIC (quinn). Zero head-of-line blocking, very fast handshakes, multi-channel connections.

</td>
<td width="50%" valign="top">

**SSH Server** — `:22`

Accepts fallback reverse tunnel connections via `russh` using standard SSH clients. Generates a unique 6-character token on connect.

</td>
</tr>
<tr>
<td width="100%" valign="top" colspan="2">

**Proxy Server & Index** — `:8080`, `:3000`

Handles HTTP proxy routing and landing pages. Unpacks token, resolves tunnel (QUIC or SSH), handles connection over relevant protocol.

</td>
</tr>
</table>

**Request flow:**

```
Client → abc123.t.tn3w.dev → Proxy :8080 → Registry lookup
       → SSH channel → localhost:3000 → response back
```

**Key implementation details:**

- **Registry** — `Arc<RwLock<HashMap<String, Option<Tunnel>>>>` maps tokens to tunnel metadata
- **Token generation** — 6-character alphanumeric using `OsRng`, collision-checked
- **Limits** — Request: 10 MB, Response: 50 MB, Timeout: 30s
- **Host key** — Ed25519, persisted to `/app/keys/ssh_host_ed25519_key`
- **KEX** — `mlkem768x25519-sha256` (post-quantum hybrid)
- **Auth** — All methods accept (no credentials required)

<br>

## Self-Hosting

**Docker**

```bash
docker build -t quicktunnel .
docker run \
  -p 22:22 \
  -p 80:8080 \
  -p 3000:3000 \
  -p 4433:4433/udp \
  -e TUNNEL_DOMAIN=yourdomain.com \
  quicktunnel
```

**Docker Compose**

```bash
docker-compose up -d
```

Configure via `.env` file:

```env
TUNNEL_DOMAIN=yourdomain.com  # Your domain (default: t.tn3w.dev)
INDEX_PORT=3000               # Landing page server
PROXY_PORT=8080               # HTTP proxy (map to 80/443)
SSH_PORT=22                   # SSH tunnel listener
QUIC_PORT=4433                # QUIC tunnel listener
INDEX_ENABLED=true            # Set false to disable landing page server
```

**From Source**

```bash
cargo build --release
./target/release/quicktunnel
```

Requires Rust 1.85+.

**Building Templates**

After editing `templates/` or `static/` files, rebuild with:

```bash
npx -y html-build-tool
```

Outputs minified HTML/CSS/JS with SRI hashes to `dist/`.

**Custom Domain**

1. Set `TUNNEL_DOMAIN=yourdomain.com`
2. Add DNS wildcard: `*.yourdomain.com → your-server-ip`
3. (Optional) Add TLS proxy (Caddy/nginx) in front of `:8080`
