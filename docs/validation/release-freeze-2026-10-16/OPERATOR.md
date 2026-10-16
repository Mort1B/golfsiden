# Frozen Sunday release — operator handoff

Verdict: **READY WITH KNOWN LIMITATIONS for the locally accepted candidate**.
Public-host deployment is not accepted yet. This is a **Linux amd64** image
bundle, schema **39**, with one common release tag recorded in `manifest.json`.
The complete application flow, production guards, logging, backup/restore and
restart/reconnect passed locally using these exact images. Native Docker,
public DNS/ACME/ports, the actual playing phone and protected off-host recovery
remain operator gates. Existing P3 Fantasy read-retry and lifecycle-test limits
remain documented in the repository release report.

## Receive and verify

Copy the bundle and its SHA-256 from the trusted staging handoff to durable
operator-controlled storage. Compare the received package with `bundle.json`
from the trusted repository release report before extracting. Checksums detect
transfer changes; they do not authenticate an untrusted replacement manifest.
Extract into a new protected directory. Keep this directory intact; do not
merge it with an older release. Python 3.11+, Docker Engine and Compose v2 are
required. The helper starts no services:

```bash
python3 verify-release.py
python3 verify-release.py --load
cp .env.production.example /PRIVATE/PATH/golfsiden.env
chmod 600 /PRIVATE/PATH/golfsiden.env
```

Set every placeholder privately. Keep the supplied `GOLFSIDEN_IMAGE_TAG`.
Use distinct owner/runtime database roles and passwords, a fresh proxy secret,
the chosen public hostname and its exact HTTPS reset origin. Do not print the
rendered Compose configuration or commit the private env file. Validate it:

```bash
export COMPOSE_PROJECT_NAME=golfsiden
python3 verify-release.py --check-env /PRIVATE/PATH/golfsiden.env --project "$COMPOSE_PROJECT_NAME"
```

The loader checks the engine architecture, refuses to replace a different
existing release alias, loads image archives and explicitly tags each exact
config ID as `golfsiden-SERVICE:RELEASE_TAG`. It does not assume Podman's
`localhost/` aliases work with Docker. These local image IDs are not registry
pull references. No image was pushed to a registry.

Use **both** Compose files below. The override fixes all five service aliases
and disables pulls. Never build this frozen candidate; a rebuild produces a
new candidate requiring affected acceptance. The archive includes runtime
files only, so a build is also unsupported. Avoid `--build`, custom overrides,
unexpected project-name changes, and `docker compose down -v` on retained data.

## New installation

From the extracted directory, after preparing DNS, TCP 80/443, optional UDP443,
durable PostgreSQL/Caddy storage and the off-host backup destination:

```bash
docker compose --env-file /PRIVATE/PATH/golfsiden.env -f compose.production.yml -f compose.release.json up -d --wait --no-build --pull never postgres
docker compose --env-file /PRIVATE/PATH/golfsiden.env -f compose.production.yml -f compose.release.json --profile tools run --rm --no-deps --pull never migrate
docker compose --env-file /PRIVATE/PATH/golfsiden.env -f compose.production.yml -f compose.release.json --profile tools run --rm --no-deps --pull never permissions
docker compose --env-file /PRIVATE/PATH/golfsiden.env -f compose.production.yml -f compose.release.json up -d --wait --no-build --pull never api web
```

Never run the development seed in production. Confirm `/api/health` and
`/api/ready` return 200 through trusted public HTTPS, and HTTP on port80 redirects
to the working HTTPS endpoint. On the actual playing phone, use distinct
organizer/player accounts to accept an invitation, claim a player, open the
prepared round, save/reload a score and observe live updates. Exercise the
prepared offline-edit/conflict flow before relying on it during play. Verify
final-round privacy and Fantasy if enabled. Acknowledged Fantasy actions whose
view failed to refresh use **Oppdater Fantasy** for a read retry.

## Existing installation and rollback

Keep the existing project identity, runtime secrets and named volumes. Export
`COMPOSE_PROJECT_NAME` to that existing identity before any command, and pass
the same value to the verifier's `--project` option. Resolve
device edits and wait for server acknowledgement before maintenance; a server
dump cannot recover unsent device edits. Keep the running image generation and
private env file. Create and verify a current dump/checksum pair and copy it
off-host before migration. The bundled scripts deliberately use only
`compose.production.yml` and inherited `COMPOSE_PROJECT_NAME`; the common loaded
release tag must select the same images without the override. `--check-env`
checks that script-only binding. Do not use `-p`, which the scripts do not inherit,
or switch projects halfway through an upgrade.

Load the frozen images before downtime. With the existing private env file,
stop `api web`, start/wait for `postgres` using the frozen files, run owner
`migrate`, run `permissions`, then start matching `api web` with the commands
above. Apply every intermediate migration; startup verifies the entire embedded
history and checksums. PostgreSQL17.11 remains major17; no data-directory format
upgrade is selected. Do not point this at another major version's volume.

Run `scripts/backup-production.sh` and `scripts/restore-production.sh` using
their existing usage and environment arguments. Rehearse the real off-host
dump/checksum pair in a separate shell: export a fresh `COMPOSE_PROJECT_NAME`
such as `golfsiden-restore-check`, use a separate private env file with distinct
credentials and unused web ports, then run `--check-env` with `--project` set
to that fresh identity. Start only target `postgres` before restore; do not
migrate or initialize application tables first. Restore into this empty target,
then verify logins and score/Fantasy facts. Preserve the original dump and its
sidecar. Restore needs explicit confirmation and rejects populated targets.

If schema rollback is needed, restore a verified pre-upgrade dump into a
**fresh volume** and use its matching application images. There are no SQL down
migrations. Preserve the current volume and any acknowledged writes since the
backup while deciding recovery; do not delete them or silently overwrite them.
The repository deployment guide contains the full backup/restore, upgrade,
account recovery and monitoring procedures. Stop launch if required host,
phone, backup/restore or health checks fail.

The patched Caddy module is v2.11.7 with Go1.27.2, x/net0.60.0 and quic-go0.63.0.
Its custom build prints `unknown` for `caddy version`; use `caddy build-info`
and the manifest image ID for identity. Residual package advisory dispositions
are conditional on the supplied configuration; the release is not scanner-clean
and does not establish public-host security or production capacity.
