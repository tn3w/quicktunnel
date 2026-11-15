# quicktunnel

Share a local web server under a public HTTPS address. No account, no install:

```sh
curl -s https://t.tn3w.dev/3000|sh
```

```
  Your local server is live at https://k3x9.tn3w.dev

  Share the link, keep this open while you need it, Ctrl+C to stop.
```

Replace `3000` with your port. Works on Linux, macOS, WSL and Git Bash: it needs only
`curl`, `sh`, `ssh` and `openssl`.

## How it works

`curl` fetches a three-line script that runs your system's `ssh` with a remote forward.
`ssh` reaches the server through `openssl` on port 443: an HTTP Upgrade turns the HTTPS
connection into a plain byte stream, so it passes through Cloudflare and any proxy that
supports WebSockets. No SSH port is needed. The script pins the server's SSH host key.

Each visitor request opens one SSH channel back to your machine. Responses and request
bodies stream, and WebSocket upgrades are passed through.

Every tunnel is user content. Browsers get a warning page before the first page of a
tunnel; other clients such as `curl`, webhooks and API calls pass through directly.

## Limits

64 tunnels in total, 3 per IP address, 1 MB/s of response traffic shared by all
tunnels, 10 seconds to start a response. Change the constants in `src/main.rs`.

## Self-hosting

```sh
cargo build --release
DOMAIN=example.com PORT=8080 ./target/release/quicktunnel
```

Put a TLS proxy for `example.com` and `*.example.com` in front of the port. Users then
run `curl -s https://t.example.com/3000|sh`, and tunnels get `https://xxxx.example.com`.
Under systemd socket activation it serves the passed socket instead of `PORT`. The SSH
host key is created in `$STATE_DIRECTORY` or the working directory.

## License

Apache-2.0
