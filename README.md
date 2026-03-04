<div align="center">

# 🔁 QuickTunnel

**SSH reverse tunneling — no install required**

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg?style=for-the-badge)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg?style=for-the-badge)](https://www.rust-lang.org)
[![Docker](https://img.shields.io/badge/docker-ready-blue.svg?style=for-the-badge)](Dockerfile)
[![SSH](https://img.shields.io/badge/transport-SSH-black.svg?style=for-the-badge)](#)

<br>

<pre align="center">
ssh -oStrictHostKeyChecking=no -NR 80:localhost:3000 t.tn3w.dev
</pre>

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

**STEP 01 — Run command**

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
| Encrypted transport |     **SSH**     |   TLS    | QUIC/HTTP2  |     TLS     |
| WebSocket support   |     **Yes**     |   Yes    |     Yes     |   Partial   |
| Custom subdomains   |     Planned     |   Paid   |     No      |  Unstable   |

<br>

## Architecture

QuickTunnel is a single Rust binary running three servers:

<table>
<tr>
<td width="33%" valign="top">

**SSH Server** — `:22`

Accepts reverse tunnel connections via [`russh`](https://github.com/Eugeny/russh). On connect, generates a unique 6-character token, displays the public URL as an SSH banner, and registers the tunnel in a shared registry.

</td>
<td width="33%" valign="top">

**Proxy Server** — `:8080`

Handles all `*.t.tn3w.dev` HTTP requests. Extracts the token from the subdomain, looks up the tunnel in the registry, opens a forwarded TCP channel over SSH, and proxies the request/response. Returns detailed error pages for common failure scenarios.

</td>
<td width="33%" valign="top">

**Index Server** — `:3000`

Serves the landing page (`index.html`) with an interactive port picker and live command generation. Returns a 404 page for non-existent routes. Can be disabled via `INDEX_ENABLED=false` if only tunnel functionality is needed.

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
- **Error handling** — Dedicated error module (`errors.rs`) provides detailed, user-friendly error pages with visual connection flow diagrams and troubleshooting steps for developers and visitors.
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
  -e INDEX_PORT=3000 \
  -e PROXY_PORT=8080 \
  -e SSH_PORT=22 \
  -e INDEX_ENABLED=true \
  quicktunnel
```

Or with custom ports:

```bash
docker run \
  -p 2222:2222 \
  -p 80:9090 \
  -p 4000:4000 \
  -e TUNNEL_DOMAIN=yourdomain.com \
  -e INDEX_PORT=4000 \
  -e PROXY_PORT=9090 \
  -e SSH_PORT=2222 \
  -e INDEX_ENABLED=true \
  quicktunnel
```

**Docker Compose**

```bash
docker-compose up -d
```

Edit `docker-compose.yml` or create a `.env` file to configure ports and domain:

```yaml
environment:
    - TUNNEL_DOMAIN=yourdomain.com # Change to your domain
    - INDEX_PORT=3000 # Landing page port (default: 3000)
    - PROXY_PORT=8080 # Proxy server port (default: 8080)
    - SSH_PORT=22 # SSH tunnel port (default: 22)
    - INDEX_ENABLED=true # Enable landing page (default: true)
```

Or use a `.env` file:

```env
TUNNEL_DOMAIN=yourdomain.com
INDEX_PORT=3000
PROXY_PORT=8080
SSH_PORT=22
INDEX_ENABLED=true
```

Set `INDEX_ENABLED=false` to disable the landing page server if you only need the tunnel functionality.

Port mappings automatically sync with the environment variables, so you only need to define them once.

**From Source**

```bash
cargo build --release
./target/release/quicktunnel
```

Requires Rust 1.70+ and OpenSSL dev libs.

**Building Templates**

To rebuild the landing page after making changes to `templates/index.html`, `static/app.js`, or `static/style.css`:

```bash
npx -y html-build-tool
```

The `-y` flag automatically accepts the package installation prompt. This tool minifies HTML/CSS/JS, inlines local resources, and generates Subresource Integrity (SRI) hashes for security. The built output is written to `dist/`.

**Port configuration:**

|  Port  | Env Variable | Service      | Purpose                                  |
| :----: | :----------- | :----------- | :--------------------------------------- |
|  `22`  | `SSH_PORT`   | SSH Server   | Accepts tunnel connections               |
| `8080` | `PROXY_PORT` | Proxy Server | Handles inbound HTTP (map to `80`/`443`) |
| `3000` | `INDEX_PORT` | Index Server | Landing page                             |

All ports can be customized via environment variables. Defaults are shown above.

**Custom domain setup:**

1. Set the `TUNNEL_DOMAIN` environment variable to your domain (e.g., `TUNNEL_DOMAIN=yourdomain.com`)
2. Add a DNS wildcard record: `*.yourdomain.com → your-server-ip`
3. (Optional) Place a TLS-terminating reverse proxy (e.g. Caddy, nginx) in front of `:8080`

The domain defaults to `t.tn3w.dev` if not specified.
