# Phased implementation plan

## Milestone 1: Foundation (complete)

- Establish the Rust workspace, Axum service, React application, PostgreSQL schema, repositories, validation, structured errors, and SSE invalidation channel.
- Implement players, handicap changes, tournaments, entrants, rounds, teams, and memberships.
- Add development data for eight players and five rounds with changed pairings.
- Add core scoring/handicap unit tests and database integrity tests.
- Deliver read-only mobile tournament, player, round, and team views.

## Milestone 2: Live scoring and leaderboards

- Add round lifecycle operations, automatic round handicap capture, and pairing-completeness validation.
- Implement individual and scramble score entry, correction endpoints, scorecard summaries, and audit retrieval.
- Implement round gross/net leaderboards and tournament standings, including team-result attribution to each player.
- Build the one-hole mobile scoring workflow with immediate saves, sync state, and SSE-driven leaderboard refresh.
- Add API integration tests for scoring, locking, correction, and standings.

## Milestone 3: Administration and access

- Add authentication and role enforcement for admin, scorer, player, and viewer roles.
- Build tournament, player, handicap, course/hole, round, entrant, and team administration screens.
- Add the explicit locked-round admin correction workflow.
- Add shareable read-only leaderboard access.

## Milestone 4: Tournament operations

- Add pairing validation and a balanced team generator that avoids repeated partners.
- Add configurable tie-break rules and more formats such as best ball and Stableford.
- Add offline-tolerant score mutation queuing with visible reconciliation states.
- Add deployment, backups, monitoring, and production security hardening.
