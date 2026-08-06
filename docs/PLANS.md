# Active plans

This file is the source of truth for implementation work. Keep only one active
step. Move durable completed behavior to `Documentation.md` or `ARCHITECTURE.md`
instead of accumulating a historical log here.

## Active step

None. Live gross/net round leaderboards and individual tournament standings are
complete. Define and approve the mobile leaderboard or scoring UI slice before
implementation.

## Upcoming work

### Milestone 2: Live scoring and leaderboards

- Build mobile gross/net round and tournament leaderboard screens with SSE-driven
  refetch and complete loading, error, empty, and live-progress states.
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
