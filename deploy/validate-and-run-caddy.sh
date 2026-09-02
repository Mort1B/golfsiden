#!/bin/sh
set -eu

: "${SITE_ADDRESS:?SITE_ADDRESS is required}"
: "${PROXY_SHARED_SECRET:?PROXY_SHARED_SECRET is required}"
case "$SITE_ADDRESS" in
    *://*|*/*|*:*|*[!A-Za-z0-9.-]*|.*|*.)
        echo "SITE_ADDRESS must be a hostname without a scheme, path, or port" >&2
        exit 1
        ;;
esac
if [ "${#PROXY_SHARED_SECRET}" -ne 43 ]; then
    echo "PROXY_SHARED_SECRET must be a 43-character base64url value" >&2
    exit 1
fi
case "$PROXY_SHARED_SECRET" in
    *[!A-Za-z0-9_-]*)
        echo "PROXY_SHARED_SECRET must be a 43-character base64url value" >&2
        exit 1
        ;;
esac

exec caddy run --config /etc/caddy/Caddyfile --adapter caddyfile
