# Latest explanation

## Flight-aware readiness and opening

Pairing validation and transactional round opening now share one flight-aware
domain decision. The lifecycle code was split before adding the rules: focused
domain modules own pairing and course configuration validation, while focused
repository modules own fact loading and the locked opening transaction. Every
production lifecycle file is comfortably below the 400-line limit.

An entrant is eligible only when both the tournament entry and global player are
active. Every eligible entrant must be assigned to exactly one nonempty flight;
withdrawn or inactive entrants cannot remain in a flight.

Format-specific rules stay explicit:

- Individual stroke play uses player-owned scores and requires no teams. Any
  remaining individual-round team is a legacy grouping and blocks opening until
  it is converted through the pairing API.
- Team scramble retains round-specific teams as shared score owners. Every
  eligible entrant must belong to one exactly two-player team, and both team
  members must be contained in the same flight. A flight may contain multiple
  complete teams.

Starting hole, tee time, names, and equal schedule values never establish team
containment. Only stored member identities do.

## Additive validation contract

Existing readiness fields and codes remain present. For compatibility,
`team_sizes` still lists stored legacy individual teams even though team missing/
ineligible rules now apply only to scramble. The response adds:

```json
{
  "missing_flight_players": [],
  "ineligible_flight_players": [],
  "flight_sizes": [
    { "flight_id": "uuid", "flight_name": "Flight 1", "player_count": 4 }
  ],
  "legacy_individual_groups": [],
  "split_teams": []
}
```

The new stable issue codes are `missing_flight_assignment`,
`ineligible_flight_assignment`, `empty_flight`,
`legacy_individual_groups_present`, and `team_split_across_flights`. Detail arrays
and issue ordering are deterministic. A team with one member missing a flight is
reported as missing assignment, not falsely as split.

## Opening transaction

The private validation GET uses repeatable-read membership authorization. Opening
locks the round first, then the tournament and entrant rows, loads teams and
flights, and applies the same pure validator. Existing pairing triggers acquire
the parent round lock, so a mutation that commits first is observed by opening;
opening that owns the lock first freezes the waiting mutation.

Failed opening writes no handicap snapshots, changes no status, and emits no SSE.
A ready round captures the same preserved handicap facts as before, changes to
open, commits, and publishes one payload-free round event. Scoring ownership,
handicap formulas, completion, leaderboards, and historical current-team reads
are unchanged.

## Validation

Pure lifecycle coverage now includes ready individual/scramble facts, legacy
groups, missing/ineligible/empty flights, exact scramble team sizes, multiple
teams in one flight, true split teams, missing-without-false-split behavior, and
unchanged course facts. Independent validation passed 6 focused pure tests and
39 focused PostgreSQL tests, including lifecycle 12/12, pairings 13/13, flight
migration 7/7, and leaderboards 7/7. The standard workspace passed 64 tests and
the complete database-enabled workspace passed 191/191. Formatting, all-target/
all-feature checking, Clippy with warnings denied, and `git diff --check` passed.

A fresh migration build applied versions 1–12 to an isolated PostgreSQL 17.10
database. The unchanged seed ran twice idempotently with one tournament, eight
players, five rounds, eight teams, sixteen team memberships, and intentionally
zero flights or flight memberships. The isolated database and temporary build
target were removed.

## Next boundary

Populate representative development seed flights and teams, then build the
mobile roster editor. Membership-wide score listing/mutation authority remains a
separate later boundary; this readiness step grants no score permission.
