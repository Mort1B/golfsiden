# Production deployment and recovery

This is the supported portable production baseline: one Linux host running
Docker Compose, Caddy, the built Vite site, the Rust release API, and a private
PostgreSQL 17 container. The browser sees one HTTPS origin. Caddy serves the
frontend and proxies `/api`, including long-lived SSE responses, to the API.
Only ports 80 and 443 are public; PostgreSQL is attached only to the internal
`data` network and has no host port.

This baseline is intentionally single-host. Backups must leave the host if they
are expected to survive loss of that host.

## Host and DNS prerequisites

- A current Linux host with Docker Engine and Docker Compose v2.
- A public DNS A/AAAA record for the host and inbound TCP 80/443. UDP 443 is
  optional but enables HTTP/3. Do not expose PostgreSQL port 5432.
- At least enough free space for two image generations, the PostgreSQL volume,
  Caddy data, and local backup staging.
- A deployment checkout of this repository. Rust and Node are not required on
  the host because the images build them.

Caddy obtains and renews public certificates automatically. `SITE_ADDRESS` must
be a hostname only: no scheme, path, query, fragment, or port. The web container
refuses invalid values. `localhost` is suitable only for a local acceptance run.

## Production configuration

Create an ignored runtime file and restrict its permissions:

```bash
cp .env.production.example .env.production
chmod 600 .env.production
```

Set every placeholder in that file. In particular:

- use distinct, randomly generated `POSTGRES_OWNER_PASSWORD` and
  `APP_DATABASE_PASSWORD` values of at least 24 characters; the PostgreSQL
  container rejects the documented placeholders, equal values, and `golf/golf`;
- keep `POSTGRES_OWNER_USER` and `APP_DATABASE_USER` different;
- keep both database URLs consistent with those values;
- generate `PROXY_SHARED_SECRET` as exactly 32 random bytes encoded as 43
  unpadded base64url characters, for example:

```bash
openssl rand -base64 32 | tr '+/' '-_' | tr -d '='
```

- choose an immutable `GOLFSIDEN_IMAGE_TAG`, normally the release Git SHA;
- leave `GOLF_COURSE_API_KEY` empty unless provider detail is required; the key
  is backend-only and must never be put in Vite variables or browser code.

The API always runs with `APP_ENV=production`, `RUN_MIGRATIONS=false`, and
`SESSION_COOKIE_SECURE=true` in this Compose model. Production validation rejects
insecure cookies, startup migrations, a missing or malformed proxy secret,
non-HTTPS CORS origins, invalid pool sizes, and port zero. The same-origin
baseline does not set `CORS_ALLOWED_ORIGIN`.

## Initial deployment

All migration and seed actions below target PostgreSQL inside the Compose
network; no database port needs to be opened.

```bash
docker compose --env-file .env.production -f compose.production.yml --profile tools build
docker compose --env-file .env.production -f compose.production.yml up -d postgres
docker compose --env-file .env.production -f compose.production.yml --profile tools run --rm migrate
docker compose --env-file .env.production -f compose.production.yml --profile tools run --rm permissions
docker compose --env-file .env.production -f compose.production.yml up -d api web
docker compose --env-file .env.production -f compose.production.yml ps
curl --fail https://YOUR_HOST/api/ready
```

Run `migrate` a second time if an idempotence check is wanted; it must report the
schema current. Never run the development `seed` binary in production. The
production API image deliberately contains only `golf-api` and `migrate`.

The PostgreSQL initialization image creates the runtime login only on a new
volume. The owner performs migrations. The `permissions` action must run after
every migration so the runtime role receives DML/sequence/function access to new
objects. It cannot create or alter schema objects and must not be a superuser.
The permission action also revokes writes to `_sqlx_migrations`; the runtime
role may read migration history only. Production API startup verifies that the
connected PostgreSQL identity exactly matches `APP_DATABASE_USER`, has no
superuser, database-creation, role-creation, or public-schema creation authority,
and cannot modify migration history. An owner URL therefore fails closed even
if it otherwise points at a compatible schema.

The API refuses to bind unless the applied `_sqlx_migrations` history exactly
matches every embedded migration, including version, success state, and
checksum. Pending, unknown, dirty, missing, or changed migration history is a
visible startup failure.

## Upgrade and rollback

Before every upgrade:

1. create and copy off-host a verified backup;
2. record the running Git SHA and image tag;
3. build the new immutable tag;
4. start PostgreSQL, run the explicit migration action, then permissions;
5. recreate API and web and verify `/api/ready` plus a login and tournament read.

Application-only rollback is a checkout/image-tag rollback followed by
recreating `api` and `web`. SQL migrations are forward-only. Do not improvise a
schema downgrade. If an older application cannot use the migrated schema,
declare downtime and restore the pre-upgrade dump into a fresh PostgreSQL volume
using the recovery procedure below.

## Health, logs, and routine operation

- `GET /api/health` is process liveness and deliberately does not query the
  database.
- `GET /api/ready` checks database reachability and exact schema compatibility;
  it returns stable `503 service_unavailable` when the API must not receive
  traffic.
- Compose healthchecks gate web startup on API readiness, and services restart
  unless stopped deliberately.
- Inspect status and logs with:

```bash
docker compose --env-file .env.production -f compose.production.yml ps
docker compose --env-file .env.production -f compose.production.yml logs --since 30m api web postgres
```

Alert on repeated restarts, readiness failures, 5xx responses, sustained 429s,
low disk space, failed backups, and PostgreSQL connection exhaustion. Logs must
not contain environment files, database URLs, cookies, CSRF tokens, provider
keys, or score mutation bodies. Caddy access logs redact cookies.

An unavailable optional course provider must not prevent startup or manual
course configuration. Investigate provider errors separately from core scoring.

## Backup

The supported backup is a PostgreSQL custom-format logical dump with ownership
and grants removed. The script writes atomically and creates a SHA-256 sidecar
whose entry uses only the dump basename, so the pair can be moved together:

```bash
scripts/backup-production.sh .env.production /secure-staging/golfsiden-YYYYMMDD-HHMM.dump
sha256sum --check /secure-staging/golfsiden-YYYYMMDD-HHMM.dump.sha256
```

Copy both files to encrypted off-host storage, then test restores on a schedule.
Choose retention based on the tournament calendar; at minimum keep multiple
daily generations during an active tournament and a pre-deployment generation.
A dump left only on the application host is not disaster recovery.

## Restore exercise and disaster recovery

Restore only to a new, empty database volume. The restore script deliberately
refuses a public schema containing any relation, routine, or standalone type and
requires an explicit confirmation phrase. It verifies the checksum when the
sidecar is present, restores in one transaction with `--exit-on-error`, and
reapplies runtime grants.

For a replacement host with fresh Docker storage:

```bash
docker compose --env-file .env.production -f compose.production.yml up -d postgres
CONFIRM_EMPTY_RESTORE=RESTORE_TO_EMPTY_DATABASE \
  scripts/restore-production.sh .env.production /secure-staging/golfsiden-YYYYMMDD-HHMM.dump
docker compose --env-file .env.production -f compose.production.yml up -d api web
curl --fail https://YOUR_HOST/api/ready
```

For a same-host recovery exercise, copy the runtime file, choose unused public
ports, and prefix every Compose and script command with a separate
`COMPOSE_PROJECT_NAME`, such as `golfsiden-restore-test`. That creates independent
networks and volumes. Do not use `down --volumes` on a real deployment unless
the exact project and volumes have been identified and a verified off-host
backup exists.

After restore, verify at least:

- user, tournament, player, round, and score counts;
- administrator login and one ordinary member login;
- tournament status, course configuration, pairings, handicap snapshots, and
  gross/net leaderboards;
- a representative scorecard and final-round hidden/released state;
- one safe score mutation only if the recovery target is authorized for use.

Loss or replacement of API/web containers does not affect tournament data. Loss
of the PostgreSQL volume requires this off-host restore procedure.

## Secret and credential changes

- Rotate `PROXY_SHARED_SECRET` by updating the runtime file and recreating API
  and web together; a mismatch intentionally makes forwarded client identity
  untrusted.
- Rotate the runtime database password in PostgreSQL and the runtime file in one
  maintenance window, then recreate API.
- Treat owner credential rotation as a migration/backup maintenance operation.
- Removing or rotating a course-provider key affects only provider detail.
- Compromise of session state requires revoking affected rows in
  `user_sessions`; ordinary logout revokes one session immediately.

Keep `.env.production`, dumps, and checksums out of Git. The repository ignores
`.env.*` except the documented example.
