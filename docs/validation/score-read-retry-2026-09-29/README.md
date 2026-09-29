# SCORE-READ-1: bounded scoring-card read recovery

Date: 2026-09-29. Baseline: `a108200`.

**READY** for this bounded application fix. Hosted and physical-device acceptance
remain user-owned; the broader assessment is not a prerequisite for testing.

## Reproduction and repair

The earlier functional checks recorded one scoring-card GET returning HTTP 500
with PostgreSQL `40001`. Its exact original overlap was not captured. This step
deterministically reproduces the same failure class: establish the read's snapshot,
hold its session or membership authority lock, commit a conflicting row update,
and observe the real router response.

Before the repair, logout and membership-removal conflicts returned 500 rather
than 401/403. Harmless membership updates also returned 500 instead of the saved
card. Tests synchronize using the blocking transaction's backend PID and
`pg_blocking_pids`; they do not rely on a timing sleep to induce the conflict.

`get_scoring_authenticated` now discards the failed transaction and retries the
entire read once, only for SQLSTATE `40001`. Each attempt re-reads the round,
session, membership and eligible owner before assembling the card. Existing
isolation and share locks remain intact. A second conflict maps to the existing
`503 service_unavailable` envelope. Score mutations and other read paths are not
retried. No API fields, schema, scoring rules or frontend source changed.

The scoring route adds `private, no-store` to responses without an existing cache
header; established 401/403 `no-store` responses remain unchanged. Denied or
unavailable reads contain no card data.

## Regression evidence

Six new PostgreSQL/API cases cover:

- Session revocation: 401, no score payload.
- Membership removal: 403, no score payload.
- Harmless membership change: identical individual card, 200.
- Harmless membership change: identical team card, 200.
- Repeated serialization failure: exactly two attempts, non-cacheable 503.
- Unrelated database error: exactly one attempt; existing internal-error mapping.

Every case compares complete score, audit and confirmation rows and checks that
the read emits no score event. Exhaustion tests use a failure-injecting view and
nontransactional attempt counter only in their disposable test schema. They prove
the bound even when each failed transaction rolls back.

All six failed against the original implementation. Four failed on the concrete
500-versus-401/403/200 mismatch, exhaustion failed on 500-versus-503, and the
unrelated-error case exposed the missing error-response cache header. The focused
cases passed after the repair. The event test harness was corrected to keep its
publisher alive when checking for no events, rather than confusing a closed
channel with a published event.

The first full database run found an existing test requiring the precise denial
header `no-store`; the middleware was adjusted to retain existing headers. The
final focused scorecard and visibility group passed all 43 cases. This was a
compatibility correction, not weakened authorization or relaxed assertions.

## Browser evidence

The retained [browser script](browser.mjs) uses installed headless Chrome, the
actual frontend and API, and real PostgreSQL lock contention. It forwards the
original scoring request and response unchanged through a route handler while
holding the database gate; it does not fabricate a successful card or denial.
Only synthetic local seed accounts are used.

All three final cases passed: a valid saved score recovered with 200 at 390px;
concurrent logout returned 401 and removed the scoring UI at 320px; a membership
downgrade returned 403 and removed editing controls at 1280px. The downgraded
viewer retained its normal projected read access. No unexpected console,
JavaScript or network errors, or horizontal overflow were observed.

Retained evidence: [assertions](browser-checks.json),
[authorized phone](valid-390.png), [logged-out phone](logout-320.png), and
[read-only desktop](membership-1280.png). Screenshots were visually inspected.

Harness iterations increased the observation deadline to cover the deliberately
held transaction, made the score-edit setup repeatable, and corrected viewer
assertions to match the existing projected-read behavior. No application UI
change was needed.

## Validation and review

All services are task-owned: rootless Podman container `golf-score-read-20260929`,
PostgreSQL 17 on loopback 55442, fresh migrated/seeded `golf_browser`, and separate
SQLx test databases. Existing services, prior test data and gg26.no are untouched.
Local logs are under `/tmp/golf-score-read-20260929`. The task API and Vite
processes were stopped after browser checks; the PostgreSQL container was stopped
after the full suite and retained with its data.

| Command/check | Result |
| --- | --- |
| `cargo test -p golf-api --test scorecards --features database-tests read_retry -- --test-threads=2` | 6 failing-first, then 6 passing |
| `cargo test -p golf-api --test scorecards --test embargo_read_projections --features database-tests -- --test-threads=2` | 43 passed after header compatibility correction |
| Ordinary backend tests | 215 passed |
| Formatting, strict Clippy | Passed |
| Full PostgreSQL-enabled ladder | 621 passed, including the 215 ordinary tests; 3 pre-existing measurement cases ignored |
| Fresh migration, development seed, API build | Passed |
| Frontend tests | 806 passed |
| Frontend typecheck, lint and build | Passed |
| `node docs/validation/score-read-retry-2026-09-29/browser.mjs` | 3 real-contention scenarios passed |

The full database command used `--features database-tests -- --test-threads=2`.
Three pre-existing match-list performance measurements were not run: their
explicit ignore reason is "exclusive disposable PostgreSQL with
pg_stat_statements required". That separate measurement environment was not
configured for this scoring-read repair. Vendored SQLx warnings remained during
builds; strict workspace Clippy still passed.

Independent read-only review found no blocking correctness/security issue in the
whole-transaction retry, error mapping or regression design. Final review also
checked the compatible cache-header handling and retained real-browser harness.
Review noted
that the new tests do not specifically cancel the second attempt or force its
reuse of a one-connection pool; existing transaction-cancellation tests exercise
the underlying vendored SQLx cleanup boundary. No broader cleanup or expiry-policy
change was made.

## Limits and handover

This fixes the reproduced serialization-conflict path for the ordinary scoring
card endpoint only. Repeated contention can still produce the deliberate 503;
unrelated failures are not hidden or retried. It does not establish that every
possible scorecard error or the exact historical overlap has been eliminated.

Hosted configuration, physical phones, other endpoint concurrency, callback
lifetime concerns and the broader security assessment remain outside this step.
No migration or deployment configuration change is needed. Continue with the
[testing checklist](../../testing_checklist.md) after deploying the API normally.
