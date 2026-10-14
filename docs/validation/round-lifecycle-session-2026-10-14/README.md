# A2 — round lifecycle session expiry

Base revision: `4d216709642460f32dc2c391a705cf8d86208ea1`.
Scope: the authorized round opening, completion and locking transaction wrappers,
PostgreSQL/API regressions and affected documentation. The
[application assessment](../application-review-2026-10-14/README.md#a2--p2-lifecycle-authorization-expires-before-commit)
identified natural expiry during a membership wait. Other private read/write
families and A3–A8 remain separate work. No frontend, sporting rule, schema,
dependency or deployment configuration changes are included.

## Result and transaction ownership

Both authorized wrappers retain their existing round, session/user, exact-admin
membership and readiness locks. After all inner work, they call the existing
`auth::lock_active_session` immediately before commit. Its database
`clock_timestamp()` check catches expiry during earlier waits. The completion
wrapper performs this check after the singles-match inner branch returns, too.
The final query reuses session/user authority locks already held, rather than
introducing a new lock order.

Expiry returns the existing `401 unauthenticated` / `no-store` contract and drops
the uncommitted transaction. Round status/timestamps, opening player/team handicap
snapshots and the first tournament handicap-lock marker roll back together.
Score data, confirmations and audit history remain intact. The handler publishes
no SSE invalidation on failure; a valid successful transition retains exactly one
round event after commit. A renewed valid session can deliberately submit again.
There is no automatic mutation retry, new authority cache or performance claim.
The final check does not freeze time during COMMIT itself.

## Regression evidence

The new regression ran against the original production code before the repair.
Opening after expiry during a membership wait returned **200 instead of 401**;
the [captured baseline failure](baseline-regression.txt) is retained. That same
case passes after the repair.

Six retained PostgreSQL/API tests cover the product of three formats
(individual stroke play, two-player foursomes and singles match play) and two
wait stages. Each test exercises open, complete and lock with an expired session,
then a renewed valid-session control: **18 expired and 18 valid requests**.

- Membership contention uses an unchanged admin membership row held `FOR UPDATE`.
  The test observes the request blocked in PostgreSQL, waits for database-clock
  expiry and then releases the row.
- Late contention uses a test-only `AFTER UPDATE OF status` trigger and advisory
  lock. The trigger verifies all four player snapshots, both foursomes snapshots
  where applicable, and the tournament handicap-lock marker already exist before
  pausing. This proves rejection after pending lifecycle writes, including the
  separate match completion path.
- Rejected requests must return 401 with `unauthenticated` and `no-store`, leave
  the entire stored round/tournament state and related snapshot, score,
  confirmation and match/audit/receipt rows equal to their prior values, and
  publish no event while blocked or after rejection.
- Valid controls wait at the same stage, return 200 with the target status,
  preserve snapshots and scoring history, and publish exactly one round event.
  First opening starts without snapshots or a tournament handicap-lock marker;
  the renewed control creates their expected counts once.

The foursomes fixture creates its draft format and 50% allowance together to
satisfy the existing database constraint. An initial diagnostic fixture used an
invalid 100% allowance and was corrected. A subsequent parallel run timed out
before observing the foursomes late gate; that case passed in isolation. The
fixture now allows five seconds before expiry and reports any response arriving
before its observed wait. The initial gate timeout did not capture the request's
response, so its cause was not conclusively established. It is distinct from the
retained old-code 200/401 failure.

## Validation

| Check | Result |
| --- | --- |
| New regression against original source | Failed as expected: 200 instead of 401 |
| Focused lifecycle expiry matrix and retained full-run repeat | 6 passed each, covering 36 requests per run; [full-run result](lifecycle-results.txt) |
| Rust formatting | Passed |
| Full all-target backend tests without database feature | 259 passed; 1 manual benchmark ignored |
| Clippy, all targets/features, warnings denied | Passed; existing 3 vendored SQLx warnings |
| API binary build | Passed |
| Full PostgreSQL-feature ladder, two test threads | 754 passed; 4 manual performance probes ignored |
| Migration and development seed ladder | Passed, schema 39 |
| Retained Chrome lifecycle suite | 2 passed |
| Independent read-only authorization/concurrency review | No actionable findings |

The backend checks include all production and test changes. The full database
run also exercises the strengthened late-gate snapshot assertions. Its total
includes ordinary backend tests and is not additive with 259.

The initial default-concurrency PostgreSQL ladder stopped in the unchanged A1
`expiry_during_score_loading_denies_all_boards` test, before observing its
controlled wait: [captured failure](initial-database-failure.txt). Its session
window is two seconds. That test [passed in isolation](retained-test-rerun.txt)
afterward. The complete
ladder passed with `-- --test-threads=2`, without concurrent browser activity;
no A1 source or assertion is changed. These observations establish timing
sensitivity in the test setup, rather than proving the initial timeout's precise
cause or a product regression.
The thread limit changes test-case scheduling only; the races still spawn their
concurrent requests and transactions inside each case.

## Browser evidence

The unchanged retained suite passed both scenarios in real Chrome:
[captured result](browser-results.txt). It exercises admin opening, completion,
locking, mounted live readiness, scorecard confirmation/correction, a second
administrator's live lock, member restrictions and final-round privacy against
real API/PostgreSQL state. It also covers delayed loading, injected 503/retry,
empty and long-content states with route fixtures. Widths are 320, 390 and 1440px;
the layout checks reject horizontal overflow and undersized lifecycle controls.
The suite's page/console and unexpected-request/response assertions passed.

The local API emitted four PostgreSQL notices saying `there is no transaction in
progress` during the browser run. The HTTP and stored-state assertions passed;
this step does not attribute or repair that driver notice behavior.

The [ready mobile](ready-mobile.png), [locked mobile](locked-mobile.png) and
[long/empty desktop](long-empty-desktop.png) captures were visually inspected.
Expiry and rollback timing are proven by the PostgreSQL/API tests; this browser
suite verifies the unchanged user flow around those endpoints.

## Reproduction and limits

Reproduce from the repository root with an approved disposable `DATABASE_URL`:

```bash
cargo fmt --all -- --check
cargo test -p golf-api --features database-tests --test lifecycle_expiry
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build -p golf-api --bin golf-api
cargo test --workspace --all-targets --features database-tests -- --test-threads=2
cargo run -p golf-api --bin migrate
cargo run -p golf-api --bin seed
```

Tests use synthetic data in disposable PostgreSQL 17 container
`golf-a2-20261014-pg`, local port 55449. SQLx creates isolated test databases.
Chrome uses a separately migrated/seeded `golf_a2_browser` database so the final
seed check cannot replace its credentials. No hosted infrastructure is involved.
Local command logs are
`/tmp/golf-a2-{before,focused,backend,clippy,build,database,database-retry,retained-retry,migrate,seed,browser}.log`.

The retained browser suite runs from `frontend/`, with the repaired API on 3000
and Vite on 5173 against that separate fresh seed:

```bash
GOLF_LIFECYCLE_BROWSER=1 npx playwright test --config playwright.lifecycle.config.ts roundLifecycle.browser.ts --output=/tmp/golf-a2-browser-results
```

No frontend code or API shape changed, so the frontend unit/type/lint/build ladder
is not repeated. Physical-device, hosted and production-load acceptance remain
separate. The ignored manual performance probes are outside this expiry repair.
They are one projection benchmark and three existing PostgreSQL profiling probes.
The changed production files remain below the 400-line repository limit (82 and
352 nonblank/noncomment production lines). The additional runtime work is one
active-session query per successful lifecycle attempt; latency was not measured.
