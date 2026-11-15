h={host}
command -v openssl >/dev/null || { echo "quicktunnel needs openssl" >&2; exit 1; }
echo "Connecting localhost:{port} to $h ..."
upgrade="GET / HTTP/1.1\r\nHost: $h\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
Sec-WebSocket-Key: $(openssl rand -base64 16)\r\nSec-WebSocket-Version: 13\r\n\r\n"
connect="openssl s_client -quiet -verify_quiet -connect $h:443 -servername $h"
known_hosts=$(mktemp)
trap 'rm -f "$known_hosts"' EXIT INT TERM
echo "$h {key}" >"$known_hosts"
ssh -NR80:localhost:{port} -oExitOnForwardFailure=yes -oServerAliveInterval=30 \
  -oUserKnownHostsFile="$known_hosts" -oUpdateHostKeys=no -oBatchMode=yes \
  -oConnectTimeout=15 -oProxyCommand="sh -c \"(printf '$upgrade';cat)|$connect\"" $h \
  || echo "Could not connect, try again in a moment." >&2
