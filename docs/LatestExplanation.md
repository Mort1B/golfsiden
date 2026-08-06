# Latest explanation

## Atomic round opening

Round readiness is a pure domain decision built from repository facts. It checks
the tournament and round states, active entrant assignments, format-specific team
sizes, the course/tee relationship, handicap ratings, and complete hole and
stroke-index ranges. The API exposes the same deterministic result before opening
and returns stable JSON conflicts when an opening attempt is not ready.

Opening repeats those checks inside a transaction after locking the round and
tournament. Exact integer-tenths arithmetic calculates the stored handicap values;
the database receives the handicap index as an exact decimal rather than through a
binary float. Snapshots are inserted before the status changes, and the SSE event
is sent only after commit.

PostgreSQL reinforces the boundary. A transaction-local round identifier permits
snapshot capture and `draft -> open` only within the repository workflow. Status
transitions are forward-only. Pairing, tee, and hole triggers serialize mutations
through the same parent-round lock, while snapshot and scoring configuration
changes are rejected after opening. Deferred restrictive participant references
allow whole-parent cascades without letting direct entrant deletion erase round
history.

## Compact example

The transaction establishes its narrow opening context only after readiness has
passed:

```rust
sqlx::query("SELECT set_config('app.round_opening_id', $1::text, true)")
    .bind(round_id)
    .execute(&mut *transaction)
    .await?;
```

The setting is local to the transaction and disappears on commit or rollback.

## Validation

- `cargo fmt --all -- --check` passed.
- `cargo test --workspace --all-targets` passed 15 unit tests.
- PostgreSQL feature tests passed 29 tests, including snapshot preservation,
  lifecycle bypass attempts, parent deletion, concurrent opening, and mutation
  races.
- Clippy passed for all targets and features with warnings denied.
- A clean PostgreSQL 17.10 database applied both migrations and loaded the seed:
  eight players, five rounds, eight teams, sixteen memberships, and eighteen
  holes.
