# Architecture

## Application boundaries

- `backend/` is an Axum service using Tokio, SQLx, PostgreSQL, Serde, UUIDs, and tracing.
- `frontend/` is a Vite React application using strict TypeScript, React Router, and TanStack Query.
- `migrations/` owns the production database schema.
- `docs/` records architecture and phased delivery decisions.

Backend request handling is split into `api`, `repositories`, and `domain`. Handlers own HTTP validation and response mapping, repositories own SQL and transaction mechanics, and pure handicap/scoring/lifecycle behavior stays in `domain`. Authentication is a separate future boundary; the database already represents users and roles.

## Domain decisions

- Team membership is round-specific. `team_memberships` includes `round_id`, with a unique constraint on `(round_id, player_id)`.
- Tournament entrants preserve an initial handicap, and `round_handicap_snapshots` preserves the exact handicap, course handicap, and playing handicap used in each round.
- Round opening locks the round and tournament, repeats readiness validation, and captures one immutable snapshot for each active entrant before changing status. A transaction-local opening context prevents direct status or snapshot bypasses.
- Course handicap uses exact tenths and rational arithmetic for `index * slope / 113 + rating - par`. Individual allowance is applied to the unrounded result before final rounding; scramble member snapshots retain rounded course handicaps for the later team formula.
- Team, membership, tee, and hole mutation guards serialize through the parent-round lock. Once open, scoring configuration and pairings cannot drift.
- A score has exactly one owner through an exclusive player/team check constraint.
- Team results can be attributed back to every round member when tournament standings are calculated. There is no permanent tournament team.
- Locked-round score protection lives in PostgreSQL as well as the domain service. A future correction transaction must explicitly set `app.admin_correction = 'true'`.
- Score changes are audited by a database trigger.
- Server-Sent Events carry invalidation notifications, not full mutable state. Clients refetch through TanStack Query.

## API milestone

Implemented resources:

| Method | Path | Purpose |
| --- | --- | --- |
| `GET`, `POST` | `/api/players` | List and create players |
| `GET`, `PATCH`, `DELETE` | `/api/players/{player_id}` | Retrieve, edit, or deactivate a player |
| `GET`, `POST` | `/api/players/{player_id}/handicaps` | Handicap history and changes |
| `GET`, `POST` | `/api/tournaments` | List and create tournaments |
| `GET` | `/api/tournaments/{tournament_id}` | Retrieve a tournament |
| `GET`, `POST` | `/api/tournaments/{tournament_id}/players` | List and register entrants |
| `GET`, `POST` | `/api/tournaments/{tournament_id}/rounds` | List and create rounds |
| `GET` | `/api/rounds/{round_id}` | Retrieve a round |
| `GET` | `/api/rounds/{round_id}/pairing-validation` | Validate assignments and course readiness |
| `POST` | `/api/rounds/{round_id}/open` | Atomically open a ready draft round |
| `GET`, `POST` | `/api/rounds/{round_id}/teams` | List and create round teams |
| `POST` | `/api/teams/{team_id}/members` | Assign a tournament player |
| `DELETE` | `/api/teams/{team_id}/members/{player_id}` | Remove an assignment |
| `GET` | `/api/live` | SSE invalidation events |
| `GET` | `/api/health` | Liveness response |

Errors consistently use `{ "error": { "code": "...", "message": "..." } }`.

## Deferred decisions

- Authentication provider, session mechanism, and authorization policy.
- Regional alternatives to the implemented WHS course-handicap conversion.
- Scramble formulas beyond the initial configurable 35%/15% implementation.
- Tie-break ordering and tournament treatment of incomplete scorecards.
- Public leaderboard token/link design.
- Offline mutation queue and score conflict presentation.
