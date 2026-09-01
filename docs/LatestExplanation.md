# Latest explanation

## Tournament start is now explicit

An exact tournament admin can move a tournament from `Kladd` to `Aktiv` in the
management workspace. The action is deliberately separate from round opening:
starting changes only `tournaments.status`; every individual round remains draft
until its own course, tee, flights, teams, and score owners pass round-opening
readiness.

The start request contains the tournament's authoritative `updated_at` and uses
the existing CSRF-protected session. The repository locks rounds in deterministic
UUID order, revalidates the active session and exact
`tournament_memberships.role = 'admin'` row, locks the tournament, and then
checks the complete numbered draft plan plus one effectively active entrant. A
global account role never grants authority in another tournament.

```rust
let result = tournaments::start_authorized(
    &state.pool,
    session_id,
    tournament_id,
    expected_tournament_updated_at,
).await?;

if result.changed {
    state.notify("tournament", tournament_id);
}
```

Successful changed transactions emit one payload-free tournament invalidation
only after commit. Already-active retries return the same authoritative
tournament without changing its timestamp or emitting an event. Stale,
not-ready, invalid-state, unauthenticated, cross-tournament, and missing-resource
requests cannot mutate state or publish SSE.

## Database and upgrade guarantees

Migration 0015 guards `draft -> active` with transaction-local tournament and
actor context, exact membership, complete round-plan, and active-entrant checks.
It also rejects every new tournament inserted with a non-draft status; existing
active tournaments remain valid, and the general repository creator is
draft-only. This closes the internal/direct-SQL creation path as well as the
legacy HTTP status field.
It also makes an active parent tournament a prerequisite for round opening while
preserving the full existing completion, locking, scorecard, snapshot, and
foursomes trigger behavior.

Older deployments could legally open a round while the tournament itself still
said `Kladd`. During upgrade, only those already-underway tournaments are
promoted to `active`; their round states are untouched and the tournament
timestamp is intentionally refreshed. Ordinary draft tournaments remain draft.
The legacy platform creation endpoint can no longer accept a caller-selected
status and always creates a draft tournament.

Tournament roster responses now include the tournament-scoped `player_active`
fact. The frontend combines it with registration status, so a withdrawn or
deactivated sole entrant cannot make the Start action appear ready. Starting
also freezes the counted-round setting; draft course and pairing work remains
available because those facts belong to each round's later opening boundary.

## Validation and review

Focused PostgreSQL coverage proves successful and idempotent starts, both
cross-tournament denial directions, `404`, stale and malformed plans, withdrawn
or deactivated entrants, draft-only inserts, lifecycle rollback/no-SSE behavior,
two concurrent HTTP starts, a start/configuration race, and a real migrations-
0001-through-0014 upgrade. The final ladder passed 236 Rust/PostgreSQL tests and
169 frontend tests, strict Clippy, typecheck, lint, production build, repeated
migration/seed checks, and real Chrome validation at 375px and 1440px. Chrome
confirmed the 44px action, disabled pending state, one request after a duplicate
click, `Aktiv` success state, five preserved `Kladd` rounds, no horizontal
overflow, and no console or network failures.

Owner review found no remaining in-scope lifecycle or authorization defect. It
did confirm one pre-existing privacy issue as the urgent next bounded step: the
legacy global player directory and its unauthenticated reads must be retired in
favor of tournament-scoped roster access before production security sign-off.
