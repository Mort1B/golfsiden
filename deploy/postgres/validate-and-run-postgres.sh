#!/bin/sh
set -eu

: "${POSTGRES_USER:?POSTGRES_USER is required}"
: "${POSTGRES_PASSWORD:?POSTGRES_PASSWORD is required}"
: "${APP_DATABASE_USER:?APP_DATABASE_USER is required}"
: "${APP_DATABASE_PASSWORD:?APP_DATABASE_PASSWORD is required}"

if [ "$POSTGRES_USER" = "$APP_DATABASE_USER" ]; then
    echo "APP_DATABASE_USER must differ from the PostgreSQL owner" >&2
    exit 1
fi
if [ "$POSTGRES_PASSWORD" = "$APP_DATABASE_PASSWORD" ]; then
    echo "database owner and runtime passwords must differ" >&2
    exit 1
fi
case "$POSTGRES_PASSWORD:$APP_DATABASE_PASSWORD" in
    *replace-with-*|golf:golf)
        echo "replace the example database credentials before startup" >&2
        exit 1
        ;;
esac
if [ "${#POSTGRES_PASSWORD}" -lt 24 ] || [ "${#APP_DATABASE_PASSWORD}" -lt 24 ]; then
    echo "database passwords must contain at least 24 characters" >&2
    exit 1
fi

exec docker-entrypoint.sh "$@"
