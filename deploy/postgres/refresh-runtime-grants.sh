#!/bin/sh
set -eu

: "${POSTGRES_USER:?POSTGRES_USER is required}"
: "${POSTGRES_DB:?POSTGRES_DB is required}"
: "${POSTGRES_OWNER_PASSWORD:?POSTGRES_OWNER_PASSWORD is required}"
: "${APP_DATABASE_USER:?APP_DATABASE_USER is required}"
if [ "$APP_DATABASE_USER" = "$POSTGRES_USER" ]; then
    echo "APP_DATABASE_USER must differ from the PostgreSQL owner" >&2
    exit 1
fi

export PGPASSWORD="$POSTGRES_OWNER_PASSWORD"
psql --set=ON_ERROR_STOP=1 \
    --host=postgres \
    --username "$POSTGRES_USER" \
    --dbname "$POSTGRES_DB" \
    --set=app_user="$APP_DATABASE_USER" <<'SQL'
GRANT USAGE ON SCHEMA public TO :"app_user";
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO :"app_user";
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO :"app_user";
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO :"app_user";
REVOKE INSERT, UPDATE, DELETE, TRUNCATE, REFERENCES, TRIGGER
    ON TABLE public._sqlx_migrations FROM :"app_user";
GRANT SELECT ON TABLE public._sqlx_migrations TO :"app_user";
SQL
