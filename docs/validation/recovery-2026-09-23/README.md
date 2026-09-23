# Disposable production deployment and recovery rehearsal

Date: 2026-09-23. Application source: `738ec1d34919223ea2e2e71f98607d8e494f922b`.
No application, migration, dependency, Compose or operator-script changes.

## Verdict

**Recovery rehearsal: READY WITH KNOWN LIMITATIONS.** The current production
images built, initialized a new database, served the application, and recovered
all stored data into a separate fresh target using the repository scripts.
**Overall friends deployment: NOT READY.** The separate security assessment,
public-host DNS/ACME acceptance, native 200% zoom and physical Android Chrome
checks remain outstanding. A confirmed P2 operator-documentation defect is
queued separately; this assessment does not silently repair it.

## Isolation and versions

Docker Compose v5.5.1 used a task-owned rootless Podman 5.8.4 API socket. The host's
Docker Engine socket was inaccessible; this is a Compose-on-Podman rehearsal,
not proof of deployment on a Docker Engine host. No host service permissions
were changed. Both environments used the unmodified `compose.production.yml`:

| Project | HTTP / HTTPS, loopback only | Database |
| --- | --- | --- |
| `golf-rehearsal-source-738ec1d` | 18581 / 18582 | Fresh named volume |
| `golf-rehearsal-restore-738ec1d` | 18591 / 18592 | Separate fresh named volume |

`PUBLIC_HTTP_PORT` and `PUBLIC_HTTPS_PORT` values included `127.0.0.1:`.
API and PostgreSQL ports were not published. `SITE_ADDRESS=localhost` used
Caddy's local certificate; diagnostic HTTPS clients explicitly accepted that
certificate. Public DNS, certificate trust/renewal and external HTTP redirects
were not tested. Each environment had independent random owner/runtime/proxy
secrets in mode-600 files inside a mode-700 task directory. Separate browser
contexts avoided sharing cookies between localhost ports.

All three images were built under `rehearsal-738ec1d`, without replacing shared
`:local` tags. PostgreSQL was 17.11, Caddy 2.10.2, and Chrome 153.0.8010.36.

| Image | Image ID |
| --- | --- |
| API | `a704f54b11b707b28ccb2e6ec7ced1b47dc241b4a402c4319e773142f9904f27` |
| Web | `19ca265955b9e87a8d8462d2e95fe2c335a26f66cccecd5ad1f0247d700e45e6` |
| PostgreSQL | `4e9677a830cdd5b25fe455484589e8d5312d3ac5bdc997b6d921ac04e49afa7f` |

The Rust release build emitted an existing vendored SQLx `try_next` deprecation
warning; it succeeded. The image includes `migrate` and `password-recovery`, and
excludes `seed`. No development seed was run.

## Execution and evidence

These are diagnostic commands for the isolated projects, not production deploy
instructions. The private task directory was `/tmp/golf-rehearsal-738ec1d`.
Environment files also contain their distinct `COMPOSE_PROJECT_NAME`; every
Compose/script invocation explicitly selected the project. Commands ran from
the repository root unless a frontend working directory is stated.

```sh
export DOCKER_HOST=unix:///tmp/golf-rehearsal-738ec1d/podman.sock
export COMPOSE_PROJECT_NAME=golf-rehearsal-source-738ec1d
docker compose --env-file /tmp/golf-rehearsal-738ec1d/source.env -f compose.production.yml --profile tools build
docker compose --env-file /tmp/golf-rehearsal-738ec1d/source.env -f compose.production.yml up -d postgres
docker compose --env-file /tmp/golf-rehearsal-738ec1d/source.env -f compose.production.yml --profile tools run --rm migrate
# Repeated migrate successfully, then:
docker compose --env-file /tmp/golf-rehearsal-738ec1d/source.env -f compose.production.yml --profile tools run --rm permissions
docker compose --env-file /tmp/golf-rehearsal-738ec1d/source.env -f compose.production.yml up -d api web
node docs/validation/recovery-2026-09-23/fixture.cjs setup
```

The actual initial fixture run expected a nonmember 404; the API returned its
correct existing 403. The diagnostic was corrected and its `capture` mode
logged in to the already-created accounts to capture the baseline. An initial
Chrome command named a nonexistent `playwright.config.ts`; the corrected
lifecycle configuration below passed. Neither was an application failure.

The existing `deploymentBoundary.browser.ts` ran once on the source and once on
the restored target, with **1/1 passed on each** (17.3 seconds each):

```sh
# Working directory: frontend. Repeat with port 18592 after restore.
GOLF_DEPLOYMENT_BROWSER=1 GOLF_DEPLOYMENT_LOCAL_CERT=1 \
GOLF_DEPLOYMENT_ORIGIN=https://localhost:18582 \
npx playwright test --config playwright.lifecycle.config.ts \
  e2e/deploymentBoundary.browser.ts --reporter=line
```

This exercised secure HttpOnly/SameSite cookies, Caddy security headers, UI
login, score persistence, native EventSource score invalidation, same-origin
traffic and reload. It added its own synthetic tournament. After the source
browser run ended, the database was quiescent for fingerprinting and backup:

```sh
python3 docs/validation/recovery-2026-09-23/snapshot.py source
COMPOSE_PROJECT_NAME=golf-rehearsal-source-738ec1d \
  scripts/backup-production.sh /tmp/golf-rehearsal-738ec1d/source.env \
  /tmp/golf-rehearsal-738ec1d/source.dump
COMPOSE_PROJECT_NAME=golf-rehearsal-restore-738ec1d \
  docker compose --env-file /tmp/golf-rehearsal-738ec1d/restore.env \
  -f compose.production.yml up -d postgres
python3 docs/validation/recovery-2026-09-23/guards.py
python3 docs/validation/recovery-2026-09-23/snapshot.py restore
COMPOSE_PROJECT_NAME=golf-rehearsal-restore-738ec1d \
  docker compose --env-file /tmp/golf-rehearsal-738ec1d/restore.env \
  -f compose.production.yml up -d api web
node docs/validation/recovery-2026-09-23/fixture.cjs verify
python3 docs/validation/recovery-2026-09-23/role-and-rollback.py
node docs/validation/recovery-2026-09-23/browser-restored.cjs
```

`guards.py` invokes the unmodified restore script with
`CONFIRM_EMPTY_RESTORE=RESTORE_TO_EMPTY_DATABASE` and the explicit restore project.
The restore target was **not migrated before restoration**. That would correctly
make it nonempty. Diagnostics are retained here outside the normal test suite;
they use the fixed disposable names above and require freshly prepared local
fixtures. They are not general-purpose production administration tools.

[Recorded output](evidence.txt) establishes:

- New-volume migrations and their repeat succeeded; both APIs became healthy and
  `/api/ready` returned `ready`. Restored `/api/health` returned `ok`.
- **47 public tables / 277 rows matched exactly**, using per-table row counts and
  SHA-256 over ordered canonical row JSON, before restored login or other writes.
  This includes 32 migration records/checksums, 4 accounts/players, 3 tournaments,
  3 rounds, 38 scores and 38 score audits, 3 round handicap snapshots, memberships,
  flights, course data, invitation redemption, sharing audit and 8 sessions.
- Administrator and member tournament/player/round reads, gross/net round and
  tournament standings, the administrator scoring card, and public responses
  matched the pre-backup JSON exactly. Fresh logins used the restored passwords.
- Preserved full-round totals were administrator **74 gross / 62 net**, member
  **92 gross / 74 net**, with playing handicaps 12 and 18. Anonymous private
  access remained 401, authenticated nonmember access 403. The public capability
  still showed only front-nine totals **36 / 45**, even with an admin cookie.
- Runtime `golfsiden_app` remained non-superuser, unable to create databases,
  roles or public-schema objects, and able only to read migration history.
  Actual CREATE TABLE and migration UPDATE probes failed. Their first diagnostic
  run expected process exit 1; `psql ON_ERROR_STOP` returns 3. Corrected checks passed.
- Missing confirmation exited 2; mismatched checksum exited 1; an incomplete
  archive with a valid checksum failed without public objects remaining; and a
  repeat restore refused the populated target (358 public objects).
- A stronger late-failure probe used a separate task-owned database in the
  disposable restore cluster. The archive's complete TOC was readable; verbose
  restore showed table creation and data loading before the injected EOF.
  `--single-transaction` rolled it all back to **zero public objects**. The probe
  database was then removed. This used the same restore flags with additional
  verbosity, rather than the wrapper script's fixed database target.
- Restored UI logins and exact administrator/member totals passed at 390×844 and
  1280×900 without horizontal overflow, page/console errors or unexpected HTTP
  errors. Programmatically focusing the last mobile row scrolled it above the
  fixed navigation; actual keyboard traversal is covered by the earlier navigation
  report.
  Four screenshots were visually inspected: [admin phone](restored-admin-390.png),
  [admin desktop](restored-admin-1280.png), [member phone](restored-member-390.png),
  [member desktop](restored-member-1280.png).

## Confirmed finding: OPS-1 (P2)

`docs/deployment_guide.md`'s Backup example checks an absolute `.sha256` path while
remaining in the checkout. The backup script deliberately writes a basename-only
checksum entry. `sha256sum` resolves that entry against the current directory,
not the sidecar's directory, so the documented command fails on a valid backup:

```text
source.dump: FAILED open or read
sha256sum: source.dump: No such file or directory
```

Running `(cd /tmp/golf-rehearsal-738ec1d && sha256sum --check source.dump.sha256)`
passed immediately. The restore script already changes to the dump directory
and therefore restored successfully. Impact: a false backup-verification failure
in the documented operator flow; no observed backup corruption or restore loss.
The bounded next repair is to correct the guide example and verify it using a
sidecar outside the checkout. Application and script changes are unnecessary.

## Limits and cleanup

This proves recovery of the exercised individual-play data and exact stored rows,
not every scoring format or a production-size recovery time objective. No external
backup store, encryption/retention policy, host-loss transfer, public certificate,
Docker Engine deployment, physical Android or native 200% browser zoom was tested.
Browser-local drafts are outside PostgreSQL backups. Restored session rows matched,
but continued use of a pre-backup browser cookie was not separately exercised.
The separate security review is preserved and is not replaced by these checks.

No production source changed, so full Rust/frontend unit, Clippy and database
regression ladders were not repeated. Current release/frontend image builds,
PostgreSQL migrations/restore, shell syntax and the focused API/Chrome checks
were the affected validation. Existing design/loading/error coverage remains in
the earlier friends and navigation reports.

Independent read-only review checked the retained fingerprints, build/migration
logs, late-failure evidence, screenshots, scope and preservation of the separate
security plan. Its one wording correction distinguished programmatic focus from
actual keyboard traversal and was applied before publication.

Only the two named rehearsal projects and their newly created volumes were
removed after evidence collection. The task-owned API socket was stopped.
The pre-existing build helper started by Compose was returned to its prior
stopped state, retaining its cache. Unrelated cancellation/security-review
containers were left running. Unique
rehearsal images and private temporary evidence remain local; no credentials,
raw dumps, tokens, session payloads or private environment files are published.
