# Latest explanation

## Representative deterministic seed pairings

The development tournament now demonstrates the complete flight-aware pairing
model instead of stopping at course, entrant, and partial team data. All five
rounds are ready under the same repository validation used by round opening.

Each round has two four-player flights. Flight 1 starts on hole 1 and Flight 2
starts on hole 10, while player membership rotates between rounds. Scramble
rounds one, two, and four each have four two-player score-owner teams, with two
complete teams contained in each flight. Individual rounds three and five have
no teams because their players own their scorecards directly; flights provide
grouping only.

This preserves the central boundary: teams own shared scramble scores, while
flights own scheduling and determine future membership-wide scoring access.
This seed step does not grant that runtime score permission.

## Safe refresh behavior

The seed remains one transaction and can be run repeatedly. Every pairing insert
joins the exact seeded parent round while it is still draft before the statement
reaches the database pairing triggers. A frozen round therefore produces no
attempted pairing mutation, rather than relying on `ON CONFLICT` after a trigger
has already rejected the statement.

Missing deterministic rows are added only when they do not conflict with an
existing player assignment. The previous development seed stored starting holes
on its first eight teams. Those known values are cleared only when the round is
draft and the exact team identity, two-player membership, matching four-player
flight, and schedule all agree:

```sql
UPDATE teams seeded_team
SET starting_hole = NULL
FROM seeded_schedule schedule
JOIN rounds seeded_round
  ON seeded_round.id = schedule.round_id
 AND seeded_round.status = 'draft'
JOIN flights seeded_flight
  ON seeded_flight.id = schedule.flight_id
WHERE seeded_team.id = schedule.team_id
  AND seeded_team.starting_hole = schedule.starting_hole;
```

The production statement adds the exact tournament, name, tee-time, membership,
and flight-size guards omitted from this compact example. That schedule
conversion leaves edited or partially conflicting facts alone; the insert
backfills neither delete nor overwrite rows, and no frozen-round trigger is
bypassed.

## Coverage and validation

The focused PostgreSQL seed test now proves exact rotations and stable totals of
12 teams, 24 team memberships, 10 flights, and 40 flight memberships. It also
checks that individual rounds remain team-free, every scramble team is exactly
two players contained in one flight, and all five rounds pass shipped pairing
readiness. The test recreates the old round-two draft seed before refreshing it,
then opens round one and reruns the seed to prove frozen pairings remain
unchanged.

Validation passed formatting, whitespace checks, Clippy with warnings denied,
all 64 standard workspace tests, and all 191 PostgreSQL-enabled tests. The latter
includes the focused seed test and all ten course-revision tests. The historical
course-revision fixture still proves its version-10 upgrade behavior, then brings
that test database through migrations 11 and 12 before executing the current
flight-aware seed.

## Next boundary

Build the mobile roster editor for teams, flights, membership, starting holes,
and tee times. Extending score-access listing and score mutations to every exact
flight member remains the following separately approved step.
