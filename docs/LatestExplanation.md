# Latest iteration — bounded password-work admission

A6 adds immediate admission control around password work. The original helper
limited execution to four blocking workers but allowed additional operations to
wait indefinitely for those slots. A regression test filled four workers plus
four waiters and showed the ninth operation still waiting instead of rejecting.

Hashing, real-user verification and unknown-user dummy verification now share a
process-local limit of eight admitted operations and four blocking-worker slots.
Excess work returns the existing capacity error in one poll. Callers retain their
generic 500 `internal_error` mapping; route rate limiting keeps its separate 429
contract. Argon2 parameters, credential-generation checks, account/session write
ordering, request quotas, dependencies, schema and frontend are unchanged.

Both permits belong to the dispatched blocking closure, including time queued
inside Tokio's pool. Cancelling a waiting request frees admission. Cancelling a
request after dispatch retains both permits until the actual work finishes;
completion, operation errors and panic unwinding release them. This bounds
password-helper admission, not every request stage or process memory. Eight is
a finite policy, not a measured production capacity or latency claim.

Seven focused tests pass, covering the ninth-operation regression, shared entry
points, waiting/active cancellation, pool-queued work, error/panic/closed-limiter
release and later recovery. The database-backed API test confirms known/dummy
login parity and creator-onboarding failure without any table-count change,
followed by successful login/onboarding and unchanged generic wrong-password
behavior. Unit tests touching the process-wide static share a test lock;
scheduling tests use local limiters and gates that open on drop.

Formatting, the 264-test backend suite and strict Clippy passed, with one existing
ignored benchmark and three unchanged vendored SQLx warnings. The first sandboxed
backend run could not bind eight existing provider HTTP fixtures; the full rerun
with socket permission passed. The complete PostgreSQL ladder passed 932 tests
with four existing ignored diagnostics/benchmarks, then migration/seed completed
with all 39 migrations current. Database-enabled counts include ordinary tests.
Its unique local fixture and temporary directory were removed; separate container
inventory confirmed cleanup. No production source changed during validation.
Independent read-only source/test/harness/documentation review has no remaining
findings.

The [A6 report](validation/password-admission-2026-10-15/README.md) contains the
baseline, focused checks and validation limits. No hosted action or production
capacity test was performed. Rebuild the API to adopt the change; no migration or
new environment setting is needed. Compatible development-dependency repairs are
the next candidate in [the plan](PLANS.md); no queued work was started.
