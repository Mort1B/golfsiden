# Architecture

## Application boundaries

- `backend/` is an Axum service using Tokio, SQLx, PostgreSQL, Serde, UUIDs, and tracing.
- `frontend/` is a Vite React application using strict TypeScript, React Router, and TanStack Query.
- `migrations/` owns the production database schema.
- `docs/` records architecture and phased delivery decisions.

Backend request handling is split into `api`, `repositories`, and `domain`. Handlers own HTTP validation and response mapping, repositories own SQL and transaction mechanics, and pure handicap/scoring/lifecycle behavior stays in `domain`. Authentication and score authorization are isolated modules rather than handler-local policy.

## Domain decisions

- Team membership is round-specific. `team_memberships` includes `round_id`, with a unique constraint on `(round_id, player_id)`.
- `tournament_players.tournament_handicap` is the current competition handicap
  for one trip. Its append-only history retains tournament changes, while
  `round_handicap_snapshots` preserves the exact handicap, course handicap, and
  playing handicap used in each opened round.
- Round opening locks the round and tournament, repeats readiness validation, and captures one immutable snapshot for each active entrant before changing status. A transaction-local opening context prevents direct status or snapshot bypasses.
- Course handicap uses exact tenths and rational arithmetic for `index * slope / 113 + rating - par`. Individual allowance is applied to the unrounded result before final rounding; scramble member snapshots retain rounded course handicaps for the later team formula.
- Team, membership, tee, and hole mutation guards serialize through the parent-round lock. Once open, scoring configuration and pairings cannot drift.
- A score has exactly one owner through an exclusive player/team check constraint.
- Session tokens are opaque 256-bit values stored only as SHA-256 hashes.
  Nullable unique `users.player_id` links an account to a golf identity without
  email inference. Global roles remain temporarily for platform compatibility;
  `tournament_memberships` is authoritative for trip administration and scoring.
- First-time creator onboarding is one transaction across the player, account,
  both initial handicap histories, tournament, admin membership, entrant,
  complete draft round plan, invitation, and session. Client-supplied roles,
  actor IDs, lifecycle status, round count, and tournament scoring summary are
  absent from the contract. The server derives them from preserved facts.
- Invitation URLs contain a non-secret UUID in the path and a 256-bit secret in
  the fragment. PostgreSQL stores only its SHA-256 hash. The invitation creator
  must be a member of the same tournament, and the raw secret is returned once
  with a non-cacheable response.
- Invitation rotation creates an immutable successor in the same series and
  preserves expiry and maximum-use policy. Redemptions are exact user/player/
  membership/entrant facts, unique per tournament identity, and append-only
  during tournament lifetime. Series-root locking plus a PostgreSQL insert guard
  enforces lifecycle and capacity for repository and direct SQL writes.
- Authenticated invitation acceptance uses only `users.player_id`. A complete
  active membership/entrant pair is idempotent before lifecycle checks; partial
  compatible state is repaired, while inactive or withdrawn identities fail
  closed. Joining never creates team or flight membership.
- Public invitation handlers authenticate an extractable token before strict
  secondary-field decoding. Registration hashes outside the transaction after a
  cheap link preflight, then revalidates with database time after row-lock waits.
  Cookies and SSE invalidations remain post-commit only.
- Argon2 creation work is capped to four blocking tasks. An owned semaphore
  permit stays inside the non-cancellable blocking closure, so request
  cancellation cannot release capacity while hashing continues.
- Tournament mutation repositories resolve the target trip from tournament,
  round, or team identifiers and revalidate the active session plus membership
  inside the write transaction. A global administrator is not a cross-tournament
  authorization bypass.
- The score authorization resolver returns tagged round owners. Tournament
  admins/scorers receive all eligible owners; tournament players receive their
  exact individual or round-team owner. Save and confirm recheck this policy
  under session and membership locks in the score transaction.
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
| `POST` | `/api/tournaments/{tournament_id}/players/{player_id}/handicaps` | Change a tournament entrant's current handicap |
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
| `GET` | `/api/me/tournaments` | List the session user's tournament memberships and player links |
| `POST` | `/api/onboarding/tournaments` | Atomically create a first-time creator, draft tournament plan, invitation, and session |
| `POST` | `/api/invitations/{invitation_id}/preview` | Preview minimal tournament data for an authenticated invitation token |
| `POST` | `/api/invitations/{invitation_id}/register` | Atomically register and join a new player account |
| `POST` | `/api/invitations/{invitation_id}/accept` | Join the exact session-linked player idempotently |
| `GET`, `POST` | `/api/tournaments/{tournament_id}/invitations` | List metadata or issue a tournament-admin invitation |
| `POST` | `/api/tournaments/{tournament_id}/invitations/{invitation_id}/rotate` | Revoke and replace one active invitation |
| `DELETE` | `/api/tournaments/{tournament_id}/invitations/{invitation_id}` | Idempotently revoke an invitation |

Errors consistently use `{ "error": { "code": "...", "message": "..." } }`.

## Deferred decisions

- Creator email verification, private-read cutover, and production signup/login
  and invitation-registration rate limiting.
- Normalized round flights and flight-wide score permissions.
- Separate migration and runtime database roles plus production privilege policy.
- Regional alternatives to the implemented WHS course-handicap conversion.
- Scramble formulas beyond the initial configurable 35%/15% implementation.
- Configurable tie-break ordering beyond shared competition positions.
- Public leaderboard token/link design.
- Offline mutation queue and score conflict presentation.
