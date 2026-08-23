# Latest explanation

## One transactional round-pairing roster

Draft pairings now have one aggregate backend boundary instead of several
granular team mutations. Any tournament member can read
`GET /api/rounds/{round_id}/pairings`; an exact tournament admin can replace the
desired current state with `PUT` plus the visible round timestamp. Success and
error responses are private and non-cacheable.

The read model contains the round identity, status, scoring format, optimistic
timestamp, effectively active/inactive entrants, shared-result teams, flights,
and legacy individual groups. Ordering is deterministic. A player is eligible
only when both the tournament entry and global player record are active.

The strict PUT accepts client-generated team/flight UUIDs, members in display
order, optional flight schedules, explicit scramble schedule-flight links, and
explicit legacy conversions. Empty and partial draft rosters are intentionally
valid; the later readiness step will report missing or split assignments.

## Transaction and conversion policy

The repository locks the exact round first, revalidates the session and admin
membership, checks draft state and `expected_round_updated_at`, validates every
identity and reference, and replaces teams, flights, and memberships in one
transaction. It advances the round timestamp and assembles the returned model
before commit. Exactly one payload-free round event is sent after success; every
failure rolls back without an event.

Individual-round teams were historically used as grouping containers even
though individual scores are player-owned. They now appear as
`legacy_individual_groups` and convert only through an exact team-to-flight
mapping. The mapped flight must preserve name, starting hole, tee time, ordered
members, nullable display orders, and group/membership timestamps. No grouping is
inferred from equal schedule facts.

Scramble teams are different: they are durable shared score owners used by
scores, confirmations, completion, and leaderboards, so their identity is never
converted. A retained scheduled scramble team must explicitly name one requested
flight containing exactly its members and identical schedule before those old
team schedule columns are cleared.

```json
{
  "expected_round_updated_at": "2026-08-23T12:00:00Z",
  "teams": [],
  "flights": [
    {
      "id": "4c6f82d9-9f45-4dfd-bc04-f84fcdb3b472",
      "name": "Flight 1",
      "starting_hole": 1,
      "tee_time": "08:30:00",
      "members": [{ "player_id": "56a5d042-cdc8-4320-a2ce-cb67ca79cf40" }]
    }
  ],
  "legacy_conversions": []
}
```

The old team-create, assign-member, and remove-member HTTP routes are retired so
supported clients cannot bypass aggregate optimistic concurrency. The existing
member-readable team list remains temporarily for the current frontend.

## Preserved invariants

- Teams remain round-specific shared-result owners; flights never own scores.
- Scramble team IDs, membership, score history, and leaderboard attribution never
  convert.
- A player appears on at most one team and one flight per round.
- Only eligible tournament entrants may be submitted, while incomplete draft
  state remains representable.
- Pairing writes are exact-admin, draft-only, atomic, and serialized with opening.
- Flight membership grants no score permission in this step; runtime score
  authorization remains a later boundary with no designated scorekeeper.

## Validation

The focused PostgreSQL pairing suite passed 13/13 tests. It covers member/private
reads, strict authorization-before-decoding, partial rosters, effective activity,
exact legacy facts, scramble identity/schedule transfer, identity/reference and
mapping conflicts, retired routes, rollback/no-event behavior, simultaneous
same-token replacement, session/membership revocation, and both opening lock
orders. Owner review found no remaining release issue.

The standard Rust suite passed 63 tests. The focused pairing, authorization,
private-read, and lifecycle PostgreSQL suites passed 31 tests, and the full
database-enabled workspace passed 187. Formatting, all-target/all-feature
checking, Clippy with warnings denied, and `git diff --check` passed. A fresh
migration build applied versions 1–12 to an isolated PostgreSQL 17.10 database;
the unchanged seed then ran twice with stable counts: one tournament, eight
players, five rounds, eight teams, sixteen team memberships, and no inferred
flights. The isolated database and build artifacts were removed.

Linking the seed binary inside the disposable fresh Cargo target hit an LLVM bus
error twice. The migration binary had already built and run there, and the
unchanged repository-target seed binary passed twice against that same isolated
migrated database, so this was an environment linker failure rather than a
product failure.

## Next boundary

Split the near-limit lifecycle modules, then make readiness validate complete
flight/team/player assignment and split teams before opening. Seed assignments,
membership-wide score authorization, and the mobile roster editor remain later
steps.
