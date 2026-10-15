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
# Preserve path bytes, including trailing newlines; command substitution would
# strip them and could verify a different archive with a similar name.
case $input_file in
    */*)
        input_directory=${input_file%/*}
        input_basename=${input_file##*/}
        [ -n "$input_directory" ] || input_directory=/
        ;;
    *) input_directory=.; input_basename=$input_file ;;
esac

compose() {
    docker compose \
        --env-file "$environment_file" \
        -f "$repository_root/compose.production.yml" \
        "$@"
}

if [ ! -f "$input_file" ] || [ ! -r "$input_file" ] || [ ! -s "$input_file" ]; then
    echo "restore requires a nonempty readable archive file" >&2
    exit 1
fi
if [ ! -f "$input_file.sha256" ] || [ ! -r "$input_file.sha256" ] || [ ! -s "$input_file.sha256" ]; then
    echo "restore requires a nonempty readable checksum sidecar" >&2
    exit 1
fi
if [ "$input_basename" = "-" ]; then
    echo "restore archive basename cannot be the standard-input sentinel" >&2
    exit 1
fi
if ! command -v sha256sum >/dev/null 2>&1 || ! command -v cmp >/dev/null 2>&1; then
    echo "restore requires sha256sum and cmp" >&2
    exit 1
fi
if ! (
    # Physical traversal matches opening the original path, even through link/..
    # or an exported CDPATH. Never continue hashing if directory selection fails.
    CDPATH= cd -P -- "$input_directory" || exit 1
    checksum_record=$(sha256sum -- "$input_basename") || exit 1
    # GNU sha256sum escapes embedded newlines/backslashes into one record. Match
    # its exact canonical output, including the single final newline, so foreign
    # filenames and extra records are never interpreted or accepted.
    printf '%s\n' "$checksum_record" | cmp -s -- - "$input_basename.sha256"
) 2>/dev/null; then
    echo "checksum sidecar does not match the selected archive's canonical SHA-256 record" >&2
    exit 1
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
