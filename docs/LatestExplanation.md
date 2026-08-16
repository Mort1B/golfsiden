# Latest explanation

## Tournament-scoped authority

Global account roles could not safely represent a private tournament: making a
creator an admin would also grant authority in every other trip. The new
`tournament_memberships` relation assigns `admin`, `scorer`, `player`, or
`viewer` inside one tournament. Existing global administrators and scorers are
explicitly backfilled for compatibility, while linked players are backfilled
only into tournaments they actually entered.

Every protected repository revalidates the active session and matching
tournament membership inside the mutation transaction. Round and team routes
derive their tournament from stored ownership rather than trusting a client ID.
Score access uses the same membership roles: tournament admins/scorers receive
all eligible cards, and both players on a scramble team retain their shared team
card without receiving access elsewhere.

Tournament handicap changes now update `tournament_players` and append an audit
row attributed to the session user. Round opening snapshots this tournament
value. Changing it affects later rounds but never rewrites an opened round.

## Compact example

The membership lock is part of the write transaction:

```sql
SELECT role
FROM tournament_memberships
WHERE tournament_id = $1 AND user_id = $2
FOR SHARE
```

## Invariants

- Global roles do not bypass tournament membership.
- Account and audit identity comes from the active session, never submitted email
  or actor fields.
- Tournament players remain independent from round-specific teams.
- Both teammates can enter their shared team score.
- Handicap changes affect only future round snapshots.
- Public read restriction is deferred until the onboarding frontend can consume
  membership-scoped resources.

## Validation

- Rust format, check, and Clippy with warnings denied pass.
- The complete PostgreSQL unit/API/integration suite passes with 76 tests.
- Migration 6 and two consecutive seed runs pass against PostgreSQL.
- Focused tests cover backfill, cross-tournament IDOR, session/CSRF failures,
  tournament handicap history, lifecycle concurrency, membership score access,
  role downgrade/removal, and both-teammate scoring.
