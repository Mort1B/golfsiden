# Latest explanation

## Atomic completion and locking

Round completion is now a separate lifecycle boundary instead of a direct status
update. A repeatable-read validation endpoint reports deterministic progress for
every required scorecard. Individual rounds derive owners from immutable opening
snapshots, while scramble rounds derive owners from the teams frozen at opening.
Every owner needs exactly the configured number of hole scores and a current
confirmation.

Completion and locking take the same round-row lock used by score saves and
confirmations. They re-read readiness inside that transaction, set a
round-specific lifecycle context, perform only `open -> completed` or
`completed -> locked`, and commit before the API emits SSE. A correction that
wins before locking invalidates confirmation and blocks the lock; a lock that
wins first makes waiting ordinary score operations re-read `locked` and fail.

Migration 4 preserves all opening guards, adds per-owner transition backstops,
locks relevant tables during its upgrade preflight, and rejects existing invalid
completed or locked rounds with their identifiers. Lifecycle settings enforce
the expected database integrity path but are not an authorization mechanism;
separate runtime roles remain planned with authentication hardening.

## Compact example

The repository rejects a transition before changing status when current facts
are not ready:

```rust
let validation = validate(facts);
if let Some(blocker) = transition_blocker(&validation, action) {
    return Err(RoundCompletionError::Blocked { action, blocker });
}
```

## Validation

- Formatting, diff checks, and Clippy with all features and warnings denied pass.
- The standard suite passes 20 tests; the PostgreSQL feature suite passes 49.
- Automated migration tests cover valid and invalid version-3 upgrades.
- Clean migration and seed validation retains eight players, five rounds, and
  eight round-specific teams.
