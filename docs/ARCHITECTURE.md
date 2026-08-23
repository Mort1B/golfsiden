# Architecture

## Application boundaries

- `backend/` is an Axum service using Tokio, SQLx, PostgreSQL, Serde, UUIDs, and tracing.
- `frontend/` is a Vite React application using strict TypeScript, React Router, and TanStack Query.
- `migrations/` owns the production database schema.
- `docs/` records architecture and phased delivery decisions.

Backend request handling is split into `api`, `repositories`, and `domain`. Handlers own HTTP validation and response mapping, repositories own SQL and transaction mechanics, and pure handicap/scoring/lifecycle behavior stays in `domain`. Authentication and score authorization are isolated modules rather than handler-local policy.

## Domain decisions

- Team membership is round-specific. `team_memberships` includes `round_id`, with a unique constraint on `(round_id, player_id)`.
- `tournament_players.tournament_handicap` is the fixed competition handicap for
  one trip. A tournament admin may use the explicit correction workflow only
  before any round has opened or snapshot has existed. Append-only history and a
  durable tournament lock marker preserve that decision even if round data is
  later deleted. `round_handicap_snapshots` preserves the exact effective index,
  course handicap, and playing handicap used in each opened round.
- Round opening locks the round and tournament, repeats readiness validation, and captures one immutable snapshot for each active entrant before changing status. A transaction-local opening context prevents direct status or snapshot bypasses.
- Course handicap uses exact tenths and rational arithmetic for `index * slope / 113 + rating - par`. Individual allowance is applied to the unrounded result before final rounding. Scramble caps each registered index at `36.0` before tee conversion; its member snapshots retain that effective index and rounded course handicap for the later team formula.
- Team, flight, membership, tee, and hole mutation guards serialize through the
  parent-round lock. Once open, scoring configuration and pairings cannot drift.
- Flights are normalized round/tournament-scoped groupings independent of teams.
  A player can belong to at most one flight per round. Score authority will be
  derived at runtime from an authenticated player's exact stored flight
  membership, so the persistence model has no designated-scorekeeper relation
  or account-link requirement. Existing team data is not inferred or migrated
  into flights.
- A score has exactly one owner through an exclusive player/team check constraint.
- Accounts use a canonical lowercase username matching `[a-z0-9_-]{3,32}` and a
  password. Usernames are case-insensitively unique; account email is not stored
  or accepted. Session tokens are opaque 256-bit values stored only as SHA-256
  hashes. Nullable unique `users.player_id` links an account to a golf identity
  without profile-email inference. Global roles remain temporarily for platform compatibility;
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
- Tournament workspace read repositories resolve the target trip from stored
  tournament or round relations and require any tournament membership. Multi-query
  reads use one repeatable-read transaction and hold that membership `FOR SHARE`
  through response assembly; tournament collections join memberships directly.
  Successful private responses are `private, no-store`, and global roles are no
  cross-tournament read bypass.
- Course discovery is a bundled JSON catalog, not provider free-text search or a
  persistence model. Its tournament-scoped GET handler requires exact admin
  membership, searches names and internal aliases locally, preserves file order,
  and reports nullable verified provider IDs plus explicit readiness. Missing or
  incomplete entries cannot cross the detail boundary. Future usable detail
  reads commit authorization before cache/network work; the adapter owns the
  sensitive Bearer credential, time/concurrency/body bounds, finite TTL cache,
  and per-process UTC quota. It decodes the provider's wrapped detail shape into
  stable local DTOs, derives ordered hole numbers, and never invents a tee ID.
  Provider facts remain untrusted until normalized; no course, tee, hole, or
  round row changes in this boundary.
- Provider and manual course facts converge on the existing `courses` → `tees` →
  `holes` identity graph through one pure validator and caller-owned repository
  transaction. A finalized revision records source, nullable opaque provider
  course identity, database import time, one selected tee name/category, rating,
  slope, and complete ordered par/stroke-index facts; hole distance is nullable.
  Deferred PostgreSQL validation prevents incomplete finalization, and locked
  ancestor reads serialize finalization with child writes. Finalized hierarchies
  are append-only. Pre-migration rows keep null revision metadata instead of
  receiving invented provenance.
- Draft-round course configuration is a conditional `PUT` with the current
  round `updated_at` as a required optimistic token. A short repeatable-read
  preflight proves exact tournament-admin scope and draft state before request
  decoding or provider quota use. Provider detail is fetched with no database
  transaction open. The final transaction locks the round, reauthorizes the
  active session and membership, rechecks status and version, inserts the
  immutable revision, and attaches its UUIDs before commit. This round-first lock
  order matches lifecycle mutations. Only a successful commit publishes one
  payload-free round invalidation; every conflict or failure rolls back the new
  hierarchy.
- Round pairings use one aggregate read/write boundary. The member-readable GET
  authorizes and assembles entrants, teams, flights, and legacy individual
  groups in one repeatable-read transaction. The admin-only PUT locks the exact
  round, reauthorizes session and membership, checks the optimistic timestamp,
  and replaces the requested partial draft roster atomically. Individual legacy
  grouping teams convert only through an exact mapping that preserves schedule,
  membership order, and timestamps. Scramble teams remain durable score-owner
  identities; old team schedule moves only to an explicitly named flight with
  identical members and facts. One round event follows commit.
- The score authorization resolver returns tagged round owners. Tournament
  admins/scorers receive all eligible owners; tournament players receive their
  exact individual or round-team owner. Save and confirm recheck this policy
  under session and membership locks in the score transaction.
- The normalized flight relation is not yet part of score authorization. A later
  shared policy can extend the resolver to return every eligible owner in the
  authenticated player's exact flight. Starting-hole and tee-time coincidences
  carry no authorization meaning, and tournament admin/scorer overrides remain
  separate from flight membership.
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
- Private workspace query keys are rooted by session user ID. Initial or changed
  identities clear that root before publication; same-user refreshes preserve it.
  SSE invalidation excludes the auth query so live events do not unmount protected
  scoring state.
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
| `GET`, `POST` | `/api/tournaments/{tournament_id}/players` | List the roster and correction state, or register entrants |
| `POST` | `/api/tournaments/{tournament_id}/players/{player_id}/handicap-corrections` | Audit a pre-opening tournament handicap correction |
| `GET`, `POST` | `/api/tournaments/{tournament_id}/rounds` | List and create rounds |
| `GET` | `/api/rounds/{round_id}` | Retrieve a round |
| `PUT` | `/api/rounds/{round_id}/course-configuration` | Atomically configure one draft round from manual or curated provider facts |
| `GET`, `PUT` | `/api/rounds/{round_id}/pairings` | Read or atomically replace the private draft team/flight roster |
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
| `GET` | `/api/rounds/{round_id}/teams` | Compatibility read for round teams |
| `GET` | `/api/tournaments/{tournament_id}/leaderboards/gross` | Retrieve individual tournament gross standings |
| `GET` | `/api/tournaments/{tournament_id}/leaderboards/net` | Retrieve individual tournament net standings |
| `GET` | `/api/live` | SSE invalidation events |
| `GET` | `/api/health` | Liveness response |
| `POST` | `/api/auth/login` | Verify credentials and create a session |
| `GET` | `/api/auth/session` | Retrieve the current session and CSRF value |
| `POST` | `/api/auth/logout` | Revoke and clear the current session |
| `GET` | `/api/me/tournaments` | List the session user's tournament memberships and player links |
| `GET` | `/api/tournaments/{tournament_id}/course-catalog` | Search the bundled curated course shortlist as a tournament admin |
| `GET` | `/api/tournaments/{tournament_id}/course-provider/courses/{provider_course_id}` | Retrieve normalized provider tee and hole detail as a tournament admin |
| `POST` | `/api/onboarding/tournaments` | Atomically create a first-time creator, draft tournament plan, invitation, and session |
| `POST` | `/api/invitations/{invitation_id}/preview` | Preview minimal tournament data for an authenticated invitation token |
| `POST` | `/api/invitations/{invitation_id}/register` | Atomically register and join a new player account |
| `POST` | `/api/invitations/{invitation_id}/accept` | Join the exact session-linked player idempotently |
| `GET`, `POST` | `/api/tournaments/{tournament_id}/invitations` | List metadata or issue a tournament-admin invitation |
| `POST` | `/api/tournaments/{tournament_id}/invitations/{invitation_id}/rotate` | Revoke and replace one active invitation |
| `DELETE` | `/api/tournaments/{tournament_id}/invitations/{invitation_id}` | Idempotently revoke an invitation |

Errors consistently use `{ "error": { "code": "...", "message": "..." } }`.

## Deferred decisions

- Global player, scorecard, and live-event read visibility; production signup,
  login, and invitation-registration rate limiting.
- Flight-aware readiness, membership-wide score authority, seed assignments, and
  the mobile pairing editor.
- Separate migration and runtime database roles plus production privilege policy.
- Regional alternatives to the implemented WHS course-handicap conversion.
- Scramble formulas beyond the initial configurable 35%/15% implementation.
- Configurable tie-break ordering beyond shared competition positions.
- Public leaderboard token/link design.
- Offline mutation queue and score conflict presentation.
