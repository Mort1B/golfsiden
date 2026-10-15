# A1z — bounded lifecycle investigation and Sunday release priorities

**Outcome: no failure reproduced and no repair justified.** Two unchanged
default-concurrency module runs, the exact previously failing case and the
two-thread module control all pass on owned PostgreSQL. No application, test,
schema, dependency or deployment source changes. The historical failure remains
unexplained; it is not relabeled as a passing full default-concurrency ladder.

The user set Sunday **18 October 2026** as the production-ish target, asked for
5–8 iterations and confirmed the supplied **single-host Docker Compose** model.
This investigation is iteration 1 of six planned iterations, with two contingency
slots for confirmed launch blockers. The remaining path prioritizes deployable
images, production integration, essential user flows and recovery. Optional work
is deferred; no deployment is performed by this step.

## Retained failure and fresh evidence

Baseline: `351f63667bee48aeb8abdb9b970b5975863f94e0`. The preceding
[default-concurrency ladder](../leaderboard-expiry-timing-2026-10-15/ladder.json)
stopped at `lifecycle_expiry::scenarios::match_late_expiry_rolls_back_all_actions`:
477 passed, 1 failed, 1 ignored. Its selected leaderboard case passed. Only the
failed name/counts survived private-log cleanup; there is no retained panic
location, action, response or deadline ordering from that failure.

[Fresh receipts](baseline.json), with `RUST_TEST_THREADS` unset:

| Run | Scheduling | Result | Seconds |
| --- | --- | --- | --- |
| Lifecycle module 1 | Default, eight available/affinity CPUs | 6 passed | 16.561 |
| Lifecycle module 2 | Same default | 6 passed | 16.409 |
| Exact match late-expiry case | Default; exact filter | 1 passed | 15.963 |
| Lifecycle module control | Explicit `--test-threads=2` | 6 passed | 47.954 |

Every run executes the named target; no tests are ignored. Source fingerprints,
commands, load averages, exact totals and target outcomes are retained. No panic,
early-response code, controlled-wait timeout, late-snapshot precondition failure
or rollback assertion failure was observed. The
[postconditions](postconditions.json) verify these outcomes, source hashes,
scheduling and cleanup.

These runs bound non-reproduction in this local workload. They do not recreate
the accumulated full-suite workload or establish what failed historically.
The unchanged test does not emit database-clock gate/response timestamps, so no
new ordering measurements or active-at-observation margin are claimed. No
synthetic delay was introduced to manufacture a diagnosis.

## Source review and decision

The existing late gate is after real lifecycle work. Its AFTER-status trigger
requires four player snapshots and the tournament handicap lock before acquiring
the controlled advisory lock; foursomes also requires both team snapshots.
The opening path inserts those snapshots before changing status. Complete/lock
use the same trigger after their inner transition. The authorized production
wrappers recheck database-clock session expiry after inner work and before commit.

For open, complete and lock, the existing case preserves:

- Observed controlled wait, natural expiry, private 401 and unauthenticated code.
- Exact stored-state rollback and no SSE after rejected transitions.
- A valid-session traversal of the same gate, expected committed status,
  preserved historical score/snapshot/match data and exactly one round event.

Independent read-only review found no actionable match-specific source defect.
The five-second session is armed before gate acquisition and the generic wait
does not check remaining lifetime at observation. Those are diagnostic limits,
not proof of the old failure's cause. There is no evidence here to select a
timeout increase, retry, ignored test, production change or new fixture repair.

The bounded investigation therefore stops with an explicit validation limitation.
It does not create another automatic test-maintenance iteration. Revisit this
case if a retained failure or critical-flow rehearsal supplies actionable
evidence. A reproduced authorization, scoring, persistence or recovery defect
remains a launch blocker regardless of the deadline.

## Sunday priorities and remaining gates

This is a readiness inventory, not a new severity-rated application defect list.
The bounded source review established no new P0–P2 product defect. The known
default-concurrency failure remains unclassified because its cause is unknown.

| Priority | Current gap | Acceptance before release sign-off |
| --- | --- | --- |
| 1 | Current production images/toolchain are unverified | Build all three supplied Dockerfiles at one immutable candidate. The API image specifies Rust 1.88; recent host builds used 1.99. This is an evidence gap, not a demonstrated incompatibility. |
| 2 | Current Compose production integration is unverified | Fresh schema 39, real runtime grants, production API startup, health/readiness, secure-cookie browser access and SSE through Caddy. Direct-API local browser tests do not establish the proxy path. |
| 3 | Current application restore is unproved | Actual backup/checksum/restore into a separate fresh Compose project with data parity, real grants, readiness, two-account login and representative results. The A5 synthetic-table probe shimmed stop/grants. |
| 4 | Actual host acceptance belongs to the operator | New install versus upgrade, domain/DNS, ports, private secrets, storage, off-host backups and an actual playing-phone workflow. Public HTTPS/readiness must be checked on that host. |

Sources: [production topology](../../../compose.production.yml),
[API Dockerfile](../../../backend/Dockerfile.production),
[host build limits](../sqlx-verify-ca-2026-10-15/README.md#reproduction-and-limits),
[browser transport limits](../leaderboard-browser-maintenance-2026-10-15/README.md),
[restore probe scope](../restore-checksum-2026-10-15/README.md#actual-disposable-postgresql-acceptance)
and [operator guide](../../deployment_guide.md).

The [active queue](../../PLANS.md) defines the next bounded image/configuration
step and the six-iteration path. P0–P2 correctness/security, an unusable essential
flow, failed image/startup/schema/grants or failed recovery blocks sign-off.
Optional features, unmeasured optimizations, broad polish and additional device
matrix work do not enter that path without evidence of a required-flow blocker.
Unverified physical-device coverage remains explicit; it is not reported done.

## Validation, review and ownership

[validate.py](validate.py) runs the exact commands in the receipt:

```sh
python3 docs/validation/lifecycle-expiry-investigation-2026-10-15/validate.py baseline
```

It reuses the [CID-owned disposable fixture](../sqlx-verify-ca-2026-10-15/fixtures.py):
PostgreSQL 17.10 Alpine, random loopback port, tmpfs data and private random
credentials. Only the recorded owned container is removed. Raw output remains
temporary; published failure diagnostics are restricted to source locations,
static test names, error codes and assertion categories, with no JSON/payloads
or session identifiers. The database and private temporary files are removed.

Fresh full Rust/frontend/browser/image/backup ladders were not repeated: no
runtime, existing test, schema, dependency or deployment source changed.
The previous [full two-thread PostgreSQL/Rust acceptance](../leaderboard-expiry-timing-2026-10-15/README.md#validation)
and [frontend/browser acceptance](../leaderboard-browser-maintenance-2026-10-15/README.md)
remain scoped evidence, not new runs or current-image/public-host acceptance.
Python syntax, receipt counts/target, scheduling, source fingerprints, local
documentation links and diff checks are verified in this iteration.

Independent reviews covered the lifecycle authority/test boundary and the
deployment-priority inventory. They executed no tests or hosted operations.
The final production readiness verdict belongs to the release-freeze iteration;
current image/proxy/recovery gaps remain open.
