# A1c — session-bound private roster and round reads

Base revision: `138c7f4812f0d78cba42882e7ed8dab3866d3bfd`.
Scope: `GET /api/tournaments/{id}/players`, `GET
/api/tournaments/{id}/rounds`, `GET /api/rounds/{id}`, focused session support,
regressions and documentation. This follows
[A1b](../tournament-read-session-2026-10-14/README.md) and the
[assessment's A1 finding](../application-review-2026-10-14/README.md#a1--p2-revoked-sessions-can-finish-private-reads).
Pairing, team and readiness reads and legacy mutations remain separate work.
No frontend source, schema, dependencies, scoring rules or hosted configuration
changed.

## Authority and compatibility

All three handlers now carry the session ID into explicitly named
`*_for_session` repositories. Their transaction derives the user from locked
active session/user rows before the exact membership check. Round ownership is
resolved from its stored tournament relation. Those share locks remain held
through payload assembly, followed by another active-session query using database
wall-clock time immediately before commit. Empty roster and round-list results
also require current authority. The clock is not frozen during COMMIT.

If revocation or credential replacement wins the authority lock, no private
payload is returned. A later writer waits behind a reader already holding the
corresponding authority locks. This establishes ordering; it does not retract an
already delivered response. Natural expiry during later waits returns the
existing 401 envelope without roster/round data.

The proven A1b setup/final-check/retry helper and error type were extracted into
`repositories/session_read.rs` to share one authority boundary across these
repositories. `TournamentReadError` remains a public alias, preserving A1b source
callers and error behavior. Its payload queries and ordered collection membership
locks remain local to `tournaments/reads.rs`. Leaderboard authority and
unrestricted internal read owners remain unchanged.

The shared helper discards a complete attempt on direct or nested authorization
SQLSTATE 40001 and retries once with fresh session, membership and payload. A
second conflict returns generic `503 service_unavailable`, with no database
information or partial data. Other errors are not retried. `api/private_read.rs`
shares the existing error mapping and `private, no-store` response helper.
Middleware is applied to the selected GET MethodRouters; it is installed before
adding POST round creation so that POST retains its existing behavior.
`rounds::preflight_create` retains its old transaction helper. No mutation gains a
retry or a changed session contract.

Existing roster SQL still returns ordered player/entrant data and the preserved
editable/locked handicap reason in one snapshot. Round list ordering, status,
format, course/tee identity, timestamps and other fields stay unchanged. All four
exact membership roles can read; a global administrator cannot bypass another
tournament's membership. Account access remains independent of player/team
participation. This repair adds two active-session queries per ordinary read,
with no query-per-player or query-per-round loop. No latency or capacity
improvement is claimed.

## Reproduction and regressions

The retained API/PostgreSQL regression ran before production edits. All three
routes returned **200 after session revocation committed**, and the expected 401
assertion failed. The [original failure](baseline-regression.txt) records each
route. With the repair, the same regression returns private 401 bodies on all
three routes.

A holding transaction locks the session and exact membership before dispatch. The
extractor can still see the old committed active session without locking. The
original repositories then wait on membership; repaired readers wait on session.
Revocation inside the holder and commit force the repaired read to discard its
stale authority snapshot before the permitted fresh attempt denies it. Credential
replacement also prelocks the user row to avoid a test-only lock-order inversion.

Thirteen new integration tests cover:

- Revocation and credential replacement before transactional authorization on all
  three routes, and later revocation/password changes waiting behind authorized
  readers before fresh requests are denied.
- Membership removal during the wait producing fresh private 403s, and demotion
  discarding the old snapshot while retaining legitimate member access.
- Natural expiry during membership waits on each route; during the roster's final
  handicap-lock query after player loading; during round-list payload loading;
  and during round-detail stored-ownership lookup. The `rounds` table gate for
  detail occurs before membership, so it is not claimed as a post-membership
  payload wait. Tests observe actual PostgreSQL blocking before natural expiry,
  using an eight-second pre-gate window.
- Empty roster and round-list payloads, plus their expiry after controlled loading
  waits in a naturally empty member-owned tournament.
- Twelve concurrent valid reads matching unrestricted payloads, stable ordering,
  preserved entrant handicap, unchanged persisted data/authority digests and no
  SSE. Cancellation releases authority and leaves later requests usable.
- Every exact membership role, a global admin with membership only in another
  trip, missing/invalid targets, unauthenticated replies and private cache policy.
  A POST control retains its original `no-store` authentication denial.
- Real tournament start/open preserving roster entries and the first stored
  `snapshot_captured` lock reason, plus exact open-round payload parity.

The first expanded run passed all authorization and populated expiry tests but
exposed fixture assumptions: deleting the seeded mandatory final round violates
its existing foreign key, and opening first records `snapshot_captured` before
the later `round_opened` marker can be inserted. The corrected empty tests create
a fresh empty tournament; the opened-roster test asserts the preserved first
reason. No production rule was changed to accommodate either fixture.

Retained A1b, private workspace, handicap correction and round-creation suites
pass with the extracted helper and scoped middleware. Shared retry tests still
raise real direct/nested 40001 errors, prove two attempts and no retry for an
ordinary denial. The existing fast API test retains generic private 503 JSON.
Independent read-only review found no remaining source or test findings after
fixture corrections.

## Validation

| Check | Result |
| --- | --- |
| New regression against original source | Failed as expected: 200 on all three routes after revocation |
| Focused new/A1b/workspace/handicap/round-creation PostgreSQL suites | 43 passed |
| Shared repository retry helpers | 2 passed |
| Formatting | Passed |
| Full backend all-target tests | 261 passed; 1 manual benchmark ignored |
| Clippy, all targets/features, warnings denied | Passed; 3 existing vendored SQLx warnings |
| All API binaries | Built |
| Local API health/readiness | Passed |
| Full PostgreSQL-feature ladder | 783 passed; 4 manual performance probes ignored |
| Explicit migration/development seed | Passed, schema 39 |
| Real Chrome, 3 flows at 320/390/1280px | 9 passed |
| Independent read-only review | No remaining findings |

All validation uses synthetic data in disposable PostgreSQL 17 container
`golf-a1c-20261014-pg` on loopback port 55449. SQLx tests own isolated databases;
browser fixtures use separate `golf_a1c_browser` so development seeding cannot
replace browser credentials. No hosted infrastructure or production account was
used. The local API/Vite processes and the owned PostgreSQL container were
stopped/removed after validation.

Commands with an approved disposable `DATABASE_URL`, from the repository root:

```bash
cargo test -p golf-api --features database-tests --test roster_round_sessions --test tournament_read_sessions --test private_workspace_reads --test tournament_handicap_corrections --test round_creation_authorization -- --test-threads=2
cargo test -p golf-api --features database-tests --lib session_read::
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build -p golf-api --bins
cargo test --workspace --all-targets --features database-tests -- --test-threads=2
cargo run -p golf-api --bin migrate
cargo run -p golf-api --bin seed
node docs/validation/roster-round-session-2026-10-14/browser-probe.mjs
```

Local logs are `/tmp/golf-a1c-{baseline,focused-first,focused-full,helper-tests,fmt,backend,clippy,build,database,migrate,seed,browser}.log`.
The backend-only repair changes no frontend code or JSON shape, so the frontend
unit/type/lint/build ladder is unaffected and was not repeated. Manual performance
probes, physical devices, public-host operation and production-load testing are
outside this step. The separately queued A1 expiry timing investigation and
retained leaderboard browser-status maintenance are unchanged. The existing A1
expiry case passed in this two-thread full run. The PostgreSQL total includes
ordinary backend tests and is not additive with 261; the four ignored cases are
one projection benchmark and three existing PostgreSQL profiling probes.

## Browser evidence

The [focused probe](browser-probe.mjs) requires the repaired API on port 3000,
`APP_ENV=development` and `DATABASE_URL` selecting the independently migrated and
seeded synthetic browser database; Vite runs on 5173 (`cd frontend`, `npm run dev
-- --host 127.0.0.1`). It uses the installed Playwright package and real Chrome.
Keep the seeded trip/round draft. The probe creates one synthetic long-name
entrant and configures long course/tee names through the existing authorized
admin APIs. It does not print claim tokens or modify database guards.

At each width the probe exercises the roster and round list on tournament
overview, and round detail on its actual route. It checks delayed loading,
real API/PostgreSQL payloads, long content, injected 503 with explicit read-only
retry, empty roster/list or denied detail, all three private 401 replies after
logout, page/console exceptions and unexpected network failures. Injected UI
states supplement the actual PostgreSQL race/expiry regressions; they do not
establish server authorization themselves. All nine flows passed with no page exceptions, console errors or unexpected
network failures; see the [captured results](browser-results.json). Each flow
passed its no-horizontal-overflow assertion. The [mobile populated roster](mobile-roster.png),
long-name 390px round detail and [desktop error](desktop-error.png) screenshots
were visually inspected. All screenshots remain locally under
`/tmp/golf-a1c-{roster,round-list,round}-*-{320,390,1280}.png`. Local Chrome coverage
does not replace physical iPhone or hosted acceptance.
