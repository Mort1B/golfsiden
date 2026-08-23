# Latest explanation

## Membership-wide flight score authority

Every authenticated player who is linked to an exact flight member will be able
to keep score for every eligible card in that flight. A special designated
scorekeeper is unnecessary and would incorrectly prevent the other flight
members from helping with scoring.

Migration 0011 had already been published with an unused
`flight_scorekeepers` table, so it remains unchanged. Forward migration 0012
drops that table. No production API, repository, seed, or frontend code used it,
and migration 0011 created no designation rows automatically. An upgrade may
discard manually inserted designation rows because they no longer represent a
product fact; all `flights` and `flight_memberships` rows are preserved exactly.

The durable flight model now has two responsibilities:

- `flights` owns the exact round/tournament grouping, name, optional starting
  hole, and optional tee time.
- `flight_memberships` assigns a tournament entrant to at most one flight in a
  round.

Account linkage does not belong in this persistence boundary. When runtime score
authorization is added, it will revalidate the active session and linked player,
then derive the writable score owners from that player's exact stored flight
membership. Tournament admin/scorer overrides remain separate. A shared starting
hole, tee time, flight name, or client-supplied identity never grants authority.

## Compact example

The correction is deliberately a forward migration rather than a rewrite of
published history:

```sql
DROP TABLE flight_scorekeepers;
```

## Preserved integrity

- A flight still belongs to one exact round and tournament.
- A tournament entrant still belongs to at most one flight per round.
- Flights remain independent of teams and never own scores.
- Flight and membership mutations still lock the parent round and fail after it
  opens, including both mutation/opening race orders.
- Deleting a flight, round, or tournament keeps its explicit membership cascade.
- Existing teams, team memberships, scores, handicap snapshots, standings, and
  legacy grouping data remain unchanged.

## Validation

The focused PostgreSQL migration suite passed 7/7 tests. It includes a populated
version-11 upgrade that proves the obsolete designation disappears while every
flight and membership field remains identical, and a version-10 upgrade that
proves migrations 11 and 12 do not infer flights or change legacy teams. Clean-
schema integrity, cascades, draft-only mutation, and both round-opening lock
orders remain covered.

The standard workspace passed 62 tests, and the full PostgreSQL-enabled workspace
passed 173 tests. Formatting, all-target/all-feature checking, Clippy with
warnings denied, and `git diff --check` passed. A clean migration build applied
versions 1–12 to an isolated PostgreSQL 17 database; the catalog contained
exactly `flights` and `flight_memberships` among the flight tables and no
`flight_scorekeepers` relation. One unrelated round-configuration concurrency
test returned its alternate conflict result on the first full run, then passed
both alone and in the complete rerun.

## Next boundary

Add one transactional tournament-admin roster API and consolidated pairing read
model, with an explicit legacy individual-team conversion policy. That API must
not expose a scorekeeper selector. Opening readiness, seed assignments,
membership-wide score authorization, and the mobile roster editor remain later
bounded steps.
