#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
    echo "usage: $0 ENV_FILE OUTPUT.dump" >&2
    exit 2
fi

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
environment_file=$1
output_file=$2
output_directory=$(dirname -- "$output_file")
temporary_file=$(mktemp "$output_directory/.golfsiden-backup.XXXXXX")
trap 'rm -f "$temporary_file"' EXIT HUP INT TERM

docker compose \
    --env-file "$environment_file" \
    -f "$repository_root/compose.production.yml" \
    exec -T postgres sh -eu -c \
    'exec pg_dump --username="$POSTGRES_USER" --dbname="$POSTGRES_DB" --format=custom --no-owner --no-privileges' \
    >"$temporary_file"

test -s "$temporary_file"
mv "$temporary_file" "$output_file"
trap - EXIT HUP INT TERM
output_basename=$(basename -- "$output_file")
(
    cd "$output_directory"
    sha256sum "$output_basename" >"$output_basename.sha256"
)
echo "Backup written to $output_file"
