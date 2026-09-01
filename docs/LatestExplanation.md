# Latest explanation

## PostgreSQL now owns the final-score embargo clock

Phase 7A adds the persistence boundary needed before hiding the final nine from
non-admin reads. Only the round whose `round_number` equals its tournament's
configured round count can receive `final_scores_hidden_until`. When insertion
of the last required confirmation makes that round ready, PostgreSQL records one
deadline from its own clock:

```sql
SET final_scores_hidden_until = workflow_time + INTERVAL '24 hours'
WHERE final_scores_hidden_until IS NULL
  AND round_scorecards_ready(id);
```

The same readiness function already understands individual snapshot owners,
scramble teams, and foursomes teams with preserved handicap snapshots. An
intermediate or duplicate confirmation does nothing, so retries cannot extend
the embargo or change the round timestamp.

## Corrections reset only an unexpired embargo

A real score correction already removes that owner's current confirmation. The
new confirmation-delete trigger clears the deadline only when it is strictly in
the future. Once every corrected card is complete and confirmed again, the last
confirmation starts a fresh full 24 hours. At or after expiry, correction keeps
the expired timestamp because the results were already revealed. Completion and
locking also preserve the exact deadline.

Schema-16 upgrades reconstruct ready final-round deadlines from the latest
required stored `confirmed_at` plus 24 hours. This preserves the historical
trusted clock rather than starting a new window at migration time.

## Final-round identity cannot drift after start

The embargo depends on a stable definition of “final.” PostgreSQL therefore now
freezes both the tournament round count and child round numbers across the start
boundary. A round renumber locks its parent after its own row, matching the
existing lifecycle order. Concurrent start and renumber operations either leave
a valid changed draft plan that cannot start or commit start and reject the
renumber; they cannot deadlock or silently reclassify the final round.

## Scope and validation

Focused tests cover database-time bounds, individual/scramble/foursomes
readiness, non-final exclusion, correction and reconfirmation, exact expiry,
duplicate stability, completion and locking, direct-write rejection, schema-16
backfill, cascade deletion, and bounded lifecycle races. The focused suites,
ten repeated concurrency runs, full Rust and serialized PostgreSQL ladders,
strict Clippy, clean/schema-16 migration, and idempotent seed all passed.

This step deliberately changes no HTTP or frontend contract. The deadline is
not yet a claim that scores are hidden. Phase 7B must apply one role-aware policy
before round and tournament calculations and direct scorecard responses, then
add trusted expiry refetching in the client.
