# SQLx PostgreSQL cancellation backport

This directory contains the crates.io `sqlx-postgres` 0.8.6 release, with its
normalized Cargo manifest and original MIT/Apache licenses. The only driver
source change is in `src/transaction.rs`: claim transaction depth after queuing
BEGIN and before awaiting its acknowledgement; undo that depth if BEGIN did
not enter a transaction. The existing drop guard can then queue rollback when
initialization is cancelled, including savepoints.

Original crates.io package SHA-256:
`db58fcd5a53cf07c184b154801ff91347e4c30d17a3562a635ff028ad5deda46`.

Backported from [upstream PR #4394](https://github.com/transact-rs/sqlx/pull/4394),
merged as `f1e94ec` on 2026-09-09. It avoids upgrading the application's SQLx 0.8
API or adding cleanup queries to successful requests. Regression coverage lives
in `backend/tests/transaction_cancellation.rs` and uses an ordinary BEGIN with
a controlled protocol-response gate.

This is third-party dependency source, excluded from workspace membership.
Upstream files retain their organization and lengths; the application's
400-line source limit applies to application-owned code, not this preserved
dependency snapshot. Existing upstream whitespace and compiler warnings are
retained to keep the behavioral backport auditable. Application diff checks
exclude this preserved snapshot. Do not refactor unrelated upstream code here.

Remove this override when adopting a compatible upstream release containing the
fix, after running the cancellation regressions, backend/database ladders and
production build. Never remove it merely because a general test suite passes.
