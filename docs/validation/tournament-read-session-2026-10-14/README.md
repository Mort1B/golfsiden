# A1b — session-bound private tournament reads

Base revision: `8c6a68f5478ba67b240c3b38554549e2fa33d403`.
Scope: `GET /api/tournaments`, `GET /api/me/tournaments`, and
`GET /api/tournaments/{id}`; their authority owner, regressions and docs.
This is the next bounded follow-up to [A1](../application-review-2026-10-14/README.md#a1--p2-revoked-sessions-can-finish-private-reads).
Round/roster/pairing/team reads and legacy writes remain separate work. No schema,
frontend source, dependencies, scoring rules or hosted configuration changed.

## Result and transaction boundary

The handlers now pass the session ID through explicitly named `*_for_session`
functions. `repositories/tournaments/reads.rs` owns one repeatable-read transaction
per attempt, derives the user from an active session and holds session/user share
locks before membership locks. Collections lock the user's existing memberships
in tournament UUID order in one query, then fetch their unchanged payload in the
same snapshot. Detail reads retain exact stored membership and existing
missing/cross-tournament contracts. The final active-session query uses database
wall-clock time after payload assembly, including for empty lists.

A revocation or credential replacement that precedes transactional authority
denies the read. A later writer waits behind an already authorized read; no
retraction of an already sent response is promised. Session expiry during later
waits returns the established 401 body without tournament data. Share locks remain
held until commit; the clock is not frozen during COMMIT.

A direct or nested authorization SQLSTATE `40001` discards the complete attempt,
including its user, role and results. One fresh transaction redoes all authority
and data loading. A second conflict returns the existing generic
`503 service_unavailable` envelope. No mutations retry. Scoped middleware makes
all responses from these three GET routes `private, no-store`, including
extractor, path, authorization and unavailable errors; sibling routes retain
their existing ownership.

Both collection orders are preserved: `/api/tournaments` uses descending start
date, name, UUID; `/api/me/tournaments` uses descending start date, lowercased name,
UUID. The own-membership shape still contains the exact membership role and only
links a player when that user's stored player is an entrant in that tournament.
Account membership and player participation remain independent. Existing complete
and archived tournaments stay readable to their members. There is no global-role
bypass and no cross-request authority cache.

This security repair adds two active-session queries to each ordinary read and
one ordered membership-lock query to each collection, plus transaction setup.
Query count is independent of the number of tournaments; no query-per-tournament
loop was introduced. No latency or production capacity improvement is claimed.

## Regression evidence

The new controlled API/PostgreSQL regression ran before production edits.
**All three routes returned 200 after session revocation committed.**
The test then failed its expected 401 assertion; the
[captured original failure](baseline-regression.txt) is retained. The same test
passes with the repair, returning private 401 envelopes on all three routes.

The holding transaction locks the session and tournament table before dispatch.
The extractor's nonlocking lookup still sees the active committed session. The
old repository waits on the tournament table; the repaired repository waits on
session authority. Revocation inside the holder followed by commit proves fresh
transactional authorization is required before returning private data. The
credential-generation case also prelocks the user row to avoid a test-only
lock-order inversion.

Thirteen new retained PostgreSQL/API cases cover:

- Revocation and credential replacement before authority on each route.
- Reads already holding session/user locks finish before later revocation or
  password changes; later requests are denied.
- Membership removal during the lock wait retries against fresh authority:
  collections omit the removed tournament and detail returns 403.
- Membership demotion during the wait discards the old role and matches a fresh
  viewer response on all three routes.
- Membership inserted after the snapshot appears only on the next read; collection
  membership removal arriving after share locks waits behind the read.
- Twelve concurrent valid reads preserve exact tournament JSON, roles, player-link
  nullability and cache policy, with unchanged persisted authority/data and no SSE.
- Cancellation releases held authority and permits revocation and later reads.
- Natural expiry during membership or tournament-table loading waits on every
  route, plus otherwise-empty collections. Tests observe a real database wait
  before expiry and use an eight-second pre-gate window.
- Stable ordering, positive/negative entrant links, empty lists, missing targets,
  cross-scope denial, invalid paths and unauthenticated private cache headers.

Repository helper tests raise real PostgreSQL 40001 errors through direct and
nested paths and prove exactly two attempts. An ordinary error gets no retry.
The fast API helper test verifies generic 503 JSON and private caching.
Retained workspace tests cover every membership role and resource isolation;
archive/completion tests use their real session IDs through the renamed read
boundary and retain historic payload checks.

The first expanded test run passed all authorization and expiry cases but stopped
on a fixture binding text as `tournament_role` (SQLSTATE 42804). It now binds the
Rust enum. Read-only review also caught that the proposed positive player-link
fixture reused an already-claimed player; the corrected fixture creates a
separate unclaimed player and entrant. These were test setup corrections, with no
production workaround. The corrected focused run passes all 48 cases.

## Validation

| Check | Result |
| --- | --- |
| Original regression against old source | All three routes returned 200 after committed revocation; failed as expected |
| Focused PostgreSQL/private workspace/archive/completion suites | 48 passed |
| Repository retry and API unavailable helper tests | 3 passed |
| Rust formatting | Passed |
| Full backend all-target tests without database feature | 261 passed; 1 manual benchmark ignored |
| Clippy, all targets/features, warnings denied | Passed; 3 existing vendored SQLx warnings |
| All API binaries | Built |
| Local API health/readiness | Passed |
| Full PostgreSQL-feature ladder | 770 passed; 4 manual performance probes ignored |
| Migration/development seed ladder | Passed, schema 39 |
| Real Chrome, 3 flows at each of 320/390/1280px | 9 passed |
| Independent read-only authorization/concurrency review | No remaining findings |

The disposable PostgreSQL 17 container is `golf-a1b-20261014-pg` on loopback port
55449. SQLx tests own isolated databases. Real browser fixtures use the separate
`golf_a1b_browser` database so the final development seed cannot change browser
credentials. No hosted infrastructure or production accounts are involved.
The owned container and local API/Vite processes were removed/stopped after
validation.

Commands, from the repository root with a disposable `DATABASE_URL`:

```bash
cargo fmt --all -- --check
cargo test -p golf-api --features database-tests --test tournament_read_sessions --test private_workspace_reads --test tournament_archive --test tournament_completion -- --test-threads=2
cargo test -p golf-api --features database-tests --lib tournaments::
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build -p golf-api --bins
cargo test --workspace --all-targets --features database-tests -- --test-threads=2
cargo run -p golf-api --bin migrate
cargo run -p golf-api --bin seed
node docs/validation/tournament-read-session-2026-10-14/browser-probe.mjs
```

Local diagnostic logs are `/tmp/golf-a1b-{baseline,focused-first,focused,helper-tests,fmt,backend,clippy,build,database,migrate,seed,browser}.log`.
No frontend source or JSON shape changed, so the frontend unit/type/lint/build
ladder is unaffected. Manual performance probes, physical devices, public hosting
and production-load acceptance are outside this step. The PostgreSQL total
includes ordinary backend tests and is not additive with 261. The four ignored cases are one projection benchmark and three existing
PostgreSQL profiling probes. The separately queued retained A1 expiry-test timing
investigation is unchanged; its existing case passed in this two-thread full run.

## Browser evidence

The browser command requires the repaired API on port 3000 (`APP_ENV=development`
and `DATABASE_URL` selecting the separately migrated/seeded synthetic browser
fixture), plus Vite on port 5173 (`cd frontend` then `npm run dev -- --host
127.0.0.1`). The seeded admin uses the development seed password. Keep the seeded
tournament draft; the probe prepares its own long content and needs no opened
rounds. It uses the installed Playwright package and real Chrome channel.

The [focused Chrome probe](browser-probe.mjs) prepares its synthetic long title
and description through the existing authorized draft-details API. A first local
fixture attempt to update those fields directly was correctly rejected by the
existing database guard; no guard was disabled or changed.

The probe exercises each affected route's actual UI consumer at 320, 390 and
1280px: tournament list, tournament detail, and score-workspace tournament list.
It checks delayed loading, real API/PostgreSQL payloads, long-content wrapping,
injected 503 and explicit read-only retry, empty collections or denied detail,
private 401 contracts after logout, page exceptions and unexpected network errors.
Injected responses exercise existing display states; PostgreSQL races above
establish the actual authorization repair. All nine flows passed with no page
exceptions, console errors or unexpected network failures; see the [captured results](browser-results.json). Each flow
preserved no-horizontal-overflow checks. The score workspace's real draft fixture
shows the existing no-open-round state; its injected empty collection separately
shows the no-tournaments state. Detail's injected 403 hides the private title.

The [mobile populated list](mobile-list.png), long 390px tournament overview and
[desktop error](desktop-error.png) screenshots were visually inspected. All
loading/populated/error/empty/denied captures remain locally under
`/tmp/golf-a1b-{mine,detail,list}-*-{320,390,1280}.png`. These checks cover local
Chrome, not physical iPhone or hosted acceptance.
