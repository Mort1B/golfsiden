# Architecture

## Application boundaries

- `backend/` is an Axum service using Tokio, SQLx, PostgreSQL, Serde, UUIDs, and tracing.
- `frontend/` is a Vite React application using strict TypeScript, React Router, and TanStack Query.
- `migrations/` owns the production database schema.
- `docs/` records architecture and phased delivery decisions.

Backend request handling is split into `api`, `repositories`, and `domain`. Handlers own HTTP validation and response mapping, repositories own SQL and transaction mechanics, and pure handicap/scoring/lifecycle behavior stays in `domain`. Authentication and score authorization are isolated modules rather than handler-local policy.

## Domain decisions

- Team membership is round-specific. `team_memberships` includes `round_id`, with a unique constraint on `(round_id, player_id)`.
- Tournament entrants preserve an initial handicap, and `round_handicap_snapshots` preserves the exact handicap, course handicap, and playing handicap used in each round.
- Round opening locks the round and tournament, repeats readiness validation, and captures one immutable snapshot for each active entrant before changing status. A transaction-local opening context prevents direct status or snapshot bypasses.
- Course handicap uses exact tenths and rational arithmetic for `index * slope / 113 + rating - par`. Individual allowance is applied to the unrounded result before final rounding; scramble member snapshots retain rounded course handicaps for the later team formula.
- Team, membership, tee, and hole mutation guards serialize through the parent-round lock. Once open, scoring configuration and pairings cannot drift.
- A score has exactly one owner through an exclusive player/team check constraint.
- Session tokens are opaque 256-bit values stored only as SHA-256 hashes.
  Nullable unique `users.player_id` links an account to a golf identity without
  email inference. Session roles remain extensible.
- The score authorization resolver returns tagged round owners. Admin/scorer
  roles receive all eligible owners; players receive their exact individual or
  round-team owner. Save and confirm recheck this policy under a session-row lock
  in the score transaction.
- A future explicit flight relation can extend that resolver to return both
  teams in one flight. Starting-hole and tee-time coincidences carry no
  authorization meaning.
- Team results can be attributed back to every round member when tournament standings are calculated. There is no permanent tournament team.
- Locked-round score protection lives in PostgreSQL as well as the domain service. A future correction transaction must explicitly set `app.admin_correction = 'true'`.
- Score changes are audited by a database trigger.
- Score writes and confirmation serialize on the round row. Repository writes set
  a transaction-local context, while database triggers acquire the same lock with
  `NOWAIT` for direct SQL so reverse lock ordering fails instead of deadlocking.
- Scorecard confirmation is separate from score submission. A correction removes
  the current confirmation; stroke audit history remains append-only.
- Completion and locking serialize on the round row before reading scorecard
  state. Individual readiness is keyed by immutable round snapshots; scramble
  readiness is keyed by frozen round teams. Both the repository and lifecycle
  trigger require every owner to have exactly the configured hole count and a
  current confirmation.
- Transaction-local lifecycle settings route application writes through the
  expected integrity paths; they are not an authorization boundary. Runtime role
  separation and database privilege hardening belong with authentication work.
- Round leaderboards calculate live gross/net score-to-par from the holes actually
  scored. Tournament leaderboards aggregate only completed or locked rounds and
  attribute scramble results through frozen membership for that round. Separate
  gross and net routes never use the other metric as a hidden tie-break.
- Leaderboard repositories bulk-load rounds, holes, snapshots, teams,
  memberships, scores, and confirmations inside one repeatable-read, read-only
  transaction. Pure domain assembly validates stored facts, calculates handicap
  results, attributes players, and applies deterministic competition ranking.
- Server-Sent Events carry invalidation notifications, not full mutable state. Clients refetch through TanStack Query.
- The global leaderboard route owns selection in canonical URL parameters instead
  of a client store. It validates round ownership before enabling hierarchical
  queries, and leaderboard responses pass focused runtime decoding before
  entering the query cache.
- The score route likewise owns tournament, round, tagged owner, hole, and view
  selection in canonical URL parameters. Completion validation is its owner
  authority, and exact runtime decoders protect scorecard state before caching.
- Hole mutation intent stays outside TanStack Query in one round/owner/hole
  coordinator. It serializes writes, coalesces rapid input, and requires an
  authoritative refetch match before reporting synchronization. Route and unload
  guards prevent unresolved intent from being silently abandoned.
- Handicap and net-score calculations remain backend-owned. Pending gross input
  is visible immediately, but net output is shown only after decoded server
  verification.

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
| `GET` | `/api/rounds/{round_id}/completion-validation` | Inspect per-owner completion and lock readiness |
| `GET` | `/api/rounds/{round_id}/score-access` | Retrieve writable score owners for the session |
| `POST` | `/api/rounds/{round_id}/complete` | Complete a ready open round atomically |
| `POST` | `/api/rounds/{round_id}/lock` | Lock a ready completed round atomically |
| `GET` | `/api/rounds/{round_id}/leaderboards/gross` | Retrieve the live gross round leaderboard |
| `GET` | `/api/rounds/{round_id}/leaderboards/net` | Retrieve the live net round leaderboard |
| `PUT` | `/api/rounds/{round_id}/scores` | Save or correct one hole score |
| `GET` | `/api/rounds/{round_id}/scorecards/{owner_type}/{owner_id}` | Retrieve a gross/net scorecard summary |
| `POST` | `/api/rounds/{round_id}/scorecards/{owner_type}/{owner_id}/confirm` | Confirm a complete scorecard |
| `GET`, `POST` | `/api/rounds/{round_id}/teams` | List and create round teams |
| `POST` | `/api/teams/{team_id}/members` | Assign a tournament player |
| `DELETE` | `/api/teams/{team_id}/members/{player_id}` | Remove an assignment |
| `GET` | `/api/tournaments/{tournament_id}/leaderboards/gross` | Retrieve individual tournament gross standings |
| `GET` | `/api/tournaments/{tournament_id}/leaderboards/net` | Retrieve individual tournament net standings |
| `GET` | `/api/live` | SSE invalidation events |
| `GET` | `/api/health` | Liveness response |
| `POST` | `/api/auth/login` | Verify credentials and create a session |
| `GET` | `/api/auth/session` | Retrieve the current session and CSRF value |
| `POST` | `/api/auth/logout` | Revoke and clear the current session |

Errors consistently use `{ "error": { "code": "...", "message": "..." } }`.

## Deferred decisions

- Authorization for all non-scoring mutation routes and production login rate limiting.
- Normalized round flights and flight-wide score permissions.
- Separate migration and runtime database roles plus production privilege policy.
- Regional alternatives to the implemented WHS course-handicap conversion.
- Scramble formulas beyond the initial configurable 35%/15% implementation.
- Configurable tie-break ordering beyond shared competition positions.
- Public leaderboard token/link design.
- Offline mutation queue and score conflict presentation.
