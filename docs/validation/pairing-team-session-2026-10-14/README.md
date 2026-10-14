# A1d — session-bound private pairing and team reads

Base revision: `7b0985844d68b61afcae54723b2176a8375534aa`.
Scope: `GET /api/rounds/{id}/pairings`, `GET /api/rounds/{id}/teams`,
focused authority support, regressions and docs. This follows
[A1c](../roster-round-session-2026-10-14/README.md) and the
[assessment's A1 finding](../application-review-2026-10-14/README.md#a1--p2-revoked-sessions-can-finish-private-reads).
Pairing PUT, readiness reads, legacy writes, scoring, frontend, schema and
dependencies remain outside this step. Team query batching remains A8.

## Result and transaction ownership

Both handlers pass session IDs to explicitly named `*_for_session` repositories.
The unchanged `session_read.rs` helper owns repeatable-read setup, initial
session/user share locks, a final database wall-clock expiry check and one full
read retry on direct or nested authorization SQLSTATE 40001. Each attempt derives
the user from the active session and resolves the exact tournament membership
through the stored round parent. Session/user/membership locks remain held until
commit, across all entrant, group and member loading. Empty responses also require
current authority. The clock is not frozen during COMMIT.

Revocation or credential replacement that wins transactional authority denies
the read without its payload. A later writer serializes behind a reader already
holding its corresponding authority locks. This does not retract an already
delivered response. Natural expiry during later database waits returns the
existing 401 body. Snapshot invalidation discards the whole attempt, including
its user, membership and payload; a second conflict returns generic private
`503 service_unavailable` rather than data or database details.

Pairing GET now maps `SessionReadError` through the shared API mapper. Its former
reachable errors were authorization or SQLx failures, so existing HTTP contracts
are preserved. The broader `RoundPairingsError` still belongs to PUT/preflight;
those paths gain no retry or authority change. The existing combined GET/PUT
private cache middleware remains unchanged. Team GET adds shared middleware so
all statuses, including extractor/path errors, use `private, no-store`.

Payload loaders are unchanged. Pairings preserve the round ID/parent, status,
format, timestamp, active/inactive entrants, distinct team/flight identities,
schedules and legacy individual-group classification. Team responses remain flat
with members. Their independent ordering remains deliberate: pairings use
lowercased group/member names plus stable identifiers; team listing uses its
existing case-sensitive names and member ordering. All membership roles remain
readable, with no global-admin bypass or coupling to account/player/team
participation.

The ordinary read adds two active-session queries. No group/player query loop was
added; the existing team member query per team remains unchanged and deferred to
A8. No query-batching, latency or capacity improvement is claimed.

## Regression evidence

The new API/PostgreSQL regression ran before production edits. **Both routes
returned 200 after committed session revocation** and failed the expected 401
assertion; the [original failure](baseline-regression.txt) records both endpoints.
The repaired source passes the same test with private 401 envelopes.

The holding transaction locks session and exact membership before dispatch.
The extractor's nonlocking read sees the old committed active session. Original
repositories wait on membership; repaired readers wait on session. Revocation in
the holder followed by commit forces stale authority to be discarded before the
fresh attempt denies it. The credential-generation case also prelocks the user
row to avoid a test-only lock-order inversion.

Fourteen new PostgreSQL/API cases cover:

- Session revocation and credential replacement before authority, plus later
  revocation/password writes waiting behind already authorized readers and fresh
  requests subsequently being denied.
- Membership removal returning fresh private 403, and demotion combined with a
  changed member order requiring a genuinely fresh payload after retry.
- Natural expiry during membership waits, the pairing loader's final
  `flight_memberships` query, and populated team member loading. The team table
  gate blocks the first member query, not a claimed final-team query. Empty team
  lists skip that loop, so their expiry test instead gates `teams`; empty pairing
  loads still query flight memberships. All cases observe PostgreSQL blocking
  before expiry, using an eight-second pre-gate window.
- Twelve concurrent valid reads preserving payloads, ordering and member/round
  identity without persisted changes or SSE; cancellation releases authority and
  permits subsequent reads.
- Group metadata and member order changing during member loading: the pending
  read returns its coherent old snapshot, and a fresh read returns changed facts.
- Every exact membership role, a global admin with membership only in another
  trip, missing/invalid IDs, private cache policy, and unchanged PUT authentication
  denial.
- Typed empty pairing layouts and empty team arrays; legacy individual groups
  preserve identifiers, schedule and timestamps, with explicit case-folded versus
  case-sensitive group/member ordering and retained inactive entrants.

The first expanded run passed 12 cases, including authority, expiry and snapshot
coherence, but two contract assertions incorrectly expected nested `team` JSON.
`TeamWithMembers` deliberately serializes its team fields flat. The corrected
assertions use top-level IDs/timestamps; production serialization was preserved.
The corrected focused run passes all 31 new/retained cases.

Retained pairing tests cover replacement concurrency, legacy conversion,
scheduled transfer, exact roles, stale/invalid writes, retired granular team
routes and post-commit SSE. Shared helper tests still prove a two-attempt limit
for real direct/nested 40001 errors and no retry for ordinary denial. The existing
fast API test verifies generic private 503 JSON. A1b and A1c compatibility remains
in the complete PostgreSQL ladder. Independent read-only review found no remaining
source/test findings after the JSON fixture correction.

## Validation

| Check | Result |
| --- | --- |
| Original new regression | Failed as expected: both routes 200 after committed revocation |
| Focused new/private-workspace/pairing suites | 31 passed |
| Shared retry helper tests | 2 passed |
| Formatting | Passed |
| Full backend all-target tests | 261 passed; 1 manual benchmark ignored |
| Clippy, all targets/features, warnings denied | Passed; 3 existing vendored SQLx warnings |
| All API binaries | Built |
| Local API health/readiness | Passed |
| Full PostgreSQL-feature ladder | 797 passed; 4 manual performance probes ignored |
| Explicit migration/development seed | Passed, schema 39 |
| Chrome pairing UI and typed team API at 320/390/1280px | Passed at all 3 widths |
| Independent read-only authorization/concurrency review | No remaining findings |

Validation uses synthetic data in disposable PostgreSQL 17 container
`golf-a1d-20261014-pg` on loopback port 55449. SQLx tests own isolated databases;
browser fixtures use separate `golf_a1d_browser` so final seeding cannot replace
their credentials. No production accounts or hosted infrastructure were used.
The local API/Vite processes and owned PostgreSQL container were
stopped/removed after validation.

Commands from the repository root with a disposable `DATABASE_URL`:

```bash
cargo test -p golf-api --features database-tests --test pairing_team_sessions --test round_pairings --test private_workspace_reads -- --test-threads=2
cargo test -p golf-api --features database-tests --lib session_read::
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build -p golf-api --bins
cargo test --workspace --all-targets --features database-tests -- --test-threads=2
cargo run -p golf-api --bin migrate
cargo run -p golf-api --bin seed
node docs/validation/pairing-team-session-2026-10-14/browser-probe.mjs
```

Logs are `/tmp/golf-a1d-{baseline,repaired,focused-first,focused,helper-tests,fmt,backend,clippy,build,database,migrate,seed,browser}.log`.
No frontend source or JSON shape changed, so the frontend unit/type/lint/build
ladder is unaffected and was not repeated. Manual performance probes, physical
devices, public-host operation and production-load testing remain outside this
step. The existing A1 expiry-test timing investigation and retained leaderboard
browser-status maintenance are unchanged. The existing A1 expiry case passed
in this two-thread full run. The PostgreSQL total includes ordinary backend
tests and is not additive with 261. The four ignored cases are one projection
benchmark and three existing PostgreSQL profiling probes.

## Browser evidence

The [probe](browser-probe.mjs) uses the repaired API on port 3000 with
`APP_ENV=development` and `DATABASE_URL` selecting the separately migrated/seeded
synthetic fixture, plus Vite on 5173 (`cd frontend`, `npm run dev -- --host
127.0.0.1`). It uses installed Playwright and real Chrome. Keep the seeded first
round draft; the probe changes synthetic team/flight names through the actual
pairing replacement API, preserving identities, membership and schedules.

At 320, 390 and 1280px the real round-detail UI passes delayed pairing loading,
populated groups with long names and no horizontal overflow, injected 503 with
explicit read-only retry, empty flights/teams and denied pairing states. Both
endpoints pass real 200/JSON/private-cache and post-logout 401 checks.
`/teams` has no current production UI consumer, so the probe separately invokes
its real typed API module/decoder inside the browser and verifies exact round,
tournament and member identity. It does not invent a UI consumer or claim team
endpoint loading/error screens.

All three width runs passed with no page exceptions, console errors or unexpected
network failures; see [captured results](browser-results.json). Injected UI
responses supplement the real database authorization/expiry tests. The
[mobile populated pairing](mobile-pairings.png) and
[desktop error](desktop-error.png) screenshots were visually inspected. Local
captures for all states remain `/tmp/golf-a1d-*-{320,390,1280}.png`. This does not
replace physical iPhone, hosted or production-load acceptance.
