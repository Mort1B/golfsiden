# Architecture

## Application boundaries

- `backend/` is an Axum service using Tokio, SQLx, PostgreSQL, Serde, UUIDs, and tracing.
- `frontend/` is a Vite React application using strict TypeScript, React Router, and TanStack Query.
- `migrations/` owns the production database schema.
- `docs/` separates current behavior, durable architecture, active work, latest
  rationale, and production deployment/recovery procedures.

Backend request handling is split into `api`, `repositories`, and `domain`. Handlers own HTTP validation and response mapping, repositories own SQL and transaction mechanics, and pure handicap/scoring/lifecycle behavior stays in `domain`. Authentication and score authorization are isolated modules rather than handler-local policy.

## Production delivery boundary

The portable production topology is one same-origin HTTPS boundary. Caddy serves
the immutable Vite build, terminates TLS, applies browser security headers, and
reverse-proxies `/api` plus unbuffered SSE to one Rust API instance. The API and
PostgreSQL have no public listeners; PostgreSQL additionally lives only on the
internal Compose data network. Named volumes own PostgreSQL durability and Caddy
certificate state. The single-instance assumption deliberately permits bounded
in-process request throttling and provider quota accounting; a multi-API
deployment would require shared implementations for both.

Caddy overwrites `X-Golf-Client-IP` and attaches a 256-bit shared proxy secret.
The API accepts that client identity only after a constant-time secret match and
otherwise collapses the request to a non-spoofable direct identity. Public
`Forwarded`, `X-Forwarded-For`, and lookalike internal headers never select a
rate-limit bucket. Abuse-sensitive authentication and invitation routes use a
bounded two-level limiter, while password verification uses a separate shared
four-task Argon2 semaphore.

Database authority is split by lifecycle. The owner credential initializes the
cluster, performs explicit migrations, backups, and restores. A distinct runtime
login receives schema usage plus DML, sequence, and function execution only; it
cannot create or alter schema and cannot modify `_sqlx_migrations`. The API never
migrates in production. Before binding it verifies the exact configured runtime
identity and absence of cluster, schema, and migration-history write authority.
Before binding, and on every readiness check, it compares the applied SQLx
history with all embedded migrations by version, success state, and checksum.
Liveness remains database-independent so operators can distinguish a live
process from a ready service.

Backup and restore are part of this boundary rather than operator folklore. A
custom-format, no-owner/no-privilege dump is written atomically with a
basename-relative checksum so the dump/sidecar pair remains relocatable.
Restore is allowed only into an empty public schema, executes in one transaction,
and reapplies runtime grants before the API is started.

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
  trigger protects these fields and typed `tie_break_policy` with exact-admin
  context and the permanent start/open/snapshot freeze. A mandatory round reserves one of N slots even
  when its result is missing; gross and net independently select the remaining
  completed contributions. Migration 0025 defaults both existing and newly created
  tournaments to `shared_positions`; `final_round_score` is selected through the
  pre-start configuration API. Creation inputs and serialized validated-plan retry
  fingerprints are unchanged. Omitted PATCH policy preserves the saved value;
  explicit null is rejected.
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
- Tournament completion is explicit and exact-admin only. The repository locks
  rounds in UUID order, reauthorizes session/membership, locks the parent, then
  rechecks wall-clock session expiry and the complete configured locked-round
  plan. An expected tournament timestamp rejects stale active transitions;
  completed retries return the current tournament without another event or record.
  One post-commit tournament invalidation follows a changed transition.
- Migration 0019 independently guards active-to-completed transitions with exact
  session context and locked-round readiness, records actor/time in append-only
  `tournament_completions`. Migration 0020 replaces only its temporary archive
  rejection with a separate archive workflow guard. The completion audit actor FK
  may become null on account deletion; completion identity/time remain preserved.
  Valid pre-19 closed history remains unchanged without invented audit actors;
  incompatible closed history fails the upgrade instead of being silently repaired.
- Tournament archive is a separate exact-admin completed-to-archived transition.
  The repository holds session/user and exact membership share locks before the
  parent update lock, then rechecks wall-clock expiry, including idempotent archived
  retries. No round locks are needed: archive never accesses rounds and the completed
  plan is already immutable. Expected parent timestamps reject stale transitions;
  only a changed commit emits a tournament event. Migration 0020 independently
  requires active exact-admin workflow context and records actor/database time in
  append-only `tournament_archives`. Actor deletion may null its FK without losing
  identity/time. Legacy closed rows are retained without fabricated audit evidence.
  Archiving does not filter API list reads, change memberships or release final results.
- Invitation issue/rotation, redemption, entrant and membership creation hold a
  shared parent lock through commit and reject completed/archived parents.
  Identity-changing member/entrant updates also check the destination. Completion
  never locks/revokes invitation rows, avoiding a parent-to-invitation lock cycle.
  Closed round plans cannot be inserted into, moved or deleted. Existing member
  reads, invitation revocation and independent final visibility remain available.
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
- Self-service profile reads/writes resolve only the session user at
  `/api/me/profile`; no caller-supplied user/player identity or global authority
  is accepted. Details updates lock session/user and linked player, compare the
  account version plus player timestamp, then update account/player names and
  append handicap history with actor, timestamp, and the server-owned
  “Egen profilendring” description atomically. The details request has no reason
  field; administrator tournament-handicap corrections retain explicit reasons. Creating an explicitly
  requested player link never enrolls existing tournaments; inactive player
  handicaps cannot be self-edited. Existing tournament handicaps and all round
  snapshots/results are untouched. Name changes use existing identity joins and
  therefore appear in historical views; targeted tournament SSE signals follow
  commit.
- Schema 23 adds monotonically increasing account profile versions and credential
  generations. A narrow user trigger advances the profile version on updates
  and the credential generation only when the password hash changes. Sessions
  capture the current generation at creation. Central session reads/locks and
  supplemental completion/archive/visibility guards require equality, preserving
  existing lifecycle error contracts. Password changes invalidate all devices
  without locking other session rows. Bounded Argon2 verification/hashing occurs
  outside transactions; current password hash/generation and version are checked
  again under exclusive session/user locks, with wall-clock expiry after waits.
  Login rechecks its verified hash, username and generation under a user share
  lock through session insertion, preventing an old verification from minting a
  newly valid session after password or username changes.
- Schema 24 introduces password-recovery grants, append-only outcome audits, and
  an authority-change ledger. Recovery tokens are independent 256-bit capabilities
  stored only as SHA-256 hashes, valid for 30 minutes and one redemption. One
  unterminated grant per account is enforced by a partial unique index; replacement,
  revocation and redemption are terminal. Grant identity is immutable and audit
  foreign keys intentionally retain referenced accounts, players and tournaments.
- Recovery HTTP mapping lives in `api/password_recovery`, locking and persistence
  in `repositories/password_recovery`, token generation/comparison in the domain,
  and trusted-origin parsing in configuration. Administrator requests resolve the
  target account from the exact tournament player; no caller-supplied account ID
  or inferred email identity is accepted. The issuer needs exact admin membership
  and current-password confirmation. Targets must be linked active ordinary
  participants with membership and no global or any-tournament admin role; self
  recovery is excluded. Administrator accounts use the deployment operator CLI.
- Admin operations lock the issuer session before account rows in UUID order,
  then the player, memberships and entrant. Public preview/redemption use the
  same account/eligibility lock order. Account `FOR UPDATE` blocks new or moved
  membership foreign keys; existing membership rows are share-locked. Authority
  and wall-clock expiry are rechecked after waits. The append-only authority
  ledger invalidates administrator grants after relevant role, link, membership
  or entrant changes even if eligibility is restored. Ledger subject UUIDs are
  retained snapshots without account foreign keys, avoiding inverse account-lock
  acquisition from authorization triggers.
- Successful recovery changes only the password hash and atomically consumes the
  grant. Schema 23's credential generation invalidates all previous sessions and
  other grants; username-only edits retain grants. Password hashing stays outside
  transactions. Recovery never creates a session or changes cookies, preserving
  an unrelated account already signed in on the recipient's browser.
- Operator provenance is an explicit grant kind, permitted only to the actual
  recovery-table owner or database superuser by an invoker trigger. It cannot be
  forged through a caller setting or broad runtime DML/function grants. Runtime
  startup also rejects membership in the recovery-table owner role. The packaged
  `password-recovery` CLI requires an exact account UUID and audit reason and
  writes a link exclusively to a new mode-0600 file; it never accepts a new
  password. Operator grants support administrator and unlinked accounts without
  assigning a web role or bypassing the same token/credential-generation checks.
- `RESET_PASSWORD_ORIGIN` is explicit HTTPS configuration (loopback HTTP only in
  development), never derived from request Host headers. Links carry the secret
  in a fragment; the public page captures it in memory and removes it from browser
  history. Preview/redeem send it only in POST bodies. Recovery responses use
  `private, no-store` and `no-referrer`; recovery requests suppress referrers.
  Tokens/passwords are excluded from query keys, persisted browser state and
  ordinary logs. PostgreSQL production settings suppress bind values and error
  details while retaining query timings and basic error messages.
- Recovery UI is a focused roster disclosure and an independent public route.
  Permission/roster refresh or errors unmount the disclosure and discard receipts.
  Inactive secret-bearing mutations are removed immediately; late receipts check
  mount and user/CSRF identity. Reset completion refreshes session state without
  signing out an unrelated account. Auth reads reject canceled responses and
  preserve a newer user/CSRF identity published while the HTTP read was pending.
- Profile caches remain user-rooted. Their reads reject responses after session
  replacement; mutation reconciliation compares both user ID and CSRF token.
  Successful mutations reconcile even after page departure, while local feedback
  remains mount-bound. Password success leaves the protected route before null
  identity publication so its sign-in confirmation survives normal route guards.
  Credential-bearing inactive mutation state is removed immediately.
- Signed-in creation uses the same pure tournament-plan normalization and
  transactional draft-plan insertion, but never creates or replaces an account,
  session or invitation. Any authenticated account can create an independent
  trip and receives admin membership only there. Its active linked player, if
  present, is enrolled with a locked current-handicap snapshot; otherwise the
  creator is a non-playing administrator. Prior tournaments are untouched.
- Schema 22 stores a per-user request UUID, normalized-plan SHA-256 hash and
  resulting tournament UUID. Exclusive session/user locks serialize requests
  across sessions of one account; current activity/handicap is read under a
  player lock and wall-clock session expiry is rechecked after waits. Exact
  retries reauthorize current admin membership and return the same ID; changed
  payloads conflict. Stable structural normalization precedes receipt lookup,
  while only a new creation rejects an end date before today's UTC date.
  Receipt FKs cascade with user/tournament deletion; receipts are retry state,
  not historical score or ownership evidence.
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
  Creator onboarding, signed-in creation and invitation registration/acceptance are the only HTTP
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
- Schema 21 adds an explicit `course_presets` registry over finalized manual
  revisions for administrator-supplied layouts. Its exact-admin read holds the
  tournament membership lock through assembly in a repeatable-read transaction;
  ordinary private round revisions are never discovered as presets. The picker
  sends inspected facts through the existing validated manual save boundary,
  creating an independent revision rather than attaching the registry's IDs.
  Presets have no provider provenance or general-purpose editing API.
- Manual course configuration creates one round-specific immutable course/tee
  revision, not a shared mutable course or multi-tee catalog. Round opening
  derives and freezes each eligible owner's Course and Playing Handicap from
  that selected tee; net scoring allocates received strokes from the preserved
  Playing Handicap through the revision's unique hole stroke indexes.
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
- Member round details consume that same decoded aggregate and canonical
  user/round pairings query as management. Flight schedules are presentation
  facts only; team cards and legacy individual groups remain distinct. Round
  and aggregate status/format disagreements require refresh, and failed reads
  suppress retained detail data. Scorecard navigation carries only the exact
  tournament and round; the scoring workspace resolves owner/access itself.
- Flight progress joins that frozen pairing membership to the canonical
  user/round completion projection, without persisting a derived cache. Individual
  snapshot owners map by player ID; each team card maps only when all preserved
  members belong to one flight. Missing historical flight mappings fail explicitly.
  Aggregates sum projected hole counts once per owner. Front-nine projections
  never derive complete/confirmed counts from totals or round lifecycle status.
  Reusing the completion key retains synchronous projection clearing on visibility
  signals, stream opening/reconnection and errors.
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
- Migration 0027 adds server-controlled positive bigint score revisions, serialized
  as canonical decimal strings only in authorized mutation/scoring DTOs. A narrow
  trigger increments revisions on actual stroke changes through every write path,
  rejects forged revisions and leaves true no-ops unchanged. Existing scores start
  at revision 1 without modifying their score, timestamps, audits or confirmation.
- Conditional hole delivery adds an account/request-ID receipt boundary alongside
  the compatible legacy PUT. The repository locks the round and reauthorizes the
  current session, membership and tagged owner before inspecting any receipt.
  An unseen operation compares explicit absence or exact score ID/revision before
  the normal no-op/write helper. Score and immutable receipt commit together;
  cross-round request-ID collisions roll back the losing transaction completely.
  Wall-clock session expiry is checked after waits and again before commit.
- A receipt acknowledges past application, never current score state. It binds
  normalized target, expected version and strokes to the account/request ID and
  supplies the applied score ID/revision for safe successors. Reusing an ID with
  different content conflicts; duplicate delivery adds no score audit or SSE.
  Receipt deletion is allowed only through intentional parent deletion, with no
  timed cleanup that could let a delayed retry become a new write. Member/public
  result projections never expose revisions or receipts.
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
- Both scorecard hole contracts expose required signed `handicap_strokes`, even
  before score entry. Domain assembly uses the existing stroke-index allocator
  with the preserved owner playing handicap and full configured round length;
  the same value is subtracted from gross for net. Read redaction copies this
  allocation only for visible holes, without reallocating over the visible prefix.
  The shared React summary renders the decoded value without a client formula.
- Transaction-local lifecycle settings route application writes through the
  expected integrity paths; they are not an authorization boundary. The
  least-privilege runtime role prevents ordinary API connections from using
  schema-management authority, while repository authorization and PostgreSQL
  integrity triggers remain the application/data boundaries.
- Round leaderboards calculate live gross/net score-to-par from the holes actually
  scored. Tournament leaderboards independently select each player's displayed
  best N from completed/locked history plus the visible scored portion of the
  deterministic highest-numbered open round. Contributions retain tagged owner,
  provisional state, and hole progress and are attributed through frozen
  membership for that exact round. Completed-only qualification count ranks
  before selected score-to-par and alone controls eligibility; separate gross
  and net routes never use the other metric as a hidden tie-break. Sporting ties
  compare only entries with selected contributions; an unstarted/unranked entry
  cannot make an even-par provisional entry tied. Optional `final_round_score`
  compares entire equal-primary groups only when every entry is eligible, has no
  selected provisional contribution, and has a complete visible non-provisional
  contribution from the configured final scheduled round. This contribution may
  be excluded from best-N. Missing comparable data retains the whole shared group;
  equal final values retain competition positions. Repositories pass the parent
  round count explicitly; assembly resolves that exact round rather than the
  greatest loaded round. Comparison consumes visible attributed contributions,
  never hidden raw facts. Required policy/final-number response fields and nullable
  entry `tie_break_score_to_par` explain comparisons, including residual ties.
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
- Public result sharing is an explicit capability boundary, separate from member
  authorization. Schema 0026 stores independent 256-bit token hashes, a fixed
  30-day lifetime, immutable grant identity and derived immutable issue/replace/
  revoke audits. A partial unique index permits one unrevoked grant per tournament;
  replacement also terminates an expired previous grant. Composite audit foreign
  keys retain tournament/grant consistency. Ordinary grant/audit deletion is
  rejected; intentional parent tournament deletion cascades both. Audit actor UUIDs
  are preserved snapshots rather than user foreign keys. Grants belong to the
  tournament and survive issuer demotion; management rechecks current exact-admin
  session, membership and credential generation under locks.
- Public reads hold the grant `FOR SHARE` through repeatable-read fact loading and
  assembly, with wall-clock expiry checks after grant waits and after loading.
  Queued readers whose grant changed under repeatable read fail unavailable.
  Rotation/revocation serialize with reads. The fact loader accepts explicit
  Internal/Member/Public projection contexts; Public always uses the non-admin
  visibility rule, regardless of ambient cookies. Private membership locks and
  existing unrestricted internal callers keep their original contracts.
- A dedicated public domain projection allowlists standings summaries after
  visibility-aware scoring/ranking. It excludes global player/owner/account IDs,
  roster/team details, contributions and hole scores. The anonymous API validates
  a body-carried capability and never extracts session authority. Strict public
  frontend decoders reject private response shapes; public rows have no private
  drill-down links. Invalid, expired and revoked capabilities share unavailable
  responses. Endpoint body limits, trusted-client throttles, no-store, no-referrer
  and noindex/nofollow headers apply independently of the authenticated workspace.
- The standalone shared-results route retains the reusable token fragment, keeps
  it out of persistent storage/query keys and creates a new QueryClient for each
  nonsecret visit identity. Native hash changes (including removal), router visits
  and same-grant token changes cannot reuse prior authorization. Unmount clears
  the visit cache; abort signals reject late responses. Visible-only 15-second
  refresh, page return, metric changes and explicit retry clear previous metric
  snapshots. Failures hide rows, terminal unavailable stops polling and bounded
  expiry timers avoid JavaScript's maximum-delay overflow. Public reads omit
  cookies and never subscribe to private SSE. Admin receipt state is isolated by
  user, CSRF and tournament identity; metadata alone enters the private cache.
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
  Ordinary `score` events invalidate only that user's leaderboard, round
  completion-validation, and read/scoring-card query families. Score saves and
  confirmations do not mutate setup or access metadata. Structural events retain
  broad reconciliation; disconnect, reconnect, and visibility handling retain
  synchronous projection clearing. Tournament leaderboard loading still fetches
  rounds before validating the response, including after score-only invalidation.
- Shared tournament subscriptions listen for visible-page return, persisted
  `pageshow`, and browser `online` events. Return restarts a stopped/retrying native
  EventSource while retaining a healthy stream; concurrent return events do not
  interrupt a replacement already connecting. Superseded sources cannot dispatch
  invalidations, and the final unsubscribe removes lifecycle listeners.
  A client-only `resume` signal deduplicates session revalidation per user/client
  before private-query refresh, and checks the resulting identity again. Ordinary
  SSE events still do not invalidate authentication. No timer or polling loop is
  introduced, and a return never itself grants score access.
- Target-bearing frontend DTOs are decoded against the requested tournament,
  round, player, owner, metric, invitation predecessor, and course-configuration
  identities before cache insertion. Roster, round, team, pairing, invitation,
  and leaderboard collections also reject duplicate or internally incoherent
  identities. Runtime validation is a fail-closed cache boundary, not an
  authorization substitute.
- A non-null mandatory-round identity is also composed with the exact decoded
  tournament round collection before settings or leaderboard data enters the
  query cache. Unknown or cross-target round identities fail closed. Tournament
  tie metadata is validated against the exact visible scheduled final and every
  member of its primary-score group before cache insertion. React renders server
  positions without sorting or recalculating sporting ranks. The settings editor
  remounts on user/session/tournament change and refreshes authoritative scoped
  queries after writes; a late response cannot populate a replacement workspace.
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
- Administrator round lifecycle controls compose the exact management membership
  gate with one URL-selected round and the existing readiness endpoints. Opening,
  completion and locking use CSRF-protected typed mutations and validate returned
  round/tournament identity plus resulting status. The frontend does not infer
  readiness from card counts or synthesize a lifecycle transition. Completion
  readiness reuses the scoring route's identity-scoped key, with full projection
  and matching status required before an action becomes available.
- The organizer summary mounts only inside the exact-admin management workspace.
  It reuses the canonical per-user pairing/completion validation keys, with one
  applicable readiness read per non-locked round (up to the existing 30-round
  creation limit). There is no summary endpoint or duplicate cache. The canonical
  round list selects which read applies; completion status/identity and full
  projection must match before any counts or suggestions render. Fetching,
  paused/error reads, or authority refresh suppress suggestions. Ready flags
  remain server-owned; counting complete-unconfirmed tagged owners only describes
  navigation tasks and never authorizes a mutation. Shared live/visibility
  invalidation therefore reaches the summary alongside lifecycle/scoring views.
  URL navigation keys also reactivate exact linked course/pairing editors when
  the destination is revisited after manual collapse; existing drafts remain
  mounted and lifecycle confirmations remain owned by their current controls.
- Lifecycle reconciliation invalidates only the affected round, tournament,
  membership/list and gross/net leaderboard consumers. It replaces pre-outcome
  reads and inspects current active query state after invalidation, so a later SSE
  refetch replacing its request is not mistaken for a failed read. Loading,
  authority and readiness gates hold pending actions. Late mutation responses
  never insert private cache data after their workspace is unmounted. Confirmation
  state is tied to action and readiness version, while mounted manual course
  drafts remain disabled after opening instead of being discarded.
- Tournament completion uses the same exact-admin management gate and a separate
  focused panel/hook. Its client readiness requires the complete configured round
  identity/number set, all locked, and an active tournament; the backend remains
  authoritative. Confirmation is bound to membership, tournament and round read
  versions and is permanently discarded during refresh. The typed POST validates
  both returned identity and completed status. Every mutation outcome invalidates
  existing user-scoped tournament, membership/list, invitation and tournament
  leaderboard queries without inserting response data. Duplicate submissions,
  unresolved reads and failed reconciliation block further completion. Independent
  final visibility and historical score ownership remain unchanged.
- Archive controls compose the same keyed exact-admin management gate with a
  completed-only panel/hook. Confirmation binds to authoritative read versions and
  expires permanently during refresh. The typed archive response must match the
  requested identity and archived status. Completion and archive share one private
  query reconciliation function, preserving pre-outcome read cancellation, newer
  SSE refetch handling and no late mutation-response insertion.
- The tournament list filters the unmodified user-scoped membership collection
  locally, using URL view selection. Current includes every non-archived status;
  archived/all views retain direct private history links. Failed authoritative
  reads hide cached cards. No per-view cache or global SSE feed is introduced:
  return/focus refresh stale data under the shared 20-second freshness window;
  manual refresh always fetches. The target-scoped management subscription
  continues live reconciliation.
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
  Bare `/score` is the ordinary resume entry; it forces fresh reads on the same
  canonical keys and waits for successful post-mount fetches before replacing
  the URL with the first missing hole or a complete writable card's summary.
  Explicit URL navigation never applies this resume decision. A narrow transient
  provider remembers only validated tournament/round/tagged-owner IDs during the
  mounted application session. It clears that state synchronously on identity
  changes without remounting the router or discarding onboarding/invitation
  success receipts. It stores no card, permission, score intent, or server data.
- Visibility events synchronously clear role-projected leaderboard, completion,
  history, drilldown, and actor-free scorecard query state before authoritative
  refetch. An EventSource error performs the same transition without refetching
  by itself; `open` repeats it and refreshes after reconnection. Browser return or
  explicit reconnect can separately revalidate the session and refresh HTTP reads.
  Writable `/scoring` queries are deliberately excluded. Browser time never
  changes authorization or locally reveals cached facts, and restricted hole
  URLs are canonicalized to the visible prefix.
- Temporary completion clearing retains only the exact cached authorized scoring
  card in an editable round, with generic owner text instead of cleared progress
  metadata. Local hole edits and navigation within that card remain available;
  confirmation and completion-dependent card selection wait for recovery. Actual
  authorization denial or locking removes write access. Pending local operations
  remain discoverable separately and cannot restore private server caches.
- Ordinary hole intent lives in account-scoped IndexedDB, separate from TanStack
  Query's authoritative server state. Persistence must commit before an edit is
  described as saved on this device. Each hole has an immutable head operation,
  latest desired successor, generation token and bounded cross-tab delivery lease.
  An acknowledgment bases a successor on the predecessor's applied revision,
  never on a newly observed unrelated score. Canonical card refetches supply
  current server values; acknowledgment data never replaces a scorecard.
- The private workspace queue runner is fenced to current account/CSRF identity,
  retries with bounded requests/backoff and wakes on reconnect/page return.
  Logout pauses delivery while preserving same-account pending edits. Other
  accounts cannot display or replay them. Broadcast notifications contain no
  score payloads; IndexedDB transactions arbitrate claims and exact-generation
  discard/resolution. Conflict review fetches an authorized current score and
  requires an explicit choice; choosing local creates a new conditional operation.
- Navigation guards cover uncommitted device writes and confirmation, while
  durable queued edits survive navigation/reload. Confirmation stays online-only,
  with an empty-card-queue lease and fresh authorized read before the existing
  server-current-card POST. No service worker, full-card persistence, cold offline
  app shell, queued confirmation or background synchronization is introduced.
- Handicap and net-score calculations remain backend-owned. Pending gross input
  is visible immediately, but net output is shown only after decoded server
  verification.

## API inventory

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
| `POST` | `/api/tournaments/{tournament_id}/complete` | Exact-admin explicit completion after every configured round is locked |
| `POST` | `/api/tournaments/{tournament_id}/archive` | Exact-admin explicit archive of a completed tournament; preserves private history |
| `GET` | `/api/rounds/{round_id}/leaderboards/gross` | Retrieve the live gross round leaderboard |
| `GET` | `/api/rounds/{round_id}/leaderboards/net` | Retrieve the live net round leaderboard |
| `PUT` | `/api/rounds/{round_id}/scores` | Save or correct one hole score (compatible legacy write) |
| `PUT` | `/api/rounds/{round_id}/scores/conditional` | Reauthorize and conditionally deliver an idempotent hole operation |
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
| `POST` | `/api/auth/password-recovery/{grant_id}/preview` | Non-consuming, private reset capability validation |
| `POST` | `/api/auth/password-recovery/{grant_id}/redeem` | Consume capability and replace password; invalidate target sessions without cookie changes |
| `POST` | `/api/tournaments/{tournament_id}/players/{player_id}/password-recovery` | Current-password-confirmed admin issue/replacement for an eligible ordinary player |
| `POST` | `/api/tournaments/{tournament_id}/players/{player_id}/password-recovery/revoke` | Revoke outstanding grants for the player in this tournament context |
| `GET` | `/api/me/tournaments` | List the session user's tournament memberships and player links |
| `GET` | `/api/me/profile` | Private self-only account and linked player details with optimistic versions |
| `PUT` | `/api/me/profile` | CSRF-protected own name/current handicap update; server-owned audit description and no tournament rewrites |
| `POST` | `/api/me/profile/username` | Current-password-confirmed canonical username change; sessions retained |
| `POST` | `/api/me/profile/password` | Current-password-confirmed password change; invalidate every session and clear cookie |
| `PATCH` | `/api/tournaments/{tournament_id}/counted-rounds` | Atomically update best-N, optional mandatory round and overall tie-break policy before tournament start |
| `GET`, `POST` | `/api/tournaments/{tournament_id}/result-share` | Exact-admin latest link metadata and CSRF-protected issue/replacement with expected grant identity |
| `DELETE` | `/api/tournaments/{tournament_id}/result-share/{grant_id}` | Exact-admin CSRF-protected revocation of the current result link |
| `POST` | `/api/public/results/{grant_id}` | Capability-authorized limited gross/net standings with non-admin visibility |
| `GET` | `/api/tournaments/{tournament_id}/course-catalog` | Search the bundled curated course shortlist as a tournament admin |
| `GET` | `/api/tournaments/{tournament_id}/course-presets` | Read only registered immutable supplied layouts as an exact tournament admin; private, no-store |
| `GET` | `/api/tournaments/{tournament_id}/course-provider/courses/{provider_course_id}` | Retrieve normalized provider tee and hole detail as a tournament admin |
| `POST` | `/api/onboarding/tournaments` | Atomically create a first-time creator, draft tournament plan, invitation, and session |
| `POST` | `/api/tournaments` | Authenticated CSRF-protected independent draft creation with a per-account retry key; no account/session replacement |
| `POST` | `/api/invitations/{invitation_id}/preview` | Preview minimal tournament data for an authenticated invitation token |
| `POST` | `/api/invitations/{invitation_id}/register` | Atomically register and join a new player account |
| `POST` | `/api/invitations/{invitation_id}/accept` | Join the exact session-linked player idempotently |
| `GET`, `POST` | `/api/tournaments/{tournament_id}/invitations` | List metadata or issue a tournament-admin invitation |
| `POST` | `/api/tournaments/{tournament_id}/invitations/{invitation_id}/rotate` | Revoke and replace one active invitation |
| `DELETE` | `/api/tournaments/{tournament_id}/invitations/{invitation_id}` | Idempotently revoke an invitation |

Errors consistently use `{ "error": { "code": "...", "message": "..." } }`.

## Deferred decisions

- Public scorecard access beyond the implemented overall result-sharing link.
- Regional alternatives to the implemented WHS course-handicap conversion.
- Scramble formulas beyond the initial configurable 35%/15% implementation.
- Additional tournament tie-break policies and configurable individual-round ties.
- Cold offline launch, offline app-shell delivery and background synchronization.
- General-purpose editable course and multi-tee library behavior beyond supplied
  immutable presets, including whether the UI should
  show explicit per-hole received-stroke badges.

## Planned four-ball stroke-play contract

**Status: defined, not implemented.** The user approved defining four-ball before
Stableford, match play and the later application reviews, and explicitly chose to
credit the side's round result to both partners in overall standings. The contract
below sets the first variant and its design defaults; it does not change current
runtime behavior. Implementation remains separately approved. Rule references
were checked on 2026-09-13.

### Rules basis and initial variant

Four-ball uses two partners, each playing their own ball. The side takes the lower
eligible score on each hole; in handicap play, the gross and net winners can be
different partners. At least one attributed gross score must be recorded for each
side-hole; the other partner need not hole out. Either partner's equal score can
count, and only one partner needs to certify the card. These are rules of the
format, not Golfside's individual tournament attribution policy.
[R&A Rule 23.1–23.4](https://www.randa.org/rog/the-rules-of-golf/rule-23).

The initial variant is **18-hole**, two-player four-ball **stroke play**, with
administrator-assigned, round-specific teams and one shared course/tee layout.
Four-ball match
play, four-ball Stableford, larger teams, per-player tees and automatic team
formation are outside this contract. Both registered partners remain on the
frozen round team even if only one supplies valid scores. Opening requires every
eligible entrant to belong to exactly one complete two-player team, with both
partners in the same valid flight. Odd/unassigned rosters fail readiness; the app
must not invent a partner or change other rounds' teams.

Nine-hole four-ball is deferred explicitly. The current code accepts shorter
layouts but calculates handicap without a round-length parameter; it cannot be
assumed to provide the Norwegian nine-hole policy. NGF rule 6.1b allocates the
18-hole handicap over the full card, then uses the strokes belonging to the nine
played holes. Supporting that requires an explicit layout/handicap contract and
preserved inputs; do not reuse the existing generic hole-count allocator as a
substitute. Reject non-18-hole four-ball configuration at validation/opening.
[NGF Handicapreglene 2024, rule 6.1b, printed page 55](https://www.golfforbundet.no/files/documents/handicapreglene-whs-2024.pdf).

### Score ownership and derived results

Separate three concepts in the format policy:

| Responsibility | Four-ball owner |
| --- | --- |
| Entered hole result, revision, audit and delivery receipt | Individual player |
| Round competition result and scorecard confirmation | Preserved round team |
| Individual overall contribution | The derived side result, once for each frozen partner |

The server derives a side-hole from its partners' eligible numeric entries:
`gross = min(partner gross)` and `net = min(partner gross - partner hole strokes)`.
Each metric selects independently. Side totals are sums of those hole results;
there is no single side Playing Handicap to subtract from a side gross total.
Do not store derived winning holes as extra team-owned score mutations, or count
both partners' raw totals as additional team results.

Preserve all entered scores and the source player for each selected metric. Equal
winners remain equal; use stable player identity only for deterministic display,
never as an additional sporting tie-break. Private cards can identify every equal
winner. The existing single-owner/single-handicap DTO must gain an explicit
four-ball representation; do not manufacture a zero team handicap or send a team
identifier to the ordinary player-score mutation path.

### Handicap policy

Proposed default allowance: **85% per player**, configurable through the existing
0–100% draft-round allowance setting and frozen at opening. Apply it once to each
uncapped, unrounded Course Handicap, then round to the nearest integer with exact
halves toward positive infinity, including signed plus handicaps. Preserve the
per-player calculation inputs, policy and final Playing Handicap in the round
snapshot. Handicaps disabled means zero strokes for both players.

This follows the recommended four-ball stroke-play allowance and avoids double
rounding. National competition terms can specify allowances; 85% is a default,
not a claim that every competition must use it.
[NGF Handicapreglene 2024, rule 6.2a and appendix C, printed pages 55 and 75–76](https://www.golfforbundet.no/files/documents/handicapreglene-whs-2024.pdf),
[R&A Appendix C](https://www.randa.org/en/roh/appendices/appendix-c).

Use each player's signed stroke-index allocation over the full configured round,
including strokes given back on the highest stroke indexes for plus handicaps.
A restricted front-nine projection must not recalculate that allocation. Keep
existing formats' historical calculations unchanged; four-ball must not inherit
the scramble index cap/team formula or silently change the individual format's
existing negative-half rounding.

### Unentered, scored and no-score holes

Model player input as three distinct states: never entered, numeric gross score,
and explicitly no valid score (for example, picked up). Numeric entries keep the
existing 1–20 range and include applicable penalty strokes. No-score is not zero,
net par, an estimated total or a Stableford point value. A blank partner does not
prevent a side-hole from having a provisional result when the other partner has
a valid numeric score. At confirmation, the scorer explicitly accepts that any
remaining blank partner entries provide no counting score on those holes; do not
write synthetic scores for them.

Permit correcting a mistakenly entered numeric score to no-score through a
separate explicit, auditable action, with confirmation before submission. Retain
its identity, revision and history as a nonnumeric state; do not physically delete
and recreate a score row. Restoring a numeric score advances the same identity's
revision. This deliberately extends the current numeric-only score contract only
for four-ball, and needs its own migration/transport/decoder/queue coverage.
Other formats must reject no-score input until their own contracts support it.

One numeric partner result makes the side-hole scorable; neither partner having
one leaves that hole without a result. Progress counts side-holes, never the sum
of player entries. Live totals are provisional and cover only holes with a side
result. Confirmation requires a numeric side result on every configured hole;
individual partner cards may remain incomplete. A known picked-up hole with no
valid partner result cannot be confirmed. Formal disqualification/withdrawal
adjudication and finishing a round with such unresolved sides are outside the
initial workflow: show the missing result and block completion, rather than
fabricating a score or declaring a sporting penalty automatically.

### Authority, confirmation and offline delivery

Keep the existing session, tournament membership, flight and round authorization
boundaries. A shared side-card UI may enter either partner's own score only when
that exact player-card write is authorized. Side confirmation requires write
authority over both partner cards; placing the partners in one flight makes this
compatible with the ordinary scorer workflow. This remains the application's
score confirmation, not an assertion that it provides official marker signatures.

Confirmation certifies the current server side-card assembled from both players,
including nonwinning and missing entries. It remains online-only and uses a fresh
side-card read. Any actual change to either partner's input invalidates the side
confirmation, even if the selected gross/net total happens to stay the same.
True no-op writes and repeated delivery receipts do not invalidate it again.
Serialize these actions with round completion/locking and existing authority
checks. Completed rounds remain explicitly correctable; locked rounds reject
ordinary writes and the new no-score action.

On this account/device, unresolved edits or post-delivery verification for either
partner block side confirmation. Extend the local confirmation lease to cover
both underlying player cards atomically; another tab must not enqueue through
the other partner while confirmation is in flight. Other devices remain subject
to current server-card confirmation semantics and server locking, as today.

Queue each player's hole independently. Numeric/no-score transitions use the
same immutable request, expected-state revision, receipt replay, account isolation
and exact-generation review guarantees as current offline scoring. A retained
no-score record is a present version, never expected absence. A partner's
independent edit updates the derived side result but is not a conflict on the
other player's score. A conflicting edit to the same player/hole presents both
states, including a clear no-score label, and requires the scorer's choice.
No queued confirmations, lifecycle changes or cold offline launch are added.

### Standings, history and visibility

**User decision:** Credit the derived side round result to each frozen partner
once in individual overall standings, matching existing team-round attribution.
Do not also count the partners' separate raw cards or require either to complete
an individual 18-hole score. Preserve the existing best-N,
mandatory-round, completed qualification and highest-numbered-open provisional
rules. Select gross/net independently. Four-ball side score-to-par can join the
existing stroke-based contributions from individual play, scramble and foursomes;
label it as a team contribution rather than an individual's own performance.
Assigning that benefit equally is a Golfside tournament rule, not an R&A rule or
a normalization guarantee between formats. Stableford has its own explicitly selected conversion in the contract below;
match-play aggregation remains separately undecided.

Round ties keep shared competition positions. The existing optional overall
final-round tie-break consumes the attributed complete, visible side result, even
when outside best-N; partners with the same final side result remain tied if all
other compared values are equal. No playoff or extra countback policy is added.
History must retain the round team and link to its derived side card after later
team changes, with partner inputs and independent gross/net winners available to
authorized members.

Apply existing hidden-final visibility before deriving any visible side total,
progress or winner explanation. Omit hidden completed finals before best-N and
tie comparisons. Public links keep their existing summary-only gross/net scope:
no raw player holes, winner identity, handicap, no-score reason, confirmation or
account data is added. Changing only hidden inputs must leave every permitted
result projection unchanged.

### Acceptance examples and implementation boundary

For the examples below, received strokes are the already-calculated per-hole
allocations, not a percentage applied to the hole score.

| Hole/par | A gross / received | B gross / received | Side gross | Side net |
| --- | --- | --- | --- | --- |
| 1 / 4 | 4 / 0 | 5 / 2 | 4 (A) | 3 (B) |
| 2 / 4 | 5 / 1 | 4 / 0 | 4 (B) | 4 (both) |
| 3 / 4 | no score | 6 / 2 | 6 (B) | 4 (B) |

The three-hole subtotal is gross 14 (+2) and net 11 (−1), with three side-holes
scored. It is not a complete 18-hole card. If both players have no score on hole 3,
progress is two holes and confirmation is unavailable. On an index-18 hole, a
player with Playing Handicap −2 giving one stroke back and scoring 4 has net 5.

Allowance examples: unrounded Course Handicap 9.6 at 85% gives 8.16, rounded to 8
(intermediate rounding to 10 would incorrectly produce 9). Course Handicap 20 at
85% gives 17. Signed Course Handicap −10 at 85% gives −8.5, rounded to −8 (shown
as plus 8). With handicaps disabled, the gross/net side results coincide.

Complete-round mixed-format example: all three rounds below are completed,
confirmed and visible, each has par 72, and the tournament counts the best two
with no mandatory round. Values are gross/net score-to-par.

| Round | Format and preserved partners | A contribution | B contribution |
| --- | --- | --- | --- |
| 1 | Individual stroke play | +8 / +2 | +8 / +2 |
| 2 | Four-ball, A + B together | −2 / −6 | −2 / −6 |
| 3 | Scramble, A + C and B + D | +1 / −1 | +5 / 0 |

Both A and B receive round 2 once. A's selected totals are gross −1 and net −7;
B's are gross +3 and net −6. Their later partners do not change round 2 history.
If only one round counts, round 2 supplies both partners' best gross/net result.
If that one slot instead belongs to mandatory round 3, round 3 supplies their
result even though round 2 is better. A missing mandatory result never gets an
extra optional replacement. An open round remains provisional under the existing
qualification and highest-numbered-open selection rules.

Separate final-round tie example: four players each have a complete earlier best
round of gross −3; best-N is 1. The final scheduled round is completed four-ball:
A + B score even par and C + D score +1. None of these final results enters best-N.
With `shared_positions`, all four retain the shared position. With
`final_round_score`, A and B share first and C and D share third. An equal final
side result retains shared places; hidden or otherwise incomparable final results
must not break the original tied group. The net view runs the same policy using
its own net contributions, never these gross values.

A planned full implementation must cover:

- A closed format policy separating input ownership, competition teams,
  aggregation, confirmation and snapshot treatment; draft creation, pairing and
  opening must use it consistently. All existing formats keep their behavior.
- Player score/no-score storage, positive revisions, immutable receipts and audits;
  derived side-card APIs, per-partner handicaps and completion/confirmation guards.
  Unknown formats remain rejected until the entire path is available.
- Gross/net side results, approved individual attribution, private history and
  visibility-safe public standings, plus mobile score entry for both partners.
- Fresh and populated-schema migration checks; pure arithmetic/aggregation tests;
  PostgreSQL authority, ownership, no-score ABA, confirmation and lock races;
  real Chrome at 320/390/1280px with offline edits on either partner, both conflict
  choices, unavailable storage, hidden scores, incomplete cards and long names.
- Best-N/final-round examples with changing partners and mixed existing formats;
  snapshot invariance after profile changes; reject unsupported nine-hole
  configuration; no duplicate player contributions;
  read-only review and the complete affected validation ladders.

The first bounded implementation candidate is the pure domain foundation:
introduce four-ball-specific per-player allowance and side-hole aggregation types
and acceptance tests, without making the format selectable or changing existing
API/database behavior. Stop after the reviewed domain contract passes backend
checks. Subsequent persistence and UI slices must ship as one coherent supported
format; an isolated enum or arithmetic helper must not be advertised as playable.
Those implementation steps remain queued behind all three format definitions.

## Planned individual Stableford contract

**Status: defined, not implemented.** The user chose to include
Stableford in mixed-format overall standings using **36 minus points** for a
completed 18-hole round. This is a points-derived contribution, not an actual
stroke total. The first variant and design defaults below preserve the existing
formats; implementation remains a separate step. Sources checked 2026-09-13.

### Variant and scoring rules

The initial format is **18-hole individual Stableford**, with the configured hole
par as its fixed target, separate gross/net views and player-owned input/results.
No team membership is required or inferred. Existing flight setup and score
permissions apply. Modified Stableford, different target scores, team Stableford,
quota games, nine-hole play and per-player tees are excluded initially. The same
nine-hole layout/handicap limitation documented for four-ball applies; reject
non-18-hole configuration rather than claiming support through generic allocation.

Stableford awards points against a hole target; the most points wins. Handicap
strokes are applied before calculating net points. A hole not holed out returns
zero points. These format rules come from
[R&A Rule 21.1](https://www.randa.org/rog/the-rules-of-golf/rule-21).
The following is the fixed Golfside points formula for the first variant:

- Numeric gross score `g`, hole par `p`, signed received strokes `s`:
  `gross_points = max(0, 2 + p - g)`;
  `net_points = max(0, 2 + p - (g - s))`.
- Explicit no-score: zero points in **both** metrics, with no invented strokes.
- Unentered hole: unresolved; no points result or completed-hole credit yet.

| Score relative to target, after handicap in the net view | Points |
| --- | --- |
| Double bogey or worse | 0 |
| Bogey | 1 |
| Par | 2 |
| Birdie | 3 |
| Eagle | 4 |
| Three under | 5 |
| Four under | 6 |

The formula continues by one point per stroke for lower adjusted scores; do not
hard-code a six-point ceiling or clamp negative net strokes to zero. The table's
par/bogey/birdie mapping is also described by
[NGF's Stableford guidance](https://www.golfforbundet.no/spiller/regler/world-handicap-system/godkjente-handicaptellende-spilleformer).
Store actual numeric gross strokes, including applicable hole penalties; calculate
points on the server. Do not accept client-supplied points as authoritative.
Formal disqualification adjudication, handicap-index updates, handicap submission
and GolfBox integration are outside the format implementation.

### Handicap snapshots

Default allowance is **100%**, consistent with NGF's individual Stableford
recommendation. Retain the existing integer 0–100% draft configuration range and
freeze it with the course/tee, fixed tournament handicap and opening snapshots.
Use the uncapped unrounded Course Handicap for this 18-hole layout, apply the
allowance once and round to an integer with exact halves toward positive infinity.
As with the planned four-ball policy, keep existing formats' stored and calculated
behavior unchanged rather than globally replacing their rounding helper.
[NGF Handicapreglene 2024, rule 6.2 and appendix C](https://www.golfforbundet.no/files/documents/whs-handicapreglene-2024-%E2%80%93-ny-versjon-juni-2024.pdf),
[NGF handicap allowances](https://www.golfforbundet.no/spiller/regler/world-handicap-system/test).

Allocate the preserved signed Playing Handicap over all 18 stroke indexes before
any visibility filtering. Plus handicaps give strokes back on the highest indexes.
Disabled handicaps give `s = 0`, so numeric gross/net points coincide. Apply
handicaps per hole, not by adding the total handicap to a gross points sum: the
zero-point floor makes those calculations different. Competition points and
Golfside's overall conversion do not constitute handicap-adjusted gross scores.

### Hole state, completeness and corrections

Use the same explicit numeric/no-score input boundary planned for four-ball,
without importing four-ball's side-level completion rule. A player hole is
resolved when it has numeric strokes or an explicitly submitted no-score state.
An unentered hole remains distinct even though its currently displayed points
might otherwise look like zero. Confirmation must never silently turn missing
entries into picked-up holes.

A scorer may record a no-score result using **Plukket opp / ingen score**, or
correct a numeric entry to it with an explicit confirmation. The action represents
the scorer declaring a zero-point hole; it does not infer why the hole was not
finished or automatically adjudicate a rules breach. A gross zero-point outcome
may still earn net points if actually holed out; the UI must not advise picking
up merely because gross points are zero. Conversely, an explicit pickup awards
zero in both views even if a hypothetical completed score could have earned points.

Retain entry identity, positive revision, audit history and immutable receipts
through numeric/no-score transitions. No physical score deletion/recreation,
zero-stroke sentinel or fabricated net-double-bogey stroke value is permitted.
A subsequent correction back to numeric strokes is conditional on that retained
version. Existing formats that require numeric scores continue to reject no-score
input. Reuse shared machinery only where the semantics are identical.

Progress is the count of resolved holes, including explicit zero-point holes.
All 18 resolved holes make the card complete even with pickups or zero total
points. All 18 blank holes remain unstarted/unranked, and 17 resolved holes remain
incomplete. Numeric totals must not be mistaken for completion evidence. A card
of 18 explicitly resolved zero-point holes is complete with zero points and can
be confirmed; a blank card cannot. The UI does not automatically fill an unplayed
remainder when the scorer returns to or confirms a card.

Confirmation remains online-only, with an empty local queue for the player card,
finished verification and a fresh authorized read. Preserve current server-card
confirmation semantics and the cross-tab lease. Any actual input change invalidates
confirmation even if both old and new strokes earn zero points. True no-op writes
and duplicate receipt delivery do not. Open/completed correction and locked-round
rejection remain as today. Round completion requires all required player cards
resolved and confirmed, using the new state-aware validator rather than counting
only numeric score rows.

### Native points and overall contributions

A Stableford round leaderboard ranks the selected metric's points **descending**.
Equal points keep shared competition positions; hole progress may stabilize
presentation but must not become an undisclosed sporting tie-break. At least one
resolved hole establishes a provisional result, including zero points. Completely
unstarted players cannot create ties with players who have recorded zero-point
holes. No extra countback, playoff or tie policy is introduced.

**User-selected overall rule:** For a completed round, derive gross and net
contributions independently as `36 - gross_points` and `36 - net_points`. For a
permitted partial/open card, use `2 * resolved_visible_holes - visible_points`.
Do not use 36 on a partial card, count unentered holes as zero-point finishes, or
normalize a partial result to 18 holes. With no resolved visible holes there is
no contribution, not an even-par entry.

The equivalent is a tournament competition rule. Numeric holes worse than double
bogey in the selected metric contribute at most +2; an explicit no-score hole
also contributes +2. The underlying actual strokes, when known, remain unchanged.
This intentionally preserves Stableford's reduced impact from bad holes when
combined with uncapped stroke-play rounds. Explain that choice in configuration
and contribution views; it does not make formats statistically equivalent.

Other supported stroke-based formats retain their existing gross/net score-to-par
contributions, including the agreed shared four-ball side result. Select best-N
using the **lowest comparable contribution** for each metric. Preserve mandatory
slots, completed-only qualification, highest-numbered-open provisional selection
and one contribution per round/player. Do not recalculate older rounds as
Stableford or add raw points directly to strokes.

Overall ties use those same comparable totals. The optional final-round policy
compares a completed visible final Stableford contribution on the selected metric;
this is equivalent to preferring more final-round points even if that final is
outside best-N. Equal final contributions remain shared, and incomplete/hidden
or otherwise incomparable final groups retain their original shared places.
All-Stableford tournaments may use the same overall equivalent: for the same N
completed selected rounds, `36*N - total_points` preserves points ordering. The
round page remains the native points display; mixed overall totals are never
labelled as a sum of points or actual gross/net strokes.

### Result types, UI and private/public projections

The current actual-stroke contracts cannot safely carry a Stableford equivalent
in fields named `gross_total`, `net_total` or `total`. Introduce explicit typed
result/value representations for actual strokes, Stableford points and the
comparison contribution, aligned across domain, API, runtime decoders and UI.
The transport design must preserve existing stroke-only clients/contracts or
version an intentional change; do not weaken decoders to accept inconsistent
old fields or disguise `par + equivalent` as strokes.

Private Stableford cards show original numeric entries, explicit no-score states,
per-hole received strokes, gross/net points and resolved progress. An actual full
stroke total is available only when every hole has numeric input; otherwise it
is absent. Any optional numeric subtotal must be labelled with the number of
numeric holes and kept distinct from a round score. History shows the round's
native points and its labelled overall equivalent using preserved snapshots.
The server owns all point/contribution arithmetic; mobile UI code only formats
validated results and separates locally pending input from confirmed points.

Project allowed holes **before** computing points, resolved progress, equivalent
contributions, completion and tie explanations. Keep the full-layout handicap
allocation. A hidden completed final remains excluded entirely under current
visibility policy. A hidden back-nine no-score or numeric edit must not influence
any non-admin or public result JSON.

Public sharing remains limited to overall gross/net standings and existing display
names; no scorecard, hole state, private identifier, handicap or account fields
are added. It needs an explicit non-private value-basis discriminator/label so
mixed comparable totals are not presented as actual strokes. Keep its current
expiry/revocation, no-store and non-admin projection semantics. Public API and
browser decoder changes must ship together; scope preservation does not mean
silently preserving an inaccurate numeric field meaning.

### Offline behavior and acceptance examples

Queue numeric/no-score edits by the same account/player/round/hole identity.
Persist before delivery; preserve immutable requests, exact-version conflicts,
acknowledged-predecessor successors, account isolation and explicit discard.
Conflicts must display both actual states, including no-score; a retained no-score
entry expects a present revision rather than absence. Refetch canonical card data
before claiming points are server-confirmed. A numeric correction from 9 to 10
that leaves points at zero still changes the preserved score and revision.
No queued confirmation, background sync or cold offline launch is added.

Version the new outcome payload deliberately: existing numeric request bodies,
normalized fingerprints and persisted receipts must continue to replay exactly.
Do not inject a new default field into old fingerprint serialization. Preserve
or explicitly migrate already-persisted numeric device queue entries without
losing their immutable request IDs or acknowledged-predecessor expectations.
Include legacy receipt/queue upgrade regressions before exposing the new format.

| Par | Gross entry | Received strokes | Gross points | Net points | Overall gross/net hole contribution |
| --- | --- | --- | --- | --- | --- |
| 4 | 4 | 0 | 2 | 2 | 0 / 0 |
| 4 | 5 | 1 | 1 | 2 | +1 / 0 |
| 4 | 6 | 2 | 0 | 2 | +2 / 0 |
| 4 | 9 | 1 | 0 | 0 | +2 / +2 |
| 3 | 2 | 0 | 3 | 3 | −1 / −1 |
| 5 | picked up | 2 | 0 | 0 | +2 / +2 |

These six resolved holes yield 6 gross points and 9 net points, with comparable
subtotals +6/+3 (`12 - points`). The card is still incomplete; the other 12 blank
holes contribute nothing. If the pickup instead remains blank, there are only
five resolved holes and the subtotals are +4/+1. Neither state is an 18-hole
result. On an index-18 par-4 hole, gross 4 with one stroke given back earns gross
2/net 1. A par-5 gross 1 with three received strokes earns gross 6/net 9 under the
explicit formula; do not impose the visible table's six-point ceiling.

Visibility example: a hidden-final front nine with nine resolved holes and 20
permitted points contributes −2 (`18 - 20`), not +16. Altering the hidden back nine
must change neither that value nor visible progress or metadata. This is a
restricted view of an 18-hole round, not support for a nine-hole competition.

Full-round cases: 18 pars with handicaps disabled earn 36 points and contribute
0. Four birdies plus 14 pars earn 40 and contribute −4. A par-72 card containing
17 pars and a holed-out 10 on the remaining par-4 has actual gross 78, earns 34
points and contributes +2 rather than its actual +6. Replacing that 10 by an
explicit pickup still earns 34 and contributes +2, but no actual full gross total
exists. All 18 explicit pickups earn 0 and contribute +36; all 18 blanks have no
result and cannot be confirmed.

Mixed completed-round example, all confirmed and visible, no mandatory round,
best-N 2, with gross/net comparable values:

| Round | Format | Player A | Player B |
| --- | --- | --- | --- |
| 1 | Individual stroke play | +4 / +1 | +3 / 0 |
| 2 | Individual Stableford | 32/40 points → +4 / −4 | 34/38 points → +2 / −2 |
| 3 | Four-ball, A and B partners | −1 / −3 | −1 / −3 |

A selects gross −1 and either tied +4 for +3; its net selection totals −7.
B selects gross −1/+2 for +1 and net −3/−2 for −5. The raw Stableford points never
enter the sum directly, and the four-ball contribution is credited once to each
partner. With best-N 1 and round 2 mandatory, A must use +4/−4 and B +2/−2, even
when another round has a better value in the selected metric.

For a final-round comparison outside best-N, suppose two otherwise eligible
players each have an earlier best net contribution of −6, with N=1. Their final
Stableford round scores are net 40 and 38 points, equivalents −4 and −2. Both
retain overall −6; `final_round_score` places the 40-point player first. With
`shared_positions`, equal final points, or an incomparable/hidden final group,
they retain the applicable shared place. Do not compare raw points ascending.

### First implementation candidate and release conditions

After all three format definitions, the first bounded Stableford candidate is
pure domain work: numeric/no-score-to-points conversion, resolved progress and
typed native-versus-comparable results, with the acceptance examples above.
Keep the format unavailable and existing calculations unchanged at that stop.
Review and run the affected backend checks before publishing that foundation.

Playable release also requires closed format/creation policies, forward schema
and no-score audit/revision/receipt guards, snapshots, state-aware completion and
confirmation, unit-aware private/public DTOs and decoding, ranking/selection,
history, offline state handling and mobile UI. Resolve those as explicit later
slices; do not expose a format enum before all paths are coherent.

Acceptance must include fresh and populated-schema migrations; numeric/no-score
ABA and lost-response delivery; authority and lock races; zero-point completion
versus empty cards; correction invalidation despite unchanged points; plus,
disabled and allowance-boundary handicaps; best-N and mandatory/final-round
comparisons; hidden-score noninterference; public labels; historical regression
coverage and real Chrome at 320/390/1280px in offline/loading/error/empty/populated
and long-content states. Follow the complete affected validation ladders and
read-only review. No runtime test result is claimed by this definition.
