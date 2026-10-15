# A1p — direct round-creation session expiry

Base revision: `e676e55992144ec1accd48e5ca1955d57f302687`.
Scope: `POST /api/tournaments/{tournament_id}/rounds`, its authorized repository
transaction, focused PostgreSQL/API regressions and affected documentation.
This repairs W1 in the [legacy-write assessment](../legacy-write-inventory-2026-10-15/README.md#w1--direct-round-creation-incomplete-p2).
Pairing replacement (W2), course configuration (W3), provisional preflights and
retained helpers remain separate. No UI, provider, schema or dependency changes.

## Transaction boundary

[create_authorized](../../../backend/src/repositories/rounds.rs#L137) retains its
existing active session/user and exact-admin membership share locks. After the
round INSERT/RETURNING and its immediate triggers, it calls the existing
[auth::lock_active_session](../../../backend/src/repositories/rounds.rs#L170)
for the same session UUID immediately before commit.
The query uses database `clock_timestamp()`, catching natural expiry during the
membership wait or subsequent insert work without a new lock order.

Expiry returns the existing `401 unauthenticated` / `no-store` error. Dropping
the transaction rolls back the round and any conditional Fantasy window, with
no handler notification. A valid creation still returns `201` and publishes one
matching round event after commit. Existing admission/CSRF, preflight, body and
semantic validation order, parent/domain guards and response fields are
preserved. There is no mutation retry or authority cache. This final check does
not freeze time during COMMIT, deferred constraints or network delivery.

## Regression evidence

Both new tests ran against unchanged production code at the base revision.
After observing their intended wait and allowing database-clock expiry, each
returned **201 instead of 401**. The
[captured baseline](baseline-regression.txt) contains both assertion failures.
The production file had no diff when that run completed.

The [new API tests](../../../backend/tests/round_creation_authorization/expiry.rs#L1)
use the existing round-creation fixture, CSRF request builder and body gate:

- Membership contention starts only after preflight finishes and the request
  pauses on its body. An unchanged admin membership is then held `FOR UPDATE`.
  The test observes the final transaction's membership query blocked behind its
  exact blocker PID, waits for the database clock to pass expiry, then releases.
- Late contention uses an isolated test-only `AFTER INSERT` trigger, ordered
  after `fantasy_round_created`. Inside the transaction it verifies the pending
  draft round and its Fantasy child already exist, then waits on an advisory
  lock. Observing the INSERT blocked there proves rejection after pending writes.
- Each rejection must return 401 with `unauthenticated` and `no-store`, preserve
  the complete stored round/window, game, tournament and overall-guard snapshot,
  and emit no event while blocked or after rejection. Session rows are excluded
  from the snapshot because the test deliberately changes their expiry.
- A fresh valid session traverses the same gate for each case, returns 201,
  creates exactly one draft round and matching child, preserves parent/game
  facts, and publishes exactly one matching event. It reuses the round number
  after rejection, also exercising successful creation after rollback.

The [focused result](focused-results.txt) passes all six round-creation tests:
two new expiry cases, each including its valid control, plus four retained
authorization/CSRF/preflight/commit cases. The temporary trigger is installed
only in each SQLx-owned disposable test database; no published migration changes.

## Validation

| Check | Result |
| --- | --- |
| Original-code regressions | Both failed as expected: 201 instead of 401 |
| Focused round-creation suite | 6 passed; covers two expired and two valid gated requests |
| Rust formatting | Passed |
| Full backend all-target tests | 264 passed, zero failures; one existing manual probe ignored |
| Strict Clippy, all targets/features | Passed; three existing vendored SQLx warnings |
| Full PostgreSQL-feature ladder, two test threads | 948 passed, zero failures; five existing manual probes ignored |
| Migrate and development seed | Passed; 39 successful migrations, eight players and five rounds seeded |
| Independent authorization/test review | No actionable findings |
| Documentation, source size and scoped diff | Passed; production file is 193 total lines |

The [retained validation summary](validation-results.txt) records per-target
database results, aggregate counts, ignored probes and command outcomes. All six
round-creation cases also pass inside the full PostgreSQL ladder. Independent
read-only implementation/test and documentation/next-plan reviews reported no
actionable findings. No source change followed those reviews or the full run.

The initial restricted backend run failed eight existing provider-unit tests at
their loopback-listener bind with `PermissionDenied` / `Operation not permitted`.
That environment prevents their local HTTP doubles. The complete rerun with
local socket permission passed; no provider source or test changed, and no
external provider was called.

Reproduce with an approved disposable `DATABASE_URL` set privately:

```bash
cargo test --locked -p golf-api --features database-tests --test round_creation_authorization -- --test-threads=2
cargo fmt --all -- --check
cargo test --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-targets --features database-tests -- --test-threads=2
cargo run --locked -p golf-api --bin migrate
cargo run --locked -p golf-api --bin seed
```

The local fixture was PostgreSQL 17.10 from rootless Podman `postgres:17-alpine`,
container `golf-a1p-20261015-pg`, bound only to loopback port 55460. The owned
container and both private environment files were removed after validation.
Logs stay local; credentials and database URLs are not retained in this report.
The full PostgreSQL count includes ordinary backend tests and is not additive.

Frontend/browser ladders are not selected: the
[source caller inventory](../legacy-write-inventory-2026-10-15/README.md#bounded-next-candidate)
finds no production UI consumer for this direct POST. Existing/new-account UI
creation uses the separate aggregate APIs, which are unchanged and excluded.
There is no user-facing UI, layout or interaction change. No hosted environment,
external provider, dependency scanner or performance benchmark was exercised.
The local acceptance does not establish hosted readiness or close W2/W3.
