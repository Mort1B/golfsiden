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

Keep `vendor/sqlx-postgres` in the build context. Cargo and the production
Dockerfile use this pinned SQLx 0.8.6 transaction-cancellation backport. Deploying
the repaired API replaces its pool and connections; there is no schema migration
for this change. Rolling back to an older binary also restores the known
transaction-initialization defect. Remove the override only after adopting an
upstream release with the fix and passing the retained cancellation regressions;
see `vendor/sqlx-postgres/PATCH.md`.

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

- set `RESET_PASSWORD_ORIGIN` to the exact public HTTPS origin, for example
  `https://golf.example.com`, without a trailing slash, path, query or fragment;
  use the same trusted hostname as `SITE_ADDRESS`; recovery links never use a
  request's Host header;
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
production API image contains `golf-api`, `migrate`, and the operator-only
`password-recovery` command; it excludes `seed`.

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

### Schema 19 completion preflight

Migration 0019 requires every already completed/archived tournament to have its
entire configured round plan locked. It preserves valid historical rows and does
not fabricate completion actors. New completions receive workflow-managed audit
records. Before upgrading from schema 18, check for incompatible history using
owner/migration access (this query does not modify data):

```sql
SELECT t.id, t.status
FROM tournaments t
WHERE t.status IN ('completed', 'archived')
  AND (
    (SELECT count(*) FROM rounds r WHERE r.tournament_id = t.id) <> t.number_of_rounds
    OR EXISTS (
      SELECT 1 FROM rounds r WHERE r.tournament_id = t.id
        AND (r.status <> 'locked' OR r.round_number NOT BETWEEN 1 AND t.number_of_rounds)
    )
  );
```

If rows are returned, stop and investigate the historical state before upgrade.
The migration fails atomically with `tournament_completion_legacy_not_ready`;
do not bypass triggers, auto-lock scores or rewrite history to force deployment.
Any remediation requires a separately reviewed operator decision and backup.
Run the normal post-migration permissions action for the new
`tournament_completions` table. Keep `RUN_MIGRATIONS=false`; the API readiness gate
requires the compiled schema. A schema-18 binary cannot serve a schema-19 database.

### Schema 20 archive upgrade

Migration 0020 adds the guarded completed-to-archived workflow and append-only
`tournament_archives` actor/time records. Existing completed/archived rows and
completion evidence are unchanged; historical archive actors are not invented.
The migration replaces schema 19's temporary archive rejection without relaxing
its completion, closed joining or historical round protections. Archive is not
deletion: it retains member access and independent final visibility.

Apply the forward migration with owner authority, then run the normal permissions
action so the runtime role receives access to the new audit table and functions.
Keep `RUN_MIGRATIONS=false`. Schema-19 binaries cannot serve schema 20; rollback
requires the normal pre-upgrade backup/fresh-volume recovery, not SQL downgrades.
For upgrades from schema 18, the schema 19 preflight above still applies.

### Schema 21 supplied-course presets

Migration 0021 adds three finalized, user-supplied men's Red Tees layouts and the
`course_presets` registry. It does not update existing rounds, scores, course
revisions or handicap snapshots. The preset picker becomes available after this
migration and the matching API/frontend are deployed; no development seed is
required or appropriate on a retained database.

Apply with owner authority and run the normal permissions action for the new
registry table. Keep `RUN_MIGRATIONS=false`; schema-20 binaries cannot serve
schema 21. Use the normal pre-upgrade backup and fresh-volume recovery for
rollback, never delete finalized presets or edit previously applied migrations.
The data was validated on disposable PostgreSQL 17, not deployed to production
by the implementation task.

### Schema 22 authenticated tournament creation

Schema 22 adds `tournament_creation_requests` for authenticated-creation retry
receipts. Existing accounts, tournaments and historical facts are unchanged.
Back up first, migrate with owner authority, refresh runtime permissions for the
new table, and deploy matching API/frontend binaries. Keep `RUN_MIGRATIONS=false`.
Schema-21 binaries cannot serve schema 22; use the normal backup/fresh-volume
recovery if rollback is required, never a SQL downgrade or development seed on
retained data. This implementation was validated only on disposable PostgreSQL;
production deployment is a separate operator action.

### Schema 23 self-service profile

Migration 0023 adds account profile versions and account/session credential
generations, plus narrow update and tournament authorization guards. Existing
valid sessions start with matching generations and remain valid; existing
expired/revoked sessions stay invalid. The next real password change invalidates
all of that user's sessions. No tournament, score, handicap snapshot, ownership
or membership data is rewritten. Profile handicap changes are a separate runtime
self-service action, not a migration or seed operation.

Back up first, apply the forward migration with owner authority, refresh runtime
permissions using the normal action, and deploy matching API/frontend binaries.
Keep `RUN_MIGRATIONS=false`. Schema-22 binaries cannot serve schema 23; rollback
requires the pre-upgrade backup/fresh-volume recovery, not dropping generation
columns or downgrading SQL. Verify readiness, login, profile loading and a
representative existing tournament read. Validate password change/sign-out with
a designated test account, never by changing an operator's real password as an
implicit deployment check. This implementation was validated on disposable
PostgreSQL only and does not itself deploy or migrate production.

### Schema 24 password recovery

Migration 0024 adds recovery grants, append-only outcomes and authority history.
It preserves existing accounts, passwords, valid sessions and all tournament
facts. Recovery becomes available through the matching API/frontend; existing
administrator accounts require the site operator's packaged CLI below.

Before using any commands with the updated Compose file, add
`RESET_PASSWORD_ORIGIN=https://YOUR_HOST` to the private runtime file: Compose
requires it even when starting only a tools service. Back up, migrate with owner
authority, refresh runtime permissions and deploy matching API/frontend. Keep
`RUN_MIGRATIONS=false`. The runtime role must neither own the recovery tables nor
inherit their owner role; startup rejects that authority. Broad runtime DML grants
do not permit operator provenance or rewriting recovery/audit history.

Recreate PostgreSQL during this upgrade to apply the Compose logging settings:
`log_parameter_max_length=0`, `log_parameter_max_length_on_error=0` and
`log_error_verbosity=terse`. These retain slow-query timings and basic errors
without bind values or row details that could reveal password/token hashes. This
reduces diagnostic detail deliberately; do not re-enable detailed logging around
real credentials. A plain container restart does not adopt changed Compose
command arguments; the normal `up -d postgres` action recreates when needed.

Schema-23 binaries cannot serve schema 24. Use the pre-upgrade backup and
fresh-volume restoration for rollback. Validate `/api/ready`, existing login and
a representative tournament read. Exercise reset only with a designated test
account, never an operator's real password as an implicit deployment check.
This implementation does not deploy or recover production accounts itself.

### Schema 25 tournament tie-breaks

Migration 0025 adds the required typed tournament tie policy and extends the
existing permanent pre-start configuration guard. Existing tournaments, including
active and finished events, retain shared places through the `shared_positions`
default. New tournaments also default to shared places; organizers may select
`final_round_score` in draft settings. Scores, handicap snapshots, creation
receipts and serialized retry fingerprints remain unchanged.

Back up, migrate with owner authority, refresh runtime permissions and deploy
matching API/frontend builds. Keep `RUN_MIGRATIONS=false`. Schema-24 binaries
cannot serve schema 25; rollback uses the pre-upgrade backup and fresh-volume
restore. The updated frontend requires the new response metadata, and older
clients can reject standings whose shared positions have been resolved. Reload
open clients onto the matching frontend before enabling the optional policy.
Verify readiness, an existing tournament read and representative gross/net
standings. Schema upgrades and historical preservation were tested against a
disposable PostgreSQL database; this change does not migrate production itself.

### Schema 26 public live result-sharing

Migration 0026 adds tournament result capabilities and derived immutable audits.
Existing tournament, score, snapshot and account history is unchanged; neither
upgrade nor development seed creates a link. Links appear only after an exact
admin deliberately issues one through the matching frontend/API. Tokens are
independent of sessions, invitations and password recovery and persist only as
hashes. There is no new origin or email configuration: the frontend builds the
same-origin reusable fragment link.

Back up first, migrate with owner authority, refresh runtime permissions and deploy
matching API/frontend images, including the updated Caddyfile. Keep
`RUN_MIGRATIONS=false`; schema-25 binaries cannot serve schema 26. Rollback uses
the pre-upgrade backup and fresh-volume restoration. Retain existing PostgreSQL
parameter/row-detail logging restrictions, and do not add capability bodies or
browser fragments to logs, telemetry or cache keys. The updated proxy marks shared
HTML and sharing API responses no-store, no-referrer and noindex/nofollow.

Verify readiness and existing private tournament reads after rollout. Using a
designated test tournament, deliberately issue a link, check anonymous limited
standings and final visibility, then revoke it and verify unavailable on the next
read. Never create or distribute a real tournament link as an implicit deployment
check. The page refreshes every 15 seconds while visible; revocation prevents new
reads but cannot recall results already delivered. This iteration used disposable
local services and does not itself deploy or expose production results.

### Upgrade sequence

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
(cd /secure-staging && sha256sum --check golfsiden-YYYYMMDD-HHMM.dump.sha256)
```

Run checksum verification from the dump directory because the sidecar records
only the dump basename. The subshell keeps your original working directory.

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

## Administrator and unlinked account recovery

Use this flow for administrator accounts, a sole organizer, unlinked accounts or
players without an eligible tournament organizer. First verify the person's
identity using a known channel and establish the exact account UUID. Use owner
access for a narrowly targeted lookup, for example this psql query with a supplied
`account_username` variable, and verify the returned identity before proceeding:

```sql
SELECT id, username, player_id FROM users WHERE username = :'account_username';
```

The CLI does not choose or accept a new password and does not assign web roles.
It requires the actual migration/table-owner database connection (or database
superuser), an exact account UUID and a 1–500-character audit reason. Runtime API
credentials are intentionally insufficient. Reasons should describe the
verification/action without contact details, passwords or tokens.

The production API image contains the command. Reuse the tools service's owner
connection and private network, overriding its entrypoint. Replace `ACCOUNT_UUID`
and `https://YOUR_HOST` below; the origin must equal the configured public origin.
The private output directory is temporary and readable only by the current host
operator. Running with that operator's UID/GID lets the container create a private
file that the operator can read without making it public:

```bash
recovery_output_dir="$(mktemp -d "${TMPDIR:-/tmp}/golfsiden-recovery.XXXXXX")"
chmod 700 "$recovery_output_dir"
docker compose --env-file .env.production -f compose.production.yml --profile tools run --rm \
  --user "$(id -u):$(id -g)" \
  --entrypoint /usr/local/bin/password-recovery \
  --env RESET_PASSWORD_ORIGIN=https://YOUR_HOST \
  --volume "${recovery_output_dir}:/recovery-output:Z" \
  migrate issue ACCOUNT_UUID "Identity verified through known contact channel" /recovery-output/link.txt
```

The command creates a new mode-0600 file and never overwrites an existing file.
It prints only success/failure guidance, never the link, to command output. Open
`$recovery_output_dir/link.txt` privately and share its contents only with the
verified account holder through your existing contact channel. Do not paste it
into a shared terminal transcript or issue. Remove the local file and empty
directory after delivery:

```bash
rm -- "$recovery_output_dir/link.txt"
rmdir -- "$recovery_output_dir"
```

The link lasts 30 minutes, previews without consumption and uses the normal public
reset page. Issuing another link replaces earlier grants for the account. If a
private-output write fails after issuance, the command revokes that exact new
grant; a reported revocation failure requires the explicit revoke command below.
An empty output file from an earlier failure must be removed or replaced with a
new path before retrying. Revocation needs no saved link:

```bash
docker compose --env-file .env.production -f compose.production.yml --profile tools run --rm \
  --entrypoint /usr/local/bin/password-recovery \
  migrate revoke ACCOUNT_UUID "Recovery cancelled after contact verification"
```

Operator grants record explicit operator provenance and database actor in retained
issue/replace/revoke/redeem audit events. Grant identity and terminal outcomes are
immutable. Redemption invalidates all target sessions through credential
generation without touching unrelated sessions, tournament membership, historical
scores or handicap snapshots. Never edit password hashes, disable guards or
invent a web administrator account as a recovery shortcut.

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

### Schema 27 durable conditional score delivery

Migration 0027 adds score revisions and immutable delivery receipts. It initializes
existing scores at revision 1 without changing their strokes, timestamps, audits,
confirmation or handicap snapshots. Apply the migration and refresh runtime grants
before starting matching API/frontend builds. The new scoring client requires the
conditional endpoint and required revision strings on authorized scoring responses;
member and public result projections are unchanged. Older API binaries fail exact
schema readiness after this migration. Rollback uses the documented backup/restore
procedure with matching binaries; do not edit or remove an applied migration.

Receipts acknowledge past writes and must not be pruned while their parent account,
round and score exist: a later retry must remain deduplicated. No receipt cleanup
job is introduced. Ordinary score changes, including the legacy endpoint and
explicit database correction path, advance revisions on actual stroke changes.

Unsent edits reside in the user's browser IndexedDB, isolated by account. Server
backups do not contain those pending edits. Logging out pauses delivery but keeps
that account's device copy; clearing browser site data removes it. Queue delivery
requires the app's private workspace to be running with an authorized session.
Cold offline launch and background sync are outside this release. Confirmation is
online-only; locked rounds and revoked access cannot be bypassed through replay.

### Schema 28 four-ball scoring

Migration 0028 adds the four-ball format and dedicated player inputs, audits and
delivery receipts. Back up the database, apply the forward migration with the
owner connection, refresh runtime grants, then start matching API and web builds.
Existing numeric scores, revisions, audits, confirmations, snapshots and immutable
offline request bodies are preserved. Older API binaries fail exact schema
readiness after migration. Rollback requires the documented backup/restore process
and matching binaries; never edit an applied migration.

Four-ball numeric/no-score inputs retain their identity and revision history.
Delivery receipts must remain while their parent account, round and input exist;
do not add a receipt-pruning job. Intentional parent deletion retains the defined
cascade behavior. The legacy numeric queue and new four-ball queue protocol share
the browser's account-isolated storage. Server backups do not include unsent
device edits. Confirmation requires an online authorized session, and locked
rounds reject ordinary four-ball corrections.

### Schema 29 individual Stableford

Migration 0029 adds `individual_stableford`, dedicated player inputs, audits and
immutable receipts, plus state-aware confirmation/completion guards. Apply it with
the owner connection after a verified backup, refresh runtime grants and deploy
matching API and web builds together. The API still requires exact schema
compatibility; do not start older binaries against schema 29 or edit a published
migration. Rollback uses the documented backup/restore procedure and matching
binaries, not an in-place downgrade of retained input states.

The populated schema-28 upgrade was tested with both legacy numeric and four-ball
scores, snapshots, confirmations, audits and receipts. Rows and accepted-request
replay remain unchanged. Fresh migration and repeat seeding also passed on
PostgreSQL 17.11. New Stableford inputs retain UUID identity and positive revisions
through numeric/pickup corrections. Keep receipts for the lifetime of their
parents; pruning them could turn a delayed retry into a new operation.

The browser adds `stableford_v1` in the existing account-isolated queue without
rewriting legacy or four-ball request heads. Server backups do not contain unsent
device edits. Confirmation remains online-only, and ordinary replay cannot bypass
a locked round or revoked authority. Stableford settings are editable only before
round opening; historical calculations use frozen snapshots.

Stableford and mixed result responses use tagged version-1 point/equivalent
values rather than reinterpreting actual-stroke fields. Deploy the corresponding
strict decoders and UI at the same time. Existing stroke-only result and delivery
contracts retain their meaning. Public sharing keeps its existing overall-only
scope, with a non-private converted-value label; it does not expose player cards.

### Schemas 30–32 singles match play

Migrations 0030–0031 add the singles format, round-local matches/opponents,
player-owned numeric notes, immutable command receipts, append-only audit and
ledger/confirmation integrity guards. Migration 0032 makes `counted_rounds`
explicitly nullable for match-only plans and adds deferred eligibility validation
with an internal `tournament_overall_configuration_guards` generation row. This
row serializes cross-table changes under snapshot isolation without advancing
public configuration timestamps.

Take and verify a backup, apply all forward migrations with the owner connection,
refresh runtime grants using the documented permissions command, and deploy matching
API and web builds together. New tables/functions require the normal runtime DML
and execution grants. Exact schema readiness rejects old binaries against schema
32. Do not edit applied migrations or attempt an in-place downgrade. Rollback uses
a pre-upgrade backup restored into a fresh volume and its matching binaries.

Schema 0032 deterministically normalizes only plans containing the new match format
that could exist at schema 30/31: all-match N/mandatory becomes null, mixed N is
capped to eligible rounds, and a mandatory match is cleared. Existing non-match
configuration and score/snapshot/audit/receipt rows are preserved. The old
configuration guard is disabled only inside the migration's transaction for this
normalization and re-enabled before completion. Populated schema-29 preservation,
schema-31 normalization, fresh migration through all 32 versions and seeding twice
passed on disposable PostgreSQL 17.11. This step did not run a Docker deployment
or a new production restore exercise.

Retain match receipts for the lifetime of their parent records; a delayed original
request must not become a new write. Corrections retain superseded audit facts and
clear confirmation/points atomically. Locked corrections require the dedicated
exact-admin command path; operator SQL must not bypass lifecycle integrity.

The frontend adds the separate `golf-match-notes-v1` IndexedDB database and
`match_notes_v1` protocol without rewriting the existing `golf-pending-scores-v1`
queues. Server backups omit unsent browser drafts. Clearing site data removes
those copies; unknown delivery must be reconciled using its original identity.
Only numeric notes can wait offline. Accepted reports, concessions, awards,
corrections and confirmation require connectivity and current authorization.
Deploy strict nullable overall and match decoders with the API: match-only private
overall reads are explicitly not applicable and public overall sharing unavailable.
Mixed public summaries keep their existing allowlist and exclude match facts.

## Current friends-deployment validation

The [2026-09-23 assessment](validation/friends-2026-09-23/README.md) covers
application commit `38e3eef` using production frontend assets, local Caddy,
disposable PostgreSQL and a restricted runtime role. Its separate HTTPS smoke
exercised production API mode, secure cookies and native score events through
the proxy. It used an internal local certificate and did not assemble the full
production Compose deployment or validate public DNS/ACME.

The subsequent [disposable recovery rehearsal](validation/recovery-2026-09-23/README.md)
built all current production images, restored 47 tables with exact data parity,
and passed Chrome checks on both stacks. It used Docker Compose on rootless
Podman and local TLS; Docker Engine and public DNS/ACME acceptance remain untested.
The report's OPS-1 documentation finding is resolved: the Backup example now
verifies from the dump directory. The restore script already handled this correctly.

The [local authentication assessment](validation/authentication-2026-09-23/README.md)
is complete for authentication, sessions, CSRF and tournament access. It confirmed
two P2 issues: active throttle eviction and session expiry during a
handicap-correction wait. The subsequent AUTH-1 repair preserves live throttle
counters under capacity pressure. The subsequent
[AUTH-2 repair](validation/handicap-session-expiry-2026-09-23/README.md) rechecks
session validity immediately before handicap-correction commit after database
waits; detected expiry rolls back the handicap/audit and emits no event. Shared
limiter saturation temporarily rejects new client/resource buckets until space
expires, using 429 and a retry hint. Existing keys retain their remaining quota.
These repairs do not complete the wider security assessment.

The subsequent [local recovery security assessment](validation/recovery-security-2026-09-23/README.md)
found no confirmed new defects. Existing local PostgreSQL tests and installed
Chrome recovery flows passed, including target-session invalidation and unrelated
session preservation. The report records remaining operator-output failure,
configured-runtime CLI and late-write expiry coverage gaps. Its loopback HTTP
checks do not establish public TLS, deployment-role or physical-device acceptance.

The [local result-projection assessment](validation/result-projection-security-2026-09-23/README.md)
confirmed SHARE-1 (P2): public-link issuance can complete after session expiry
during a late audit-table wait. The reproducer required a maintenance-style lock;
no anonymous mechanism for inducing it was shown. The subsequent
[SHARE-1 repair](validation/result-share-session-expiry-2026-09-23/README.md)
rechecks session validity after writes and immediately before commit. Expired
issue/replacement/revoke requests roll back with 401 and no invalidation event;
failed replacement/revoke preserves the old grant unchanged. No migration or operator
configuration change is required.

The [browser/offline persistence assessment](validation/browser-persistence-2026-09-23/README.md)
confirmed PERSIST-1 (transient match input loss), PERSIST-2 (uncontrolled retry on
storage failure) and PERSIST-3 (late mutation recreating cleared memory cache).
The [match-note retention repair](validation/match-note-retention-2026-09-23/README.md)
resolves PERSIST-1 without migration or operator configuration changes. Unsaved
input remains memory-only until a device write commits. The
[score retry repair](validation/score-storage-retry-2026-09-23/README.md) resolves
PERSIST-2 with no operator/configuration change: failed storage ends the immediate
drain, preserves pending operations, and waits for polling or an explicit wake.
An existing lease still governs retry timing. The
[Stableford callback repair](validation/stableford-settings-lifetime-2026-09-23/README.md)
resolves the confirmed PERSIST-3 path without configuration changes. Late responses
from that departed editor are ignored; a fresh authorized read recovers server
settings. Similar pairing, tournament-start and visibility callbacks remain
source-supported concerns requiring separate reproduction. These findings do not
establish a server authorization bypass or another account's UI disclosure.
Frontend repair validation used real Chrome with synthetic API responses. The
PERSIST-1 PostgreSQL-backed repeat remains blocked by its documented Docker socket
permissions.

Deployment sign-off is **NOT READY**: related callback concerns, remaining operational assessment,
public-host acceptance, native 200% browser zoom and physical Android Chrome remain
unresolved.
The report's three frontend
accessibility defects were repaired in the subsequent
[navigation accessibility step](validation/navigation-accessibility-2026-09-23/README.md).
Complete the outstanding deployment/browser gates before treating
the local browser results as a production-readiness verdict.
