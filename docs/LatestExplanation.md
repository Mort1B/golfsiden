# Latest explanation

## Normalized round-flight persistence

Phase 5 now has a database foundation for explicit flights without changing any
current API, readiness, scoring, seed, or frontend behavior. Migration 0011 is
additive: it creates no flight rows and leaves every existing team and team
membership unchanged. This is intentional because names, member order, starting
holes, and tee times do not prove a truthful flight or scorekeeper assignment.

Three focused relations keep the responsibilities explicit:

- `flights` owns one named round/tournament grouping plus optional starting hole
  and tee time.
- `flight_memberships` joins exact tournament entrants to a flight and enforces
  at most one flight per player in a round.
- `flight_scorekeepers` stores zero or one designation per flight. Its composite
  foreign key proves the designated player is an exact member of that flight,
  while its player foreign key requires a linked user account.

The designation is separate rather than a nullable column on `flights`. That
avoids a circular flight-to-membership dependency and gives membership deletion
a simple, intentional cascade. Zero scorekeepers must remain representable while
a draft is incomplete; a later readiness boundary will require exactly one
eligible scorekeeper before opening.

## Compact example

The database rejects a designation unless all four identity columns match one
membership:

```sql
INSERT INTO flight_scorekeepers
  (flight_id, round_id, tournament_id, player_id)
VALUES ($1, $2, $3, $4);
```

That row also requires `player_id` to exist in the unique linked-player column on
`users`. It does not yet grant score permission; runtime authorization remains a
separate future change.

## Integrity and concurrency

Composite foreign keys prevent cross-round, cross-tournament, non-entrant, and
nonmember assignments. Round/name and round/player uniqueness prevent duplicate
flight identities. Deleting a membership, flight, round, or tournament removes
only the dependent flight hierarchy through declared cascades. A linked account
cannot be unlinked or deleted while it is the designated scorekeeper.

All three relations reuse the existing `protect_round_pairing()` trigger. Every
insert, update, or delete locks affected parent rounds in UUID order and rejects
a non-draft round. Tests cover both concurrency directions: a flight mutation
that owns the lock completes before opening and is observed by it; opening that
owns the lock first causes the waiting flight mutation to recheck and fail with
`round_pairing_frozen`.

## Preserved invariants

- Tournament-player identity remains independent of both teams and flights.
- Teams remain the current round-specific score owners for shared results;
  flights never own scores.
- Missing designation is draft configuration state, not inferred authority.
- Legacy teams, score attribution, handicap snapshots, lifecycle readiness,
  score access, standings, and SSE behavior are unchanged.

## Validation

- Focused PostgreSQL migration coverage passed 6/6, including a version-10
  upgrade that compares legacy team and membership JSON byte-for-byte.
- Standard workspace tests passed 62/62; the full database-enabled workspace
  passed 172 tests.
- Formatting, all-target/all-feature checking, Clippy with warnings denied, and
  `git diff --check` passed.
- A fresh-target migration binary applied versions 1–11 to an isolated
  PostgreSQL 17 database. Catalog inspection confirmed the three flight tables,
  14 expected constraints, and four expected triggers.
- A previously cached migration binary still embedded versions 1–10 and first
  reported the isolated database as current. Rebuilding with a fresh Cargo target
  embedded migration 11 correctly; clean-build migration evidence is therefore
  the authoritative result.

## Next boundary

Define one transactional admin roster API and consolidated pairing read model,
including an explicit policy for any legacy individual-round grouping teams.
Opening readiness, seed assignments, flight-wide scorekeeper authorization, and
the mobile editor remain separate reviewable steps.
