# Latest explanation

## Counted-round configuration

Every tournament now preserves `counted_rounds` in the database, bounded to
`1..=number_of_rounds`. Migration 0014 backfills existing tournaments to count
all of their rounds, while creator onboarding can choose a smaller best-N value.
The wizard continues tracking “all” as rounds are added, preserves an explicit
smaller choice, and clamps it safely when rounds are removed.

The representative five-round seed deliberately stores three counted rounds.
Leaderboard ranking is unchanged in this release; consuming the persisted value
is the next Phase 7 boundary.

```ts
const countedRounds = draft.countedRoundsMode === 'all'
  ? draft.rounds.length + 1
  : draft.countedRounds
```

## Permanent administration boundary

An exact tournament admin can update the setting from the management workspace
while every round is still draft. The strict PATCH request includes the expected
tournament timestamp, rejects stale writes, returns the authoritative private
tournament, and emits one payload-free tournament invalidation only after a real
committed change. No-op writes preserve the timestamp and emit no event.

The repository locks every round in deterministic order before locking the
tournament, matching the opening workflow so configuration and opening cannot
cross. PostgreSQL independently requires transaction-local tournament and admin
context. Any non-draft round, handicap snapshot, team snapshot, or durable
first-opening marker freezes the value permanently, including after child data
is later deleted.

The frontend editor treats its all-draft check as presentation only. Stale and
locked responses discard the local draft and trigger an authoritative refetch;
successful responses update the detail cache and invalidate the complete
user-scoped tournament query root.

## Review and validation

Review identified four issues: preserving a custom N through remove-then-add,
testing the real opening race, avoiding round locks for clearly unauthorized
requests, and explicit no-op persistence/SSE coverage. All four were corrected
and re-review found no remaining defect.

The final automated ladder passed formatting, 75 standard Rust tests, strict
all-feature Clippy, 215 PostgreSQL-enabled tests, an isolated migration plus two
idempotent seed runs with a `5/3` readback, 158 frontend tests, strict TypeScript,
ESLint, the production build, diff checks, and production file-size checks. The
course-revision upgrade fixture was extended through migration 0014 so the
current seed is also verified against the complete upgraded schema.

Real-Chrome validation could not run in this iteration because the environment's
automatic approval quota rejected starting the isolated backend and explicitly
prohibited retry until August 31. The requested mobile/desktop onboarding,
management, stale/error, locked, non-admin, accessibility, overflow, console,
and network cases therefore remain recorded as unavailable browser evidence;
they are not claimed as passed.

## Roadmap order

The persisted configuration boundary is complete. The next Phase 7 step is to
select and expose each metric's best completed N contributions. Additional play
modes remain deferred until the remaining roadmap, optimization, and security
review are finished.
