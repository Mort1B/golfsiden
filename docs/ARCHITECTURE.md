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
  `round_team_handicap_snapshots` preserves the final team Playing Handicap for
  each opened foursomes team and is immutable except for ancestor deletion.
- Round opening locks the round and tournament, repeats readiness validation, and captures one immutable snapshot for each active entrant before changing status. A transaction-local opening context prevents direct status or snapshot bypasses.
- Tournament start is a separate exact-admin lifecycle boundary. Its transaction
  locks the stored round set deterministically before reauthorizing the session
  and exact tournament membership, locking the tournament, and share-locking
  entrants. A complete `1..=number_of_rounds` draft plan plus one effectively
  active entrant is required for `draft -> active`; already-active retries are
  read-only and idempotent. PostgreSQL independently verifies the transaction-
  local tournament/actor context, exact admin membership, plan, and entrant,
  while a separate insert trigger requires every new tournament to begin draft.
  The general repository creation boundary is draft-only as defense in depth.
  Round opening requires an active parent, but course and pairing configuration
  remain independently editable while their individual rounds are draft.
- `tournaments.counted_rounds` is a required cross-column bounded configuration
  fact. Nullable `mandatory_round_id` has a deferred composite foreign key to a
  round in that same tournament. Creator onboarding preallocates round UUIDs so
  both facts persist atomically; the admin mutation uses optimistic tournament
  time and the same round-before-tournament lock order as opening. One database
  trigger protects both fields with exact-admin context and the permanent
  start/open/snapshot freeze. A mandatory round reserves one of N slots even
  when its result is missing; gross and net independently select the remaining
  completed contributions.
- Course handicap uses exact tenths and rational arithmetic for `index * slope / 113 + rating - par`. Individual allowance is applied to the unrounded result before final rounding. Scramble caps each registered index at `36.0` before tee conversion; its member snapshots retain that effective index and rounded course handicap for the later team formula.
- One closed round-format policy is the application source of truth for score-
  owner kind, exact team size, snapshot-handicap treatment, and team playing-
  handicap calculation. Individual stroke play is player-owned and keeps its
  uncapped, unrounded allowance path. Scramble is an exact two-player team format
  with the existing `36.0` cap and 35%/15% calculation. Foursomes is an exact
  two-player team format with a mandatory 50% allowance applied to the combined
  unrounded Course Handicaps and rounded once under WHS allowance rules. Lifecycle readiness,
  pairing persistence, completion, score authorization, and scorecard validation
  consume this policy instead of treating every non-individual format as
  scramble. PostgreSQL constraints and lifecycle triggers remain independent
  enforcement boundaries.
- Team, flight, membership, tee, and hole mutation guards serialize through the
  parent-round lock. Once open, scoring configuration and pairings cannot drift.
- Flights are normalized round/tournament-scoped groupings independent of teams.
  A player can belong to at most one flight per round. Runtime score authority is
  derived from an authenticated player's exact stored flight membership, so the
  persistence model has no designated-scorekeeper relation. Existing team data
  is not inferred or migrated into flights.
- A score has exactly one owner through an exclusive player/team check constraint.
- Accounts use a canonical lowercase username matching `[a-z0-9_-]{3,32}` and a
  password. Usernames are case-insensitively unique; account email is not stored
  or accepted. Session tokens are opaque 256-bit values stored only as SHA-256
  hashes. Nullable unique `users.player_id` links an account to a golf identity
  without profile-email inference. The account role remains part of session
  identity, but has no product-facing global player or tournament-creation
  authority; `tournament_memberships` is authoritative for trip administration
  and scoring.
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
- Direct `POST /api/tournaments/{tournament_id}/players` registration is retired.
  Creator onboarding and invitation registration/acceptance are the only HTTP
  paths that establish participation, so no product route accepts an arbitrary
  global player ID or exposes a global player search.
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
- Direct round creation performs an exact-admin preflight before inspecting
  content type, body, schema, or target-dependent round/course facts. The insert
  transaction then revalidates the active session and exact admin membership
  under locks before writing; invalid or unauthorized requests emit no event.
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
  identifier-free round invalidation; every conflict or failure rolls back the new
  hierarchy.
- Round pairings use one aggregate read/write boundary. The member-readable GET
  authorizes and assembles entrants, teams, flights, and legacy individual
  groups in one repeatable-read transaction. The admin-only PUT locks the exact
  round, reauthorizes session and membership, checks the optimistic timestamp,
  and replaces the requested partial draft roster atomically. Individual legacy
  grouping teams convert only through an exact mapping that preserves schedule,
  membership order, and timestamps. Scramble and foursomes teams remain durable score-owner
  identities; old team schedule moves only to an explicitly named flight with
  identical members and facts. One round event follows commit.
- Pairing replacement keeps one transaction but separates orchestration,
  identity/roster and legacy/schedule validation, and persistence writes. The
  split preserves optimistic concurrency, validation precedence, deterministic
  membership ordering, legacy timestamps, and the single post-commit event.
- Lifecycle readiness is one pure decision shared by the private validation read
  and locked opening transaction. Every effectively active entrant must be in a
  nonempty flight. Individual rounds reject legacy teams; scramble and foursomes
  rounds retain exact two-player score-owner teams and require each team to be wholly contained
  in one flight, while allowing several teams per flight. Schedule metadata is
  deliberately absent from readiness facts. Opening reads pairings only after
  locking the round/tournament and entrant rows; pairing triggers use that same
  round lock, so validation and mutation cannot cross unnoticed.
- The score authorization resolver returns tagged round owners. Tournament
  admins/scorers receive all eligible owners. Tournament players receive their
  direct owner plus every eligible owner in their exact round flight: frozen
  player snapshots for individual play, or complete two-player teams wholly
  contained in that flight for scramble and foursomes. Foursomes authorization
  additionally requires the preserved team handicap snapshot. Starting-hole, tee-time, name, and
  ordering coincidences carry no authorization meaning.
- The private score-access read re-locks the active session/user and exact
  tournament membership through deterministic owner assembly in a repeatable-
  read transaction. Missing target membership is forbidden rather than
  represented as an empty authorized owner set; exact viewers and exact unlinked
  members retain the empty authorized result. Save and confirm invoke the same
  resolver under the existing round, session, and membership locks, retaining
  the session user as audit actor and preventing listing/mutation policy drift.
- A PostgreSQL two-tournament acceptance fixture reuses one global account/player
  with independent tournament handicaps and round snapshots, then combines an A
  player card with a B-only two-player foursomes team. It guards roster, flight,
  team, score authority, card, gross/net leaderboard, mutation, and identifier-free
  event isolation without introducing a permanent tournament team.
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
  readiness is keyed by frozen round teams; foursomes also requires one immutable
  team handicap snapshot per owner. Both the repository and lifecycle
  trigger require every owner to have exactly the configured hole count and a
  current confirmation.
- `tournaments.final_round_back_nine_hidden` is the database-owned visibility
  state for the configured final. It defaults to hidden and has an independent
  `visibility_updated_at` concurrency token. Migration 0018 preserves finals
  already released by the former deadline while keeping every other tournament
  hidden, then removes the deadline column and confirmation-maintenance triggers.
  A narrow PostgreSQL trigger accepts changes only from the exact active-admin
  workflow and advances the dedicated timestamp monotonically.
- Final-round identity is immutable after tournament start: PostgreSQL freezes
  both `tournaments.number_of_rounds` and each child `rounds.round_number` across
  the start boundary. Round-number checks lock round then parent tournament,
  matching lifecycle lock order and serializing start-versus-renumber races.
- One pure visibility policy consumes the exact tournament role, authoritative
  final-round identity, round state, configured hole count, and persisted hidden
  state. Exact admins receive full reads. Other members receive only holes 1–9
  for a hidden open, completed, or locked 18-hole final. Confirmation, completion,
  locking, database time, and browser time never change that decision.
- Round standings and member scorecards validate the complete stored fact set
  before redaction, then recompute every visible total, progress value, rank,
  and tie from visible facts. Completion reads likewise count actual front-nine
  scores and null completion, confirmation, and readiness. Tournament standings
  omit a hidden completed/locked final round before best-N selection and ranking.
- Member scorecard reads are actor-free projections. A separate `/scoring` GET
  repeats exact writable-owner authorization and returns the full mutation DTO;
  it is non-locking and unavailable once the round is locked. Read and scoring
  projections have separate session-owned frontend cache keys.
- Transaction-local lifecycle settings route application writes through the
  expected integrity paths; they are not an authorization boundary. Runtime role
  separation and database privilege hardening belong with authentication work.
- Round leaderboards calculate live gross/net score-to-par from the holes actually
  scored. Tournament leaderboards independently select each player's displayed
  best N from completed/locked history plus the visible scored portion of the
  deterministic highest-numbered open round. Contributions retain tagged owner,
  provisional state, and hole progress and are attributed through frozen
  membership for that exact round. Completed-only qualification count ranks
  before selected score-to-par and alone controls eligibility; separate gross
  and net routes never use the other metric as a hidden tie-break.
- Round-leaderboard owner construction is isolated from format-neutral stored-
  fact validation, score/confirmation assembly, totals, and ranking. One closed,
  exhaustive policy maps each current scoring format to snapshot-owned entries
  or an exact-size team plus its approved handicap policy. Individual stroke play
  uses the preserved snapshot playing handicap; two-player scramble alone selects
  the existing 35%/15% calculation; foursomes selects its preserved team Playing
  Handicap. No unrecognized format falls back to another
  path, including in the frontend's typed format label mapping.
- Leaderboard repositories bulk-load rounds, holes, snapshots, teams,
  memberships, scores, and confirmations inside one repeatable-read, read-only
  transaction. Pure domain assembly validates stored facts, calculates handicap
  results, attributes players, selects deterministic metric-specific best-N
  contributions, and applies competition ranking. Open-round facts are validated
  fail-closed; only the deterministic highest-numbered open round may enter the
  displayed selection provisionally, while completed-only qualification remains
  unchanged.
- Server-Sent Events carry invalidation notifications, not full mutable state. Clients refetch through TanStack Query.
- Live events carry internal tournament scope from every post-commit producer.
  `/api/tournaments/{tournament_id}/live` authenticates and authorizes the exact
  membership at handshake, filters before emission, revalidates access for every
  matching event, and serializes only the event type plus a fixed, non-sensitive
  `invalidate` data marker required for browser dispatch. The internal tournament
  and resource identifiers never enter the SSE frame. A lagged receiver closes;
  initial connection and native reconnection invalidate the user's private query
  root and refetch authoritative state. The visibility mutation emits a dedicated
  `visibility` event.
- Private workspace query keys are rooted by session user ID. Initial or changed
  identities clear that root before publication; same-user refreshes preserve it.
  Scorecards use that same user-owned root. Tournament-live invalidation targets
  only the active user's private queries and excludes provider/catalog queries,
  so events cannot unmount authentication or expose a predecessor's card.
- Target-bearing frontend DTOs are decoded against the requested tournament,
  round, player, owner, metric, invitation predecessor, and course-configuration
  identities before cache insertion. Roster, round, team, pairing, invitation,
  and leaderboard collections also reject duplicate or internally incoherent
  identities. Runtime validation is a fail-closed cache boundary, not an
  authorization substitute.
- A non-null mandatory-round identity is also composed with the exact decoded
  tournament round collection before settings or leaderboard data enters the
  query cache. Unknown or cross-target round identities fail closed.
- Route shells key tournament, management, round, leaderboard, result-history,
  read-card, and invitation workspaces by their target identity. This makes
  correction/count drafts,
  mutation receipts and errors, and one-time invitation tokens target-local even
  when React Router reuses the page component or an old request completes late.
- The global leaderboard route owns selection in canonical URL parameters instead
  of a client store. It validates round ownership before enabling hierarchical
  queries, and leaderboard responses pass focused runtime and aggregate-coherence
  decoding before entering the query cache.
- Tournament standings refresh the exact rounds query before fetching and
  composing a leaderboard response. This sequences SSE lifecycle transitions so
  a new open/completed projection is never validated against stale round status;
  the extra authoritative fetch is an explicit correctness cost for later
  performance review.
- Protected result-history routes project one exact player from the canonical
  metric-specific tournament leaderboard. Contribution links use the preserved
  tagged historical owner, never the player's current team. Protected result-card
  routes first compose tournament and round identity, then require that exact
  owner in the role-projected round leaderboard before enabling the canonical
  actor-free scorecard read. Both routes reuse session-owned canonical query keys,
  so explicit mutation invalidation, SSE, logout, and identity changes address
  the same cached facts. No drilldown requests score access, completion,
  `/scoring`, confirmation, or mutation endpoints.
- The score route likewise owns tournament, round, tagged owner, hole, and view
  selection in canonical URL parameters. Completion validation is its owner
  authority, and exact runtime decoders protect scorecard state before caching.
  Its writable-card rail intersects completion progress with server-provided
  score access, preserves the hole on quick switches, and replaces rapid switch
  history. The route prefetches only adjacent writable owner keys; TanStack Query
  remains the sole owner of authoritative scorecard reads.
- Visibility events synchronously clear role-projected leaderboard, completion,
  history, drilldown, and actor-free scorecard query state before authoritative
  refetch. An EventSource error performs the same transition without refetching
  while disconnected; `open` repeats it and refreshes after reconnection.
  Writable `/scoring` queries are deliberately excluded. Browser time never
  changes authorization or locally reveals cached facts, and restricted hole
  URLs are canonicalized to the visible prefix.
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
| `GET` | `/api/tournaments` | List only the authenticated account's tournament memberships |
| `GET` | `/api/tournaments/{tournament_id}` | Retrieve a tournament |
| `POST` | `/api/tournaments/{tournament_id}/start` | Start a ready draft tournament as its exact admin without opening a round |
| `GET`, `PATCH` | `/api/tournaments/{tournament_id}/final-round-visibility` | Read or change the exact-admin final-back-nine visibility setting |
| `GET` | `/api/tournaments/{tournament_id}/players` | List the private roster and handicap-correction state |
| `POST` | `/api/tournaments/{tournament_id}/players/{player_id}/handicap-corrections` | Audit a pre-opening tournament handicap correction |
| `GET`, `POST` | `/api/tournaments/{tournament_id}/rounds` | List and create rounds |
| `GET` | `/api/rounds/{round_id}` | Retrieve a round |
| `PUT` | `/api/rounds/{round_id}/course-configuration` | Atomically configure one draft round from manual or curated provider facts |
| `GET`, `PUT` | `/api/rounds/{round_id}/pairings` | Read or atomically replace the private draft team/flight roster |
| `GET` | `/api/rounds/{round_id}/pairing-validation` | Validate assignments and course readiness |
| `POST` | `/api/rounds/{round_id}/open` | Atomically open a ready draft round |
| `GET` | `/api/rounds/{round_id}/completion-validation` | Inspect role-aware visible per-owner progress and lifecycle readiness |
| `GET` | `/api/rounds/{round_id}/score-access` | Retrieve writable score owners for the session |
| `POST` | `/api/rounds/{round_id}/complete` | Complete a ready open round atomically |
| `POST` | `/api/rounds/{round_id}/lock` | Lock a ready completed round atomically |
| `GET` | `/api/rounds/{round_id}/leaderboards/gross` | Retrieve the live gross round leaderboard |
| `GET` | `/api/rounds/{round_id}/leaderboards/net` | Retrieve the live net round leaderboard |
| `PUT` | `/api/rounds/{round_id}/scores` | Save or correct one hole score |
| `GET` | `/api/rounds/{round_id}/scorecards/{owner_type}/{owner_id}` | Retrieve a private member-authorized gross/net scorecard summary |
| `GET` | `/api/rounds/{round_id}/scorecards/{owner_type}/{owner_id}/scoring` | Retrieve the full card after exact writable-owner authorization |
| `POST` | `/api/rounds/{round_id}/scorecards/{owner_type}/{owner_id}/confirm` | Confirm a complete scorecard |
| `GET` | `/api/rounds/{round_id}/teams` | Compatibility read for round teams |
| `GET` | `/api/tournaments/{tournament_id}/leaderboards/gross` | Retrieve individual tournament gross standings |
| `GET` | `/api/tournaments/{tournament_id}/leaderboards/net` | Retrieve individual tournament net standings |
| `GET` | `/api/tournaments/{tournament_id}/live` | Receive identifier-free invalidations for one exact tournament membership |
| `GET` | `/api/health` | Liveness response |
| `POST` | `/api/auth/login` | Verify credentials and create a session |
| `GET` | `/api/auth/session` | Retrieve the current session and CSRF value |
| `POST` | `/api/auth/logout` | Revoke and clear the current session |
| `GET` | `/api/me/tournaments` | List the session user's tournament memberships and player links |
| `PATCH` | `/api/tournaments/{tournament_id}/counted-rounds` | Atomically update best-N and optional mandatory-round configuration before tournament start |
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

- Public scorecard/share-link policy; production signup, login, and invitation-
  registration rate limiting.
- Faster phone switching among several writable flight cards.
- Separate migration and runtime database roles plus production privilege policy.
- Regional alternatives to the implemented WHS course-handicap conversion.
- Scramble formulas beyond the initial configurable 35%/15% implementation.
- Configurable tie-break ordering beyond shared competition positions.
- Public leaderboard token/link design.
- Offline mutation queue and score conflict presentation.
