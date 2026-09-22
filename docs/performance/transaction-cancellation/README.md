# Cancellation-safe PostgreSQL transaction initialization

**READY.** The scoped repair is reproduced, reviewed and validated.

The scoped repair backports [SQLx PR #4394](https://github.com/transact-rs/sqlx/pull/4394)
into the pinned 0.8.6 PostgreSQL driver. Only transaction initialization changes;
repository SQL, authorization, isolation levels, APIs and frontend behavior remain
unchanged. The production image uses the same local driver override.

## Controlled diagnosis

The retained test proxy forwards SQL unchanged and pauses the PostgreSQL
`CommandComplete` acknowledgement for an ordinary `BEGIN`. A notification proves
that the server accepted BEGIN before the test cancels the Rust future. There is
no `pg_sleep` inside BEGIN and no timing guess deciding when cancellation occurs.
Five-second deadlines fail a stuck fixture; they do not choose the race order.
A one-connection pool and PostgreSQL backend PID equality prove actual reuse.

Against the original driver, the next repeatable-read transaction fails with
SQLSTATE 25001. Cancelling a nested start also rolls back the outer transaction:
the independent database read sees only the second marker, losing the first.
Both regressions pass with the repair. Three other controls pass on both drivers:
ordinary transaction-drop cleanup, failed/custom begin modes, and successful reuse.

A separate test runs the real Axum router on a TCP listener. It holds the same
ordinary BEGIN acknowledgement, closes the client socket and observes the request
future being dropped before releasing the reply. The original driver queues zero
rollbacks; the repaired driver queues one. Subsequent admin and viewer requests
reuse the same connection successfully with private/no-store responses. Removing
the viewer's membership yields 403; expiring the administrator session yields 401.
No authority context survives a request.

The historical desktop failure cannot be replayed exactly because its original
abort timing was not retained. The controlled server test establishes a concrete
HTTP-disconnect → request-drop → interrupted-BEGIN → poisoned-reuse chain, and
fresh Chrome tests exercise the actual return/refetch flows. Source tracing also
confirms that the match API forwards AbortSignal to fetch, Vite closes an unfinished
upstream request when the client disconnects, and Hyper/Axum drop the pending handler
on connection termination. This is evidence for the mechanism, not a claim that
we recovered the historical browser's exact sequence.

## Repair boundary

The driver claims depth after queuing BEGIN and before awaiting readiness. Its
existing drop guard can now queue rollback in the same protocol buffer, after
BEGIN. For nested initialization it rolls back the new savepoint, preserving the
outer transaction. If BEGIN completes without entering a transaction, the driver
undoes the provisional depth. Successful isolation/read-only modes and their reset
between requests remain intact.

The local dependency contains the normalized crates.io manifest, licenses and
original source. Only `src/transaction.rs` differs; `PATCH.md` records provenance
and removal criteria. The driver is excluded from application workspace membership.
The lockfile changes only the driver's source resolution. There is no schema
change, blanket pool-reset hook, task spawning, cancellation suppression, retry
that hides failures, or weakened repeatable-read setting.

## Reproduction and overhead

Use an exclusive disposable PostgreSQL 17 server and configure `DATABASE_URL`
privately. Run from the repository root:

```sh
cargo test -p golf-api --features database-tests --test transaction_cancellation -- --nocapture
cargo test -p golf-api --features database-tests --test singles_match http_disconnect_during_begin -- --nocapture
```

The original-version control uses a command-local Cargo
`patch.crates-io.sqlx-postgres.path` override to an untouched extracted 0.8.6 crate.
It does not change application source or the committed dependency override.

A protocol audit runs 100 ordinary begin/query/commit cycles. Both versions retain
one backend PID and emit exactly 100 BEGIN, 100 COMMIT and zero ROLLBACK commands.
Thus the repair adds zero queries and zero connections on this successful path.
An interrupted ordinary begin adds the one required rollback. No throughput,
production latency or remote-database speedup is claimed from this test proxy.

## Validation

The [validation record](validation.json) records passing formatting, 208 ordinary
tests, and 593 tests with the PostgreSQL feature enabled (the same 208 ordinary
tests plus 385 database integration tests). All-target/all-feature Clippy with
warnings denied, migrate, seed and the production container build pass. Three
unrelated release measurement tests remain intentionally ignored. Preserved
third-party driver warnings are reported separately from application Clippy.
The original controls retain [ordinary/savepoint failures](original-controls.txt)
and the [HTTP-disconnect failure](original-http.txt).

All 37 Chrome tests pass: 11 actual-API match/history cases and 26 mocked-API
cancellation, result-visibility and return/loading cases. The actual API is the
production container in development configuration with a one-connection pool.
History returns, hidden/released results, SSE and persisted pageshow pass at
320/390/1280px. Loading, error, empty, populated, long-content, offline, frozen-return
and overlapping-authority states are covered by the existing browser suites.
Console/network assertions pass; four mobile/desktop screenshots were inspected.
The API log contains no SQLSTATE 25001. Frontend source/contracts/dependencies are
unchanged, so its unit/type/lint/build ladder was not repeated.

Tests cover rollback of writes and transaction-local settings, advisory
lock release, same-connection reuse, nested starts, failed starts, read-only and
repeatable-read modes, and cross-account session/membership enforcement.

The wider operational security review is separate. Physical-device suspension,
Safari and a complete production Caddy deployment are outside this validation.
