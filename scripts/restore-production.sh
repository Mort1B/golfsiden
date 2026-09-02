#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
    echo "usage: $0 ENV_FILE INPUT.dump" >&2
    exit 2
fi
if [ "${CONFIRM_EMPTY_RESTORE:-}" != "RESTORE_TO_EMPTY_DATABASE" ]; then
    echo "set CONFIRM_EMPTY_RESTORE=RESTORE_TO_EMPTY_DATABASE after verifying the target" >&2
    exit 2
fi

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
environment_file=$1
input_file=$2
input_directory=$(dirname -- "$input_file")
input_basename=$(basename -- "$input_file")

compose() {
    docker compose \
        --env-file "$environment_file" \
        -f "$repository_root/compose.production.yml" \
        "$@"
}

test -s "$input_file"
if [ -f "$input_file.sha256" ]; then
    (
        cd "$input_directory"
        sha256sum --check "$input_basename.sha256"
    )
fi

# The target must have a pristine public schema. Count relations, routines, and
# standalone types so a partial earlier restore cannot pass this guard.
object_count=$(compose exec -T postgres sh -eu -c \
    'psql --username="$POSTGRES_USER" --dbname="$POSTGRES_DB" --tuples-only --no-align --command="SELECT (SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace WHERE n.nspname = '\''public'\'') + (SELECT count(*) FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace WHERE n.nspname = '\''public'\'') + (SELECT count(*) FROM pg_type t JOIN pg_namespace n ON n.oid = t.typnamespace WHERE n.nspname = '\''public'\'')"')
if [ "$object_count" -ne 0 ]; then
    echo "refusing restore: target public schema contains $object_count objects" >&2
    exit 1
fi

compose stop web api >/dev/null 2>&1 || true
compose exec -T postgres sh -eu -c \
    'exec pg_restore --username="$POSTGRES_USER" --dbname="$POSTGRES_DB" --exit-on-error --single-transaction --no-owner --no-privileges' \
    <"$input_file"
compose --profile tools run --rm permissions
echo "Restore complete; start api and web, then verify /api/ready and tournament state."
