# Latest explanation

## Smaller lifecycle error representation

`OpenRoundError::NotReady` still owns the complete `PairingValidation`, but the
payload now lives behind one `Box`. This reduces the error enum's inline size and
removes strict Clippy's three `result_large_err` findings at the opening helper
and its two public repository entry points.

```rust
#[error("round is not ready to open")]
NotReady(Box<PairingValidation>),
```

The opening path moves the already-built validation into the box only when
readiness fails. No issue is transformed or dropped. Existing pattern matching
continues through dereference coercion, including the specialized
`ReadinessIssueCode::RoundNotDraft` branch and its established public `conflict`
response.

## Preserved boundaries

This is a representation-only repair. Round locking, authorization, readiness
timing, snapshot calculation and insertion, transaction commit/rollback, API
status and body mapping, and post-commit SSE publication are unchanged. The lint
was fixed at the large variant itself; no allow attribute was added and the
entire error was not boxed.

Owner review found no implementation, lifecycle, authorization, transaction, or
error-mapping regression. It corrected only documentation wording so the
internal `RoundNotDraft` readiness issue is not mistaken for a new public error
code.

## Validation

The final ladder passed Rust formatting, 82 standard tests, strict all-target
and all-feature Clippy with `-D warnings`, 13 focused PostgreSQL lifecycle tests,
and the complete PostgreSQL-enabled workspace suite with 222 tests. Diff checks
and production file-size checks passed; the two changed Rust modules remain 77
and 112 physical lines.

The first focused database invocation omitted `DATABASE_URL` and timed out while
trying SQLx's default setup database. The correctly configured rerun against the
disposable PostgreSQL 17 instance on port 55432 passed 13/13, and the full suite
then passed 222/222. Frontend and browser checks were deliberately skipped
because neither the API contract nor user-facing behavior changed.

## Roadmap order

The validation baseline is restored. The next Phase 7 boundary is open-round
provisional tournament contributions with explicit holes-played progress.
Additional play modes remain deferred until the remaining roadmap, optimization,
and security review are complete.
