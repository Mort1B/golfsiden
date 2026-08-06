# Active plans

This file is the source of truth for implementation work. Keep only one active
step. Move durable completed behavior to `Documentation.md` or `ARCHITECTURE.md`
instead of accumulating a historical log here.

## Active step

None. The agent operating model is installed and milestone 1 is complete.

An implementation step must use this shape before code changes begin:

```text
Goal:
Files/modules:
Exact change:
Validation:
Invariants:
Stop condition:
```

## Upcoming work

### Milestone 2: Live scoring and leaderboards

- Add round lifecycle operations and capture immutable round handicap snapshots
  when a round opens.
- Add pairing-completeness validation before opening a round.
- Implement individual and two-player scramble score mutation, correction, audit,
  and scorecard summary APIs.
- Implement round gross/net leaderboards and individual tournament standings,
  including attribution of team results to every team member.
- Build the one-hole mobile scoring flow with immediate save, explicit sync state,
  correction support, and SSE-driven refetch.
- Add API and PostgreSQL integration coverage for scoring, locking, correction,
  snapshots, and standings.

### Milestone 3: Administration and access

- Add authentication and role enforcement for admin, scorer, player, and viewer.
- Add tournament, player, handicap, course/hole, round, entrant, and team admin
  workflows.
- Add the explicit locked-round admin correction workflow.
- Add shareable read-only leaderboard access.

### Milestone 4: Tournament operations

- Add pairing validation UI and a balanced team generator that avoids repeat
  partners.
- Add configurable tie-break rules and additional scoring formats.
- Add offline-tolerant score mutation queuing with visible reconciliation.
- Add deployment, backups, monitoring, and production security hardening.
