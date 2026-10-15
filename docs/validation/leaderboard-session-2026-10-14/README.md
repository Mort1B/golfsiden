# A1 — session-bound private leaderboards

Base revision: `f80f41426cc8c9b885387054df211024de6cf761`.
Scope: only the four private round/tournament gross/net leaderboard endpoints,
their repository authority boundary, regression tests and documentation.
The [assessment](../application-review-2026-10-14/README.md) identified the
in-flight private-read revocation defect; sibling setup reads and A2–A8 are
outside this repair. No frontend, scoring formula, schema, dependency or hosted
configuration change is included.

## Result and transaction ownership

HTTP handlers now pass session IDs to explicitly named `*_for_session`
repository functions. `leaderboards/authority.rs` owns transaction setup,
session/user locking, final expiry recheck and bounded retry. All related facts
remain in one repeatable-read transaction. Session/user share locks precede
membership locks and remain held until commit. The established membership and
visibility projection paths still own tournament roles and embargo policy.

If revocation or credential replacement commits before transactional authority
is acquired, no result is returned. If the read already holds authority locks,
the later revocation/credential write waits until that read finishes. This
preserves a deliberate ordering; the repair does not try to retract a response
already sent. Database wall-clock expiry is checked again after result assembly,
including match-only not-applicable envelopes. Expired reads return the existing
401 envelope without standings. This check does not freeze time during COMMIT.

A PostgreSQL `40001` from either direct database work or nested authorization
errors discards the entire attempt. The server permits one fresh transaction
which repeats session, membership, visibility and fact loading. It never reuses
the previous user or role. Repeated conflict returns generic
`503 service_unavailable`, with no database details or result payload. Scoped
response middleware applies `private, no-store` on every status from the four
routes. API JSON, ordinary valid results, gross/net separation and public
capability projections remain unchanged. Four retained test callers were updated
to pass their actual session IDs through the renamed repository functions.

The normal private read adds two active-session queries (initial authority and
final recheck) and retains session/user share locks. Internal unrestricted and
public sharing assembly keep their existing authority and transaction owners.
There is no cross-request authority cache or timing improvement claim.

## Regression evidence

The new API/PostgreSQL regression was run before changing production code. It
failed at the first populated round leaderboard with **200 instead of 401** after
committed revocation. [The captured failure](baseline-regression.txt) is retained;
the same test passes after the repair.

The regression holds session and membership rows before dispatch. The extractor
can read the old committed active session without locks; the old implementation
then waits on membership while the repaired implementation waits on session
state. Revoking inside the holding transaction and committing proves the stale
snapshot is discarded before the permitted retry denies the read. Credential
replacement also prelocks the user row to avoid creating a test-only deadlock.

Ten new retained integration tests cover:

- Revocation and credential-generation replacement before transactional authority,
  across all four routes, with normal 401 bodies and non-cacheable denials.
- Original membership-only contention: later revocation waits behind the reader,
  the authorized response matches its baseline, then future requests return 401.
- Membership removal during a wait: fresh authorization returns 403 rather than
  an unexpected database 500.
- Administrator demotion during a wait: both round and tournament gross/net reads
  retry with the viewer role, recompute hidden-final visibility and match a fresh
  viewer response. Hidden back-nine facts do not survive from the failed attempt.
- Natural expiry during membership waits and later score-loading waits on all
  four endpoints, plus expiry of match-only not-applicable envelopes.
- Twelve concurrent valid reads with exact serialization parity against internal
  admin results, unchanged cache headers and no invalidation events.
- Request cancellation while waiting, release of authority, successful subsequent
  transaction setup and denial of the subsequently revoked session.

A new PostgreSQL-backed helper test raises real `40001` errors through direct
and nested error paths and proves exhaustion after two attempts. A fast response
test verifies the resulting generic private 503 contract. Existing authorization,
missing-resource, every-membership-role, hidden results, scoring, awards, match,
four-ball, Stableford and public-sharing tests remain in the validation ladder.

## Validation

| Check | Result |
| --- | --- |
| Original new regression against old source | Failed as expected: 200 instead of 401 |
| Focused PostgreSQL race/privacy/format/award suites | 90 passed |
| Focused leaderboard domain/repository/response tests with PostgreSQL | 50 passed |
| Rust formatting | Passed |
| Full all-target backend tests without database feature | 259 passed; 1 manual benchmark ignored |
| Clippy, all targets/features, warnings denied | Passed; existing 3 vendored SQLx warnings |
| API binary build | Passed |
| Full PostgreSQL-feature ladder | 748 passed; 4 manual performance probes ignored |
| Migration and development seed ladder | Passed, schema 39 |
| Focused Chrome leaderboard scenarios | Passed at 320, 390 and 1280px |
| Broader retained Chrome leaderboard flow | Blocked by existing obsolete score-status assertion; details below |
| Independent read-only authorization/concurrency review | No actionable findings |

All tests use synthetic data in disposable PostgreSQL 17 container
`golf-a1-20261014-pg`, local port 55449. SQLx tests create isolated test databases;
the browser owns a separate `golf_a1_browser` database so the final development
seed cannot replace its account credentials. No hosted infrastructure was used.

Reproduction from the repository root with an approved disposable `DATABASE_URL`:

```bash
cargo fmt --all -- --check
cargo test -p golf-api --features database-tests --test leaderboard_sessions
cargo test -p golf-api --features database-tests --lib leaderboards::
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build -p golf-api --bin golf-api
cargo test --workspace --all-targets --features database-tests
cargo run -p golf-api --bin migrate
cargo run -p golf-api --bin seed
```

The broader retained browser command runs from `frontend/` with Vite on 5173, port 3000
free, and `DATABASE_URL` selecting its independently migrated/seeded fixture:

```bash
GOLF_LEADERBOARD_LIVE_BROWSER=1 npx playwright test --config playwright.lifecycle.config.ts leaderboardLive.browser.ts --output=/tmp/golf-a1-browser-results
```

The suite owns its API child process, including deliberate crash/restart. Local
logs are `/tmp/golf-a1-{before,focused,retry-tests,backend,clippy,build,database,migrate,seed,browser}.log`.
No frontend source or JSON shape changed, so its unit/type/lint/build ladder is
not repeated for this backend-only step. No physical-device, public hosting or
production load test is included. The manual performance probes remain outside
this authority repair.


## Browser evidence and remaining test maintenance

Follow-up: [A1x test maintenance](../leaderboard-browser-maintenance-2026-10-15/README.md)
reproduces the stale label, updates the current failed-delivery fixture and passes
the complete retained reconnect/hidden-final case. The A1 snapshot below remains
historical and did not establish those later scenarios.

The unchanged broader suite stopped at `frontend/e2e/leaderboardLive.browser.ts:71`:
it expects `Synkronisert` or `Lagret`, while existing `HoleEntry.tsx:28` renders
`Lagret på serveren`. The same old assertion appears at line 103. The captured
[retained-suite failure](retained-browser-failure.txt) shows the score reach its
acknowledged server state. No backend exception or authorization failure caused
this assertion. The rest of that suite did not run, so this report does not claim
its later reconnection/hidden-final scenarios passed. Test maintenance is queued
separately; the test was not weakened or changed in A1.

A focused [Chrome probe](browser-probe.mjs), with
[captured results](browser-results.json), instead exercises the four affected
endpoints against real API/PostgreSQL data at 320, 390 and 1280px. At each width it
checks a deliberately delayed loading state, populated data and long-name wrapping,
an injected 503 with no retained standings, successful read-only retry, a draft
round's empty state, and all four private endpoints returning 401/no-store after
logout. Console/page exceptions and unexpected network failures were absent.
Representative mobile populated and desktop error screenshots were inspected;
local screenshots are `/tmp/golf-a1-{populated,error,empty}-{320,390,1280}.png`.
The probe's first diagnostic attempt counted only scorecard links rather than
all result rows; the corrected selector includes unstarted owners. This was a
probe assertion error, not an application failure.

To reproduce the focused probe, run the repaired local API plus Vite against a
synthetic seed with the first seeded round open and its first team/hole scored,
second round still draft, and a long synthetic player display name. This fixture
matches `backend/tests/leaderboard_sessions/support.rs`, with the extra long name.
The previous retained run supplied the same opened/scored fixture before its
text assertion failed. Then run:

```bash
node docs/validation/leaderboard-session-2026-10-14/browser-probe.mjs
```

The PostgreSQL total includes ordinary backend tests; it is not additive with
259. The four ignored cases are one projection benchmark and three pre-existing
PostgreSQL profiling probes. None was run in this authority-only step.
