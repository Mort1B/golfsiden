# Recheck session expiry before legacy score commits

M1 closes an expiry gap in legacy numeric score saves and scorecard confirmation.
A request could pass session validation, wait on a tournament-membership lock,
and then commit after the session expired. Both changed and unchanged operations
now recheck the active session immediately before their transaction commits.

For example, a confirmed card contains 4 strokes. A correction to 5 begins while
the session is valid but waits for membership access. If the session expires
before the lock is released, the API now returns `401 unauthenticated`; the score
stays 4 with its original revision, audit rows and confirmation. No SSE event is
published. The same rule applies to first and repeated confirmation requests.

## Implementation and boundaries

`repositories/scorecards/mutations.rs` calls the existing central
`auth::lock_active_session` predicate immediately before `save_authenticated` and
`confirm_authenticated` commit. That predicate uses PostgreSQL `clock_timestamp()`
rather than transaction-start time. The transaction already holds session/user
share locks, so the added checks preserve the established lock order and reuse
revocation and credential-generation rules.

Checks run after the existing mutation helpers, which also return on no-op paths.
A failed check drops the transaction, rolling back any tentative score, audit,
revision or confirmation change. API error mapping and post-commit SSE behavior
are unchanged. Internal actor-based helpers, newer conditional receipts,
four-ball, Stableford and match ledger contracts are unchanged.

This is a final application-level session check before commit, following the
existing conditional-delivery contract. It does not introduce a database-wide
expiry constraint or a new authentication policy. Round ownership, preserved
handicaps, locked-round rejection and administrator-managed teams are unaffected.

## Regression evidence

The new PostgreSQL/API suite covers five operations with an expiring session and
five matching valid-session controls: new score, correction of a confirmed card,
unchanged score on a confirmed card, first confirmation and repeated confirmation.
Each request is observed waiting on the exact membership-lock holder while its
session is still valid. Expiry is verified using PostgreSQL wall-clock time before
releasing that lock. Tests compare complete score, audit and confirmation rows,
including IDs, actor, timestamps and revision, and check the live-event receiver.

Before repair, all five expiry cases returned 200 instead of 401; all five valid
controls passed. After repair, **all ten focused cases passed**. Rejected requests
leave every captured row unchanged and emit no SSE. Valid changed operations emit
one event; valid no-ops preserve all rows and emit none.

Independent read-only review found no source, regression-test or documentation
issues. Completed validation:

- `cargo fmt --all -- --check`: passed.
- `cargo test --workspace --all-targets`: **204 passed**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
- Full PostgreSQL feature-enabled workspace/all-targets suite: **568 passed**
  (including the 204 non-database cases), with two test threads per target.
  Conditional delivery, four-ball, Stableford, match, authorization and lifecycle
  regression targets all passed, including the ten new M1 cases.
- Existing migration and seed binaries passed against fresh disposable
  `golf_m1_validation` on PostgreSQL 17.11: **32 successful migrations**, eight
  seeded players and five rounds confirmed by direct reads.
- `git diff --check`: passed. The only changed production file has 250
  nonblank/noncomment lines, within the repository limit.

The initial unprivileged backend run could not bind the local mock HTTP servers
used by course-provider tests. The complete rerun with local-listener permissions
passed; no test or configuration was weakened.

Evidence logs: `/tmp/m1-red.log`, `/tmp/m1-green.log`, `/tmp/m1-backend.log`,
`/tmp/m1-clippy.log`, `/tmp/m1-database.log`, `/tmp/m1-migrate.log` and
`/tmp/m1-seed.log`. These are disposable local artifacts; committed regression
tests preserve the reproduction.

## Remaining scope

M2 (hide private projections after denied refresh), L1–L3, performance measurement
and the wider security review remain queued independently in [PLANS.md](PLANS.md).
No frontend or migration source changed. Frontend tests and Chrome layout checks
were not rerun because this repair changes transaction authorization only; the
HTTP/status, persistence and event behavior are exercised against PostgreSQL.
Production deployment was not run.

**READY for M1.** The bounded repair, regression proof, full affected validation
ladders and documentation review are complete. Remaining findings above are not
part of this readiness verdict.
