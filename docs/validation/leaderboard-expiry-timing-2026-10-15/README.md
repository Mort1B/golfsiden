# A1y — leaderboard expiry-test timing

The **P3 test setup timing exposure** is repaired. The score-loading expiry case
now acquires its scores gate before starting the same two-second deadline and
proves that the request reaches that gate while authority is still active.
Production authorization, SQL, concurrency, retries, API and scoring are unchanged.
The precise cause of the retained historical timeout remains unproven.

## Evidence and limits

Baseline: `e92aa70d4ebe63e17c6b75cba07935cea52d2b54`. The original failure in the
[round-lifecycle report](../round-lifecycle-session-2026-10-14/README.md#validation)
records only `wait_blocked` timing out. It captured neither an early response nor
database deadline ordering. No retrospective diagnosis is possible from that
trace alone.

The unchanged 10-test leaderboard module passed three default-concurrency runs
and one two-thread control: [baseline receipts](baseline.json). Default means
`RUST_TEST_THREADS` unset, no `--test-threads` override, eight available/affinity
CPUs. Each receipt records scheduling, command, source SHA-256, durations and host
load averages. Those runs bound non-reproduction on this local environment; they
do not recreate the historical concurrent workload.

Source exposed a separate demonstrable mechanism: the two-second deadline began
before acquiring the gate connection, querying its PID and locking `scores`.
A controlled **2250ms async delay** in that interval establishes the mechanism:

| Case | Result | Evidence |
| --- | --- | --- |
| Original test plus delay | Exact original controlled-wait timeout; target alone fails | [receipt](controlled-original.json), [source patch](controlled-original.patch) |
| Same pre-gate expiry ordering with diagnostics and delay | Gate setup completes after expiry; early HTTP 401, no scores gate observed | [receipt](controlled-before.json), [source patch](controlled-before.patch) |
| Repaired ordering plus the same pre-gate delay | All four routes reach the scores gate while active, expire there, then return 401 | [receipt](controlled-after.json), [source patch](controlled-after.patch) |

The instrumented negative control observed **−317.577ms** remaining after setup
and **−367.747ms** at the early 401. The repaired controlled run spent
2308.427–2319.430ms in setup yet retained **1914.218–1921.820ms** at the actual
scores gate. Release occurred 3.767–13.190ms after expiry. This proves that
pre-gate setup can exhaust the old fixture's deadline and that the repair removes
that exposure. It does **not** prove that this delay occurred historically.

No artificial delay remains in the final test. Three repaired default runs and
the two-thread control each passed all 10 cases: [repaired receipts](repaired.json).
Across their 16 board requests, the observed scores gate had
1907.614–1984.844ms remaining; release followed expiry by 2.371–17.833ms. Every
response retained `401`, `private, no-store` and `unauthenticated` assertions.
Neither an early 401 nor a late successful payload can pass this case.

## Final test contract

- Acquire the `scores` access-exclusive gate before arming expiry; keep the
  two-second session window and existing five-second waits.
- Race request completion against gate observation. Any early response fails
  setup, with a sanitized status/deadline diagnostic.
- Require an ungranted `scores` `AccessShareLock` in the same database, blocked
  by the owned gate PID. Check positive remaining lifetime with PostgreSQL
  `clock_timestamp()` at observation.
- Wait for natural database-clock expiry while retaining the gate, release it,
  and apply all existing denial/header/error assertions to all four routes.

Diagnostics contain only board ordinals, relative times and status codes. The
`query_wait_ms` field is **query age** from `pg_stat_activity.query_start` to
observation; it includes work before lock waiting and is not an isolated lock
duration or performance measurement. The two-thread control changes scheduling
between test cases; the HTTP/database race inside each case remains concurrent.
No setup retry or timeout increase was added. Extreme post-arm starvation can
still fail the test, now with an explicit precondition failure rather than
accepting unrelated early rejection as expiry-race coverage.

## Validation

| Check | Result |
| --- | --- |
| Unchanged module, default ×3 plus two-thread control | 10 passed per run |
| Controlled original / instrumented negative | Expected target-only failures, with timeout / early-expired-401 evidence |
| Repaired controlled setup delay | 1 passed, all four routes |
| Repaired module, default ×3 plus two-thread control | 10 passed per run |
| Formatting | Passed |
| Full ordinary Rust tests | 264 passed, 1 manual probe ignored |
| All-feature Clippy with warnings denied | Passed |
| Full PostgreSQL-feature ladder, default concurrency | Blocked by separate unchanged lifecycle case: 477 passed, 1 failed, 1 ignored; selected leaderboard case passed |
| Full PostgreSQL-feature ladder, explicit two-thread control | 953 passed, 5 manual probes ignored |
| Migration / development seed ladder | Passed |
| Independent read-only review | No remaining findings; scoped validation limitation recorded |
| Final offline receipt audit | All seven receipts verified, including explicit blocked-default classification |

The initial affected ladder is retained in [ladder.json](ladder.json). Its
default PostgreSQL run stopped at the unchanged
`lifecycle_expiry::scenarios::match_late_expiry_rolls_back_all_actions`, after the
selected leaderboard case had passed. The private raw output was deleted with
the owned fixture; this receipt retains the failure's name/counts, but no panic
location or cause. No lifecycle source was changed, and no cause or severity is
assigned from that limited evidence. It is a separate next investigation.

The later [A1z investigation](../lifecycle-expiry-investigation-2026-10-15/README.md)
bounds non-reproduction with unchanged default-module/exact-case and two-thread
checks. It selects no repair and leaves this original full-run failure explicit.

The [continuation](continuation.json) runs the full database suite with
`--no-fail-fast -- --test-threads=2`, followed by migration and seed. Its final
PostgreSQL count includes ordinary tests and must not be added to the ordinary
Rust total or the failed partial run. A two-thread success cannot establish a
passing full default-concurrency ladder. The focused default and two-thread
results above remain separately identified. Frontend/browser and release-build
checks are not affected by this backend-test-only repair and were not repeated.
No schema or dependency changed; architecture and deployment procedures remain
unchanged.

The read-only reviewer confirmed the race is stronger, all denial assertions
remain, controlled patches match their recorded source hashes, and the final
source equals the repaired control with its single artificial delay removed.
Review found a P3 evidence-runner gap: exit code alone did not require the target
test or complete timing scenario. [evidence.py](evidence.py) now requires exact
counts and target outcomes, all four timing sequences, expected negative
controls, default-concurrency full-ladder execution and owned-fixture cleanup.
The reviewer rechecked that repair with no further findings.
The receipt checker explicitly labels the retained default-ladder failure as
blocked; it requires the separate continuation's complete counts and target
execution before accepting its success.
Ten [offline invalid-receipt controls](postcondition-controls.json) were rejected,
including zero tests, missing target/board evidence, expiry before gate,
premature release, wrong negative-control status/deadline, missing cleanup and
relabeling the blocked default ladder as successful.

## Reproduction and fixture ownership

[validate.py](validate.py) runs `baseline`, `controlled-original`,
`controlled-before`, `controlled-after`, `repaired`, `ladder` or `continuation`.
Baseline and control source patches are relative to the base commit above; replay them only
in an isolated checkout at that revision with `git apply --unidiff-zero` (the
patches omit context to avoid whitespace-only context lines). The driver does not modify source or
apply patches. Remove the controlled delay before final `repaired` / `ladder`
runs. Never overwrite retained baseline receipts with a different source while
calling it the original baseline.

The runner reuses the reviewed [CID-owned fixture helper](../sqlx-verify-ca-2026-10-15/fixtures.py):
cached PostgreSQL 17 Alpine, random loopback port, private random credentials and
tmpfs database data. Each phase owns and removes only its own container; private
raw logs and credentials stay in temporary files. Receipts record the image ID,
actual PostgreSQL 17.10 version and cleanup result. No external database or
hosted environment is used. All seven retained phases confirm owned-container
and private-file removal.

After all phases finish, audit the retained receipts without starting services:

```sh
python3 docs/validation/leaderboard-expiry-timing-2026-10-15/evidence.py
```

The final [audit output](receipt-audit.txt) verifies every retained phase and the
final source fingerprints, including cleanup. It explicitly reports the default
ladder as blocked and the complete two-thread continuation as passed.

This step establishes local test-fixture correctness and bounded timing evidence.
It does not establish general load capacity, universal test-flake elimination,
hosted security readiness or a new production performance finding.
