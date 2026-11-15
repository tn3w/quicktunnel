h={host}
upgrade="GET / HTTP/1.1\r\nHost: $h\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
Sec-WebSocket-Key: $(openssl rand -base64 16)\r\nSec-WebSocket-Version: 13\r\n\r\n"
connect="openssl s_client -quiet -connect $h:443 -servername $h 2>/dev/null"
known_hosts=$(mktemp)
trap 'rm -f "$known_hosts"' EXIT INT TERM
echo "$h {key}" >"$known_hosts"
ssh -NR80:localhost:{port} -oExitOnForwardFailure=yes -oServerAliveInterval=30 \
  -oUserKnownHostsFile="$known_hosts" -oUpdateHostKeys=no -oBatchMode=yes \
  -oProxyCommand="sh -c \"(printf '$upgrade';cat)|$connect\"" $h
