# Project documentation

## Current product state

The application is a private, mobile-first tournament workspace backed by a
Rust/Axum API, PostgreSQL migrations, an idempotent development seed, and a
strict TypeScript React client. Username/password accounts enter through atomic
creator onboarding or tournament invitations; tournament membership, rather
than a global role or player directory, owns access.

Exact tournament admins configure counted and optional mandatory rounds, select
or manually register one immutable course/tee revision per draft round, manage
teams and flights, start the tournament, and open, complete, or lock individual
rounds through separate controls in the management workspace's Lifecycle section.
Opening calculates and freezes
handicap snapshots from the selected tee. Individual stroke play, two-player
scramble, and two-player foursomes have distinct preserved score ownership and
handicap rules.

Members can enter authorized flight scorecards, confirm cards, and browse live
gross/net round and best-N tournament standings, player contribution histories,
and read-only preserved result cards. Server-Sent Events trigger authoritative
private refetches. For the configured 18-hole final, holes 10–18 default to
hidden from non-admin result projections until the exact tournament admin
releases them; the admin can re-hide them without any time dependency.

## Production and operator behavior

The supported production baseline is a single-host Docker Compose deployment:
HTTPS Caddy serves the built Vite output and proxies same-origin `/api` and SSE
traffic to the Rust release binary; PostgreSQL is reachable only on an internal
container network. API and web restart automatically, while the PostgreSQL data
and Caddy state use named volumes. Production database ownership is split: an
owner URL performs the explicit migration action, then a separate runtime role
receives application DML, sequence, and function privileges but read-only access
to SQLx migration history. The API does not migrate on startup and refuses to
bind when its connected identity differs from `APP_DATABASE_USER`, retains
schema/cluster authority, can modify migration history, or sees missing,
pending, dirty, unknown, or checksum-mismatched migration history.

`GET /api/health` reports process liveness without depending on PostgreSQL.
`GET /api/ready` verifies database reachability and the exact embedded schema
history, returning a stable non-secret `503` while unavailable. Production
configuration requires secure session cookies, a shared proxy secret, an HTTPS
CORS origin if a separate origin is deliberately enabled, and a bounded database
pool. Caddy overwrites the internal client-IP and proxy-secret headers; the API
trusts the client address only when that secret matches, so public forwarding
headers cannot choose a rate-limit identity.

The deployment runbook uses explicit migration and permission actions, never a
production seed. Its backup script creates an atomic custom-format PostgreSQL
dump and relocatable SHA-256 sidecar. Restore requires an explicit confirmation, an empty
public schema, a valid checksum, and a single-transaction restore before runtime
grants are reapplied. See `deployment_guide.md` for deployment, rollback,
monitoring, backup, credential rotation, and disaster recovery commands.

## Repository structure

- `backend/src/api/`: Axum routes, validation, response mapping, and SSE.
- `backend/src/domain/`: models and pure scoring/handicap behavior.
- `backend/src/repositories/`: SQLx queries and persistence operations.
- `backend/tests/`: PostgreSQL integration tests.
- `frontend/src/api/`: typed frontend API boundary.
- `frontend/src/features/`: focused feature controls, presentation, and pure utilities.
- `frontend/src/pages/`: route-level mobile-first views.
- `frontend/src/ui/`: reusable application UI.
- `migrations/`: forward PostgreSQL schema changes.
- `.codex/agents/`: repository specialist role definitions.
- `docs/`: current behavior, durable architecture, active work, workflow, latest
  rationale, and the production deployment/recovery runbook.

## Preserved domain behavior

- Tournament players retain identity and accumulated results across changing
  round teams.
- Opening a round captures the tournament entrant's fixed handicap and the
  calculated course and playing handicaps. A tournament admin may make audited
  corrections before the first opening, but no correction is possible after any
  round has opened or snapshot has existed.
- Team membership is unique per player and round.
- Scores have exclusive player/team ownership.
- PostgreSQL reserves an explicit audited administrator context for any future
  locked-round correction; no operator flow currently exposes that path.
- Score mutations are auditable in PostgreSQL.
- The initial two-player scramble formula is isolated in the domain layer and
  uses 35% of the lower plus 15% of the higher course handicap. Each registered
  index is capped at `36.0` before conversion for scramble only.
- Two-player foursomes uses one alternate-shot team card and exactly two members.
  Its fixed 50% allowance is applied once to the sum of both unrounded Course
  Handicaps, then the team value is rounded under WHS allowance rules. The final
  Playing Handicap is captured in an immutable round-team snapshot.
- SSE messages invalidate client queries; clients refetch authoritative data.

## Round opening

A round can open only while its parent tournament is `active`. Tournament start
therefore cannot be bypassed through the round lifecycle. Starting a tournament
does not require course, tee, team, flight, or scoring readiness; those checks
remain authoritative at the separate round-opening boundary below.

`GET /api/rounds/{round_id}/pairing-validation` reports stable readiness issue
codes plus deterministic team, flight, legacy-group, and split-team details. An
eligible entrant has both an active tournament entry and an active player record.
Every eligible entrant must belong to exactly one nonempty flight, and ineligible
entrants cannot remain assigned. Individual rounds require no teams and reject
all remaining legacy grouping teams. Scramble and foursomes rounds additionally
require every eligible entrant in exactly one two-player score-owning team, with
both members contained in one flight; one flight may contain multiple complete
teams. Their exact-size issue codes remain format-specific. Flight tee time and
starting hole are optional metadata and never prove grouping or readiness.

The response preserves `missing_players`, `ineligible_players`, and `team_sizes`;
team assignment details apply to both team formats, while legacy individual team sizes
remain visible for compatibility. It adds `missing_flight_players`,
`ineligible_flight_players`, `flight_sizes`, `legacy_individual_groups`, and
`split_teams`, plus stable issue codes for each new invalid state. Course, tee,
rating, complete hole-number, and stroke-index rules remain unchanged.

`POST /api/rounds/{round_id}/open` uses the same fact loader and pure validator
while holding the round and tournament transaction locks plus entrant share
locks. Team and flight mutations serialize through the parent round lock. A
failed validation writes no snapshots and emits no event. Success captures exact
decimal handicap inputs and inserts one immutable snapshot per eligible entrant.
For foursomes it also calculates from the unrounded course values and inserts one
immutable team Playing Handicap snapshot per complete team. It then changes
`draft` to `open`, commits, and only then publishes one SSE invalidation.
Concurrent opens cannot duplicate either snapshot kind. Database triggers freeze pairings
and scoring configuration after draft and require all status/snapshot changes to
use the lifecycle transaction.

## Live scorecards

`PUT /api/rounds/{round_id}/scores` immediately saves or corrects one hole. Its
`owner` is tagged as `player` or `team`; `submitted_by` is derived exclusively
from the authenticated session. Same-value retries preserve the original
submitter, timestamp, confirmation, audit count, and SSE state. Changed strokes
append an audit row and invalidate any current scorecard confirmation.

`GET /api/rounds/{round_id}/scorecards/{owner_type}/{owner_id}` returns an
actor-free member read projection with ordered holes, visible gross/net totals,
and the preserved playing handicap. Individual net uses the opening snapshot.
Scramble net applies 35%/15% to the members' rounded course handicaps and applies
the round allowance once. Foursomes net reads the immutable team Playing Handicap
captured at opening; it never recalculates from rounded member snapshots. This
read requires an active session plus any exact membership role in the round's
tournament. Round lookup, session revalidation, membership `FOR SHARE`, and card
assembly share one repeatable-read transaction; successful responses are
`private, no-store`.

When the final back nine is hidden, non-admin member reads contain only
holes 1–9 and totals derived from those holes; authoritative completeness,
confirmation, and confirmation time are null. The full card is available at the
same path plus `/scoring` only after exact admin/scorer/flight-owner write
authorization. That read is non-locking, remains private/non-cacheable, and
rejects locked rounds. Database audit actors remain preserved but are absent
from member read projections.

`POST` to that scorecard path plus `/confirm` requires all holes and records the
session actor as `confirmed_by` plus `confirmed_at`. Confirmation records represent current state;
stroke changes remain historically audited, but superseded confirmation states
are not retained as a separate event history.

## Mobile score entry

The React `/score` page keeps tournament, round, tagged owner, hole, and
hole/summary view in canonical URL parameters. It excludes draft rounds and uses
completion validation as the stable authority for eligible players or teams.
Gross and net values always come from decoded backend scorecards; the browser
does not duplicate handicap calculations.

Each hole has large par, minus, and plus actions bounded to 1-20 strokes. An
owner-and-hole-scoped coordinator serializes writes, coalesces rapid taps to the
latest desired score, and refetches the exact scorecard before showing
`Synkronisert`. Failed writes retain the desired score with Retry and Discard.
Navigation is guarded while a write, verification, failure decision, or
confirmation is unresolved. SSE remains a second invalidation path only.

Complete cards can be confirmed from their summary. Confirmed editable cards
require an explicit correction mode; a changed score removes confirmation and
the corrected card must be confirmed again. Completed rounds remain correctable,
while locked rounds are read-only.

`GET /api/rounds/{round_id}/score-access` supplies the exact player or team owners
the current session may write. The phone-first card rail follows that
deterministic server order, shows completion/confirmation progress, and preserves
the current hole when its semantic buttons switch cards. Rapid card switches
replace browser history. The full owner selector remains available for browsing
eligible read-only cards.

The selected card and at most its two writable neighbors use the user-, round-,
and owner-scoped private TanStack Query cache. Focus or pointer intent may prefetch one
additional chosen card; there is no eager all-flight fetch and no duplicate
client score state. The same unresolved-save or confirmation navigation lock
disables both selectors. The browser never reproduces role or membership policy.
The private, non-cacheable access read revalidates and locks the active session/
player link plus the exact tournament membership inside one repeatable-read
transaction before it assembles the owner list. An authenticated account without
that target membership receives `403`; an exact viewer or exact player membership
without a linked player remains authorized but receives an empty writable-owner
list.

## Authentication and scoring access

Accounts authenticate with a canonical lowercase username and password. A
username contains 3-32 ASCII lowercase letters, digits, underscores, or hyphens;
the API trims and lowercases accepted input. Account email is neither required
nor stored. The optional `players.email` field remains profile contact data and
does not participate in authentication or identity linking.

Login creates a revocable server session and returns an opaque token only in an
`HttpOnly`, `SameSite=Lax` cookie. PostgreSQL stores only the token's SHA-256
hash. Auth responses are not cacheable. Password syntax and a 4 KiB login-body
limit are checked before database work, and Argon2 verification runs through a
shared four-task semaphore off the async executor so credential bursts cannot
consume unbounded blocking workers. Score mutations plus logout require the
session-derived CSRF token. Score writes lock and revalidate the session in their
existing round transaction, so logout cannot complete before an
already-authorized write.

Production request throttling uses a bounded in-process store appropriate to the
single API instance. Each sensitive route has a narrow client-and-resource key
plus a broader per-client ceiling: login allows 10 attempts per normalized
account and 40 per client per minute; creator onboarding allows 3/6 per hour;
invitation preview allows 30/100 per minute; registration allows 5/20 per ten
minutes; authenticated invitation acceptance allows 10/40 per minute; profile
credential changes share a 5-per-client/account and 20-per-client minute limit.
Rejected requests return the stable `rate_limited` JSON error, `429`,
`Retry-After`, and `Cache-Control: no-store`. A narrow-key rejection does not
charge the broad bucket, stale buckets are evicted, storage is capped, and the
limiter is disabled in development and direct unit-test state unless selected
explicitly.

`tournament_memberships` owns the role for a specific trip. Tournament admins
and scorers can write any eligible card in that tournament. A tournament player
linked to an exact flight member can enter, correct, and confirm every eligible
card in that round flight: snapshot-backed player cards for individual play and
every exact-round two-player scramble or foursomes team whose complete membership
is in that flight. Foursomes owners additionally require their preserved team
handicap snapshot. A player without stored flight membership retains the legacy-safe direct
fallback to their own individual card or exact round team. The authenticated
account remains the audit actor regardless of card ownership.

Listing and save/confirmation authorization use the same owner resolver. Flight
membership grants authority only; it never changes score ownership. Starting
hole, tee time, name, order, or a partial team overlap never implies authority.
Viewers, unlinked accounts, cross-tournament roles, non-members, and players in
another flight cannot write. Team identity remains specific to one round.

Tournament detail, roster, round, team, readiness, completion-validation,
scorecard, and round/tournament leaderboard reads require an active session plus
any membership role in the target tournament. The tournament collection is selected directly
through the current user's memberships. Existing resources outside that scope
return `403`, missing resources return `404`, and successful private reads use
`Cache-Control: private, no-store`. Multi-query reads authorize and assemble the
response in one repeatable-read transaction while holding the membership row
`FOR SHARE`, so membership removal cannot commit partway through a response.
The backend isolation suite reuses one account/player across two tournaments with
different tournament handicaps, round snapshots, flights, and player-versus-team
score owners. It proves that rosters, pairings, teams, writable owners,
scorecards, gross/net results, rejected mutations, and identifier-free live events
remain bound to the exact target.

The strict frontend boundary independently verifies every target-bearing
tournament detail, roster, round list/detail, team, pairing, leaderboard,
handicap correction, counted-round, start, final visibility, course-configuration, and invitation
response before TanStack Query or transient UI receives it. Duplicate resource
identities and mismatched tournament, round, player, team, metric, or invitation
predecessor facts fail closed as invalid server data. Query keys remain rooted by
session user and target resource; decoder checks supplement those keys instead of
replacing backend authorization.

Tournament, management, round, leaderboard, and invitation workspaces remount on
route target changes. Unsaved handicap/count configuration, mutation state,
errors, receipts, and revealed one-time invitation secrets therefore belong only
to the tournament where they originated. A late invitation completion may update
its original private query, but cannot render its token after navigation.

Management and lifecycle mutations resolve the target tournament from the
resource and lock/revalidate both the session and admin membership inside the
write transaction. Direct round creation additionally performs an exact-admin
preflight before content-type, JSON, unknown-field, round-count, or course/tee
validation. Existing unauthorized targets therefore return `403` without
revealing those facts; missing targets return `404`, and the insert transaction
reauthorizes before writing and publishing its post-commit invalidation. The
former global player/profile/handicap routes and legacy
`POST /api/tournaments` platform-admin contract are retired. The same POST path
now exposes authenticated per-account creation, described below; it does not
restore global administrator authority. Player discovery is available only from
a target tournament's private roster. Direct `POST
/api/tournaments/{tournament_id}/players` registration is also retired: admins
enroll players through tournament invitations, never by submitting a global
player identifier. `GET
/api/me/tournaments` returns only the active user's tournament roles and linked
entrant identities.

## User profile

Choose **Profil** in the main navigation, or open `/profile` while signed in.
The page starts with **Mine turneringer**, listing all your tournament
memberships, including archived tournaments, with their current role and a link
to each tournament. It also links to creating
another tournament and the full tournament overview. Name and handicap stay visible
below the list. **Endre brukernavn** and **Endre passord** are separate, initially
collapsed sections operable by keyboard. Save/error feedback remains visible
when a section is collapsed.

- **Name and handicap:** names are trimmed and limited to 100 characters. A name
  change updates the account and linked player, so the new name also appears in
  past results. Profile handicap accepts comma or point, −10.0 through 54.0 with
  at most one decimal, and displays Norwegian comma formatting. A changed or
  newly created handicap needs no typed explanation. The server records
  “Egen profilendring” with the actor, timestamp, and handicap in
  `handicap_history`. Existing tournament handicaps, including draft
  entries, and preserved round handicaps/results do not change. New entries use
  the current profile handicap. Corrections within an existing tournament remain
  the separate administrator workflow with an explicit audit reason.
- **Unlinked or inactive accounts:** an unlinked account can explicitly create
  its own player profile by entering handicap; this does not enroll
  it into existing trips. An inactive linked player cannot self-change handicap
  or reactivate the player; name/credential editing remains available.
- **Username:** changing it requires the current password. The existing 3–32
  ASCII letter/digit/underscore/hyphen grammar and lowercase normalization apply;
  occupied usernames are rejected. Use the new username at the next login.
  Existing sessions remain active.
- **Password:** the current password and repeated new password are required in
  the UI. Normal guidance suggests a long password or phrase. Profile, creator
  onboarding, and invitation registration use the same validator: the account
  contract remains 12–128 UTF-8 bytes, with spaces preserved. Too-short/too-long
  messages explain the applicable byte limit and how to adjust the password;
  multibyte characters are not treated as single bytes. A successful change
  logs the account out on every device and returns to sign-in with confirmation.
  There is no current-password bypass or account-recovery operation here.

The API is self-only and all responses use `Cache-Control: private, no-store`.
`GET /api/me/profile` returns `user_id`, `username`, `display_name`, `version`,
and nullable `player_id`, `handicap`, `player_active`, `player_updated_at`.
`PUT /api/me/profile` accepts `version`, `player_updated_at`, `display_name`,
`handicap`. The server owns the self-service audit description; caller-supplied
`reason` is rejected as an unknown field. Older open clients must reload after
this contract update. `POST /api/me/profile/username` accepts `version`,
`username`, `current_password`; the password POST accepts `version`,
`new_password`, `current_password`. Both credential POSTs return 204; the details
PUT returns the authoritative profile. Mutations require the session CSRF header,
reject unknown fields, and have an 8 KiB body limit. Passwords/hashes/generations
are never included in profile responses.

Stale edits return `409 profile_stale`; an occupied username returns
`409 username_unavailable`, incorrect current password returns
`409 current_password_incorrect`, and inactive handicap editing returns
`409 profile_inactive`. Expired/replaced sessions return 401, missing/wrong CSRF
403, malformed input 400, oversized input 413 and credential throttling 429.
Refresh/retry reads authoritative state and may replace unsaved form fields.
After an uncertain response, refresh before retrying; if a password change may
have committed, try signing in with the new password. Email, avatars, account
deletion, recovery and editing someone else's account are outside this page.

## Creator onboarding

`POST /api/onboarding/tournaments` accepts nested creator account/player data,
tournament dates, and one to thirty contiguous round definitions. It rejects
unknown privileged fields, already-authenticated callers, duplicate normalized
usernames, invalid date ranges, unsupported formats, oversized bodies, and bounded
field violations before running Argon2.

One transaction creates the linked player and global `player` account, both
initial handicap histories, draft tournament, tournament-admin membership,
entrant, all draft rounds, hashed invitation, and hashed session. The server
derives round count and the individual/team/combined tournament scoring mode.
The creator chooses how many rounds count and may select one optional mandatory
round. The wizard defaults to all configured rounds, preserves a smaller
explicit best-N choice while rounds are edited, clears a mandatory selection if
that draft round is removed, and submits `counted_rounds` plus the selected
`mandatory_round_number`. The backend maps that number to a preallocated round
UUID inside the creation transaction.
Individual and scramble rounds retain their existing default allowance. A
foursomes round is created with the required 50% allowance; no separate allowance
editor is exposed by onboarding.
Draft rounds intentionally have null course/tee IDs until later configuration,
so the existing opening-readiness checks keep them closed.

The initial invitation is unlimited-use until its expiry seven days after the
tournament end date. Its URL uses `/join/{invitation_id}#token={secret}` and only
the hash is stored. The raw secret appears once in a `Cache-Control: no-store`
response and transient browser state. Invitation redemption and authenticated
reissue use the same hashed-secret model.

The React `/` route is the signed-out start screen, `/create` is the accessible
mobile setup wizard, and `/tournaments` lists only the signed-in user's
memberships. Membership query keys include the account ID and identity-scoped
caches are cleared before an initial or changed identity is published. Tournament,
round, score, and leaderboard routes are session-protected. Same-user background
session refreshes keep the workspace mounted, and SSE invalidation excludes the
auth query while refetching affected workspace data.

## Signed-in tournament creation

Choose **Opprett ny turnering** from **Dine turneringer**, or visit `/create`
while signed in. The three steps are tournament details, round plan, and review;
no new username, password or player profile is requested. You become administrator
of the new trip, independently of your role in other trips. Your active linked
player is enrolled with their current handicap captured as the new fixed
tournament handicap. An account with no active linked player creates as a
non-playing administrator. No existing account, session, profile, membership,
score or handicap snapshot is replaced.

After creation, administration opens at **Baner**. Configure saved courses/tees,
set up teams/flights, and issue invitations through the existing controls. Unlike
first-account onboarding, this path does not issue an invitation automatically.
The existing tournament list is preserved and refreshed, including prior tests
and real trips.

`POST /api/tournaments` requires an active session and CSRF token. Its strict
64 KiB JSON contract is `{request_id, tournament, rounds}`. The latter two fields
match first-account onboarding; `request_id` is a non-nil UUID. Actors, roles,
entrant IDs, status, round count and scoring summary cannot be supplied. The
server creates the complete draft plan, exact admin membership, optional entrant
and initial tournament-handicap history atomically. Requests are throttled at
20 attempts per client/account and 40 per client per hour in the process-local
production limiter; `429` includes `Retry-After`.

The private/no-store response is `{request_id, tournament_id, created}`: `201`
for a new creation and `200` with `created:false` for an identical retry. No
session cookie or invitation token is returned. Schema 22 retains the request's
normalized hash and resulting ID per account; reusing the key with different
facts gives `409 tournament_creation_key_reused`. Retries still require current
admin access. Previously successful plans can be retried after their end date,
while new plans cannot end before today's UTC date.

The wizard prevents duplicate submissions. If the outcome is uncertain, it
freezes the submitted plan and keeps the same retry key even when a later retry
is throttled or rejected. Retry without editing to recover safely. The key is
held only in that mounted wizard, not persisted across refresh/navigation; check
**Dine turneringer** before starting a fresh attempt after leaving it. Account
changes unmount the wizard; late responses cannot navigate or recreate private
cache data for the previous account.

## Invitation onboarding

`POST /api/invitations/{invitation_id}/preview` authenticates the fragment token
from a JSON body before revealing only the tournament name, dates, and invitation
expiry. New visitors use `/register`; the server preflights the link before
Argon2, then atomically creates the account, player, both handicap histories,
entrant, player membership, append-only redemption, and session. Authenticated
visitors use the CSRF-protected `/accept` route, which relies only on the exact
session-linked player and never infers identity from email.

Together with creator onboarding and signed-in creation, these are the only HTTP participation entry
points. Registration or acceptance creates or verifies the membership and
entrant in the same target tournament and records the initial tournament
handicap and redemption atomically. There is no admin account/player lookup or
direct roster insertion endpoint.

Complete active participation is idempotent even after a link is later expired,
revoked, rotated, or exhausted. Viewer memberships are promoted to player;
player, scorer, and admin roles are retained. Inactive players, withdrawn
entrants, and accounts without a linked player fail closed. Joining creates no
team or flight assignment.

Tournament admins manage links at
`/api/tournaments/{tournament_id}/invitations`. Rotation revokes one active link
and creates one successor with the same expiry, maximum uses, and series root.
Capacity is counted across the entire series. PostgreSQL serializes redemption
against exact identity, membership, invitation, series, and entrant rows and
rejects invalid lifecycle or over-capacity inserts. Redemption facts are
immutable during tournament lifetime; explicit tournament deletion may cascade
them. Legacy revocations retain an explicit unknown-actor marker instead of
inventing audit provenance.

The React `/join/:invitationId` page keeps the fragment through preview and
retry so a reload can recover, but sends the secret only in JSON bodies and
clears the fragment after successful joining. It never places the token in query
keys, query parameters, storage, logs, or request URLs. The admin page shows
newly issued plaintext links once in component state; lost links are replaced
through rotation rather than recovered from storage.

## Tournament management workspace

Authenticated tournament administrators enter the management workspace at
`/manage/tournaments/{tournament_id}`. The route confirms both the canonical
tournament detail and the current user's tournament-specific `admin` membership
before enabling roster or round reads. Signed-out visitors retain the complete
return URL through login. Invalid identifiers, missing tournaments, non-admin
memberships, loading, retryable failures, empty collections, and populated data
have distinct states. Client gating controls presentation only; every private
read and invitation mutation remains protected by backend membership policy.

The workspace provides semantic anchors for settings, entrants, invitations,
rounds, courses, pairings, and lifecycle. Most sections report only facts already
preserved by the existing private APIs and link to the invitation and round
surfaces. Settings exposes “Tellende runder” and an optional mandatory-round
selector while the tournament and every existing round remain draft. `PATCH
/api/tournaments/{tournament_id}/counted-rounds` requires an explicit
`mandatory_round_id` UUID or JSON `null` together with counted N, the current
tournament timestamp, CSRF, and exact tournament-admin membership. It locks the
round set before the tournament, rejects cross-tournament IDs and stale writes,
and permanently freezes both values when the tournament starts or a round has
opened. PostgreSQL independently requires the admin workflow context and uses
durable opening/snapshot markers even if later data is removed. An unchanged
pair preserves `updated_at` and emits no event; a real change returns the
authoritative private tournament and publishes one post-commit invalidation.

The Lifecycle section exposes `Start turneringen` only to the exact tournament
admin. `POST /api/tournaments/{tournament_id}/start` requires CSRF plus the
current tournament `updated_at`. Under deterministic locks it revalidates the
active session and exact `admin` membership, requires rounds numbered exactly
`1..=number_of_rounds`, requires every round to remain draft, and requires at
least one registered, non-withdrawn player whose account is active. Course, tee,
pairing, and flight readiness are deliberately deferred to round opening.

A successful start changes only the tournament from `draft` to `active`, returns
the authoritative private/non-cacheable tournament, updates the identity-scoped
query cache, and publishes one identifier-free tournament invalidation after commit.
An already-active retry is idempotent and emits no event. Stale, not-ready, and
invalid-state requests use `tournament_start_stale`,
`tournament_start_not_ready`, and `tournament_start_invalid_state`; `401`, `403`,
and `404` retain their existing meanings. The mobile panel fails closed while
roster or rounds load, gives explicit retry guidance, prevents duplicate starts,
and confirms that individual rounds remain in `Kladd` after tournament start.

Migration 0015 protects the transition and preserves the complete existing
round completion, locking, snapshot, and foursomes database guards. Tournaments
that legally had a non-draft round while still marked draft under the previous
schema are promoted to active during upgrade, with their round states preserved
and the tournament timestamp intentionally refreshed. Untouched draft
tournaments remain draft and use the explicit start workflow. A separate
`BEFORE INSERT` guard rejects every non-draft tournament creation, so internal
repository or direct-SQL callers cannot manufacture an active, completed, or
archived tournament. The general repository creator and legacy HTTP creator are
both draft-only. Concurrent starts commit one changed result and one event;
start racing the counted-round update serializes without deadlock and leaves
either the started original configuration or a newer draft configuration that
must be started with its refreshed timestamp.

The Courses section is writable for draft rounds through the existing
atomic course-configuration API. It does not infer revisions, load teams per
round, or expose unsupported mutation controls. Returning from invitation
administration replaces that history entry, restores the Invitations section,
and moves focus to its labelled region without creating a browser-Back loop.

Tournament admins search the bundled shortlist through
`GET /api/tournaments/{tournament_id}/course-catalog?q=...`. Omitting `q` or
passing a blank value lists all eight entries in deterministic catalog order;
otherwise matching is case-insensitive across the display name and internal
aliases. Search performs no external request and works without a provider key.
It currently includes Hacienda del Álamo, Saurines de la Torre, Mar Menor,
Oppegård, Drøbak, Miklagard, Oslo, and Haga.

Each result reports the display name, country, provider, nullable verified
provider course ID, and an explicit `usable`, `incomplete`, or `missing` status
with a short reason. Aliases remain server-internal. Live verification found
Oslo and Haga but their provider holes omit stroke indexes; Miklagard has no
provider tees; the other five returned no verified match. All eight therefore
remain deliberately unavailable for scorecard import rather than receiving
guessed IDs or incomplete hole facts.

For a future catalog entry verified as usable, the backend retrieves detail
through the [official GolfCourseAPI contract](https://api.golfcourseapi.com/docs/api/)
at
`GET /api/tournaments/{tournament_id}/course-provider/courses/{provider_course_id}`.
The backend checks the exact tournament-admin membership and catalog readiness,
then commits before consulting its cache or provider. Unknown IDs return `404`;
known incomplete IDs return `409`, both before provider I/O. Provider IDs use
the official opaque, case-insensitive eight-character alphabet.

The catalog and detail responses are deliberately separate contracts. Catalog
rows expose only curated identity and readiness. A usable detail response adds
provider club/course names, optional scorecard URL, location, and category-
labelled tees with rating, slope, length, par, and ordered holes. Hole numbers
are derived from provider order and provider `handicap` is named explicitly as
`stroke_index`; no upstream tee ID is invented. This step does not persist
provider facts or configure a round.

`GOLF_COURSE_API_KEY` is optional and backend-only. For usable detail reads it is
sent in a sensitive Bearer header and never returned or logged. Uncached calls are
bounded to two concurrent requests, a two-second connect timeout, five seconds
total, and a 1 MiB response. A 256-entry in-process cache retains details for 24
hours. `GOLF_COURSE_API_DAILY_LIMIT` defaults to
50 uncached requests per UTC day per backend process; a provider `429` exhausts
that local day immediately. Multi-instance deployments need a shared quota to
enforce an account-wide ceiling. Successful reads and provider errors are
private/non-cacheable; unavailable, busy, timeout, exhausted, malformed,
upstream-failure, incomplete-catalog, and missing-course states use the standard
error envelope. The provider's live `{ "course": ... }` envelope is decoded,
and empty tees or missing/duplicate stroke indexes fail closed.

Saved user-supplied presets are available without a provider key. In tournament
administration, open **Baner**, choose **Endre** or **Konfigurer** for a draft
round, and use **Velg lagret bane**. Inspect the men's red tee rating, slope, par,
and expandable 18-hole par/stroke-index table, then explicitly click **Bruk
lagret bane på runden**. Saving sets that round to 18 holes and creates its own
immutable revision; other rounds and historical results are unchanged.

| Saved course | Tee category/name | Rating | Slope | Par |
| --- | --- | --- | --- | --- |
| Hacienda del Alamo Golf Club | Men / Red Tees | 72.5 | 125 | 72 |
| Saurines Golf Course | Men / Red Tees | 66.2 | 116 | 72 |
| Mar Menor Golf Course | Men / Red Tees | 66.9 | 118 | 72 |

Migration 0021 stores the administrator-supplied ordered pars and stroke indexes.
These facts are not provider-verified; distances and location remain null.
`GET /api/tournaments/{tournament_id}/course-presets` returns only explicitly
registered layouts, never arbitrary private course revisions, and requires exact
tournament-admin membership (`401`/`403`/`404`, `Cache-Control: private, no-store`).
Its response is an array of `{id, course_name, location, tee}`; tee contains
`category`, `name`, `course_rating`, `slope_rating`, and ordered `holes` with
`number`, `par`, `stroke_index`, and nullable `distance`. The frontend validates
this contract before caching under the current user and tournament. Loading,
empty, retryable error, pending save and non-draft states disable unsafe saves;
late save responses cannot repopulate private caches after workspace unmount.

The configuration fallback for other missing or incomplete provider courses
is manual tournament-admin entry. The admin chooses or names one tee and must
provide its category, course rating, slope, plus every ordered hole's par and
complete unique stroke-index permutation. Hole distance in yards is optional.

The backend persistence boundary for that flow is implemented. Both manual and
provider-tagged commands pass through the same pure validation and caller-owned
transaction into the existing course, tee, and hole tables. Source, nullable
opaque provider course ID, database import time, selected tee category/name,
rating, slope, and complete holes are stored together. PostgreSQL defers the
completeness check until finalization so the hierarchy can be built atomically,
then rejects inserts, updates, or deletes anywhere in the finalized revision.
Concurrent child writes serialize on the ancestor course row. Rows predating the
revision migration remain explicitly legacy with null provenance rather than
being mislabeled.

`PUT /api/rounds/{round_id}/course-configuration` invokes this boundary for an
exact tournament admin. The JSON body includes the current round's
`expected_round_updated_at` and one tagged selection. A `manual` selection sends
course name, optional location, one tee category/name/rating/slope, and ordered
hole par/stroke-index facts with optional positive `distance`. A
`golf_course_api` selection sends only a curated provider course ID and one tee
category/name; names, ratings, slope, location, and holes are always refreshed
server-side.

The endpoint checks session, CSRF, exact membership, draft state, and the
optimistic timestamp before provider quota can be spent. It never holds a
database transaction across provider I/O. Its final transaction locks and
reauthorizes the round, repeats the draft/version checks, inserts the immutable
revision, and attaches its course/tee IDs, copied names, and hole count. It
returns the updated private/non-cacheable round and publishes one round
invalidation only after commit. Concurrent saves use
`round_configuration_stale`; opened rounds use `round_not_draft`; a disappeared
provider tee uses `course_provider_tee_stale`. Invalid, stale, unauthorized,
provider-failed, or attachment-failed requests create no revision and emit no
event. Requests must be JSON and are capped at 32 KiB.

The Courses section shows every round's preserved course/tee summary and allows
only one draft-round editor to be expanded. Its private catalog search accepts
the backend's UTF-8 byte limits, retains but disables previous results while a
new query loads, and shows each unavailable row's reason. A usable row loads
complete tee facts—category, rating, slope, optional length, par, and hole
completeness—before the admin sends only its provider ID plus exact tee selector.
No bundled row is presently verified as usable, so the production UI points all
eight provider entries to manual entry while remaining ready for a future
verified row. The separate saved-course picker above does not depend on those
provider identities being available.

The manual form accepts 1–36 ordered holes, Norwegian comma or dot course
ratings, required tee/category/rating/slope/par/stroke-index facts, and optional
location and yard distances. It identifies errors on the exact field, requires
a complete unique stroke-index permutation, preserves entered rows while the
hole count changes, prevents duplicate saves, and restores focus after success.
Successful saves replace and refetch the precise round queries, collapse the
editor, and announce the preserved course/tee. Stale, opened-round, provider-
tee-stale, loading, empty, retry, and retained-result states preserve safe input
without silently overwriting a newer round. Catalog/provider queries are
excluded from generic SSE invalidation so score events cannot consume provider
quota; authoritative round events still refresh the round summaries.

PostgreSQL has a normalized flight persistence boundary. `flights`
owns exact round/tournament identity, a trimmed round-unique name, nullable
starting hole and tee time, and timestamps. `flight_memberships` proves the
exact flight, round, tournament, and entrant relationship and permits at most
one flight per player in a round. There is no separate scorekeeper designation:
every authenticated tournament player linked to an exact flight member can
score every eligible card in that flight at runtime.

Deleting a flight, round, or tournament cascades its memberships. Direct
inserts, updates, and deletes on both relations reuse the parent-round pairing
lock and fail after draft, including when mutation races opening. Migration 0011
created the normalized hierarchy; forward migration 0012 removes its unused
single-scorekeeper table. The upgrade intentionally discards only obsolete
designation metadata and preserves flights and memberships exactly. Legacy team
data is unchanged: equal starting holes, tee times, order, or team membership
never infer a flight.

`GET /api/rounds/{round_id}/pairings` gives any exact tournament member one
private, non-cacheable, deterministically ordered view of the round version,
effectively active and inactive entrants, shared-result teams, flights, and
legacy individual groups. Its repeatable-read transaction holds membership
authorization through response assembly. `players.active` and tournament-entry
status jointly determine eligibility.

`PUT /api/rounds/{round_id}/pairings` is the only supported pairing mutation.
It requires CSRF, exact tournament-admin membership, `application/json`, a
256 KiB body limit, the visible `expected_round_updated_at`, and complete arrays
describing the desired current teams, flights, members, and explicit legacy
conversions. Empty and partial draft rosters are valid; later readiness owns
missing-assignment policy. Submitted members must be eligible entrants, UUIDs and
names cannot conflict within or across rounds, and array order determines new
membership display order.

The final transaction locks the round, reauthorizes the active session and admin
membership, repeats draft/version checks, validates identities and references,
then replaces the roster and advances the round timestamp atomically. Only a
successful commit emits one identifier-free round event. Errors, including
malformed, oversized, stale, opened, identity-conflict, conversion, and
referenced-team failures, are private/non-cacheable and leave no partial rows or
event.

For an individual round, every existing grouping-only team must be acknowledged
exactly once and mapped to a requested flight that has the identical name,
schedule, ordered members, nullable display orders, and timestamps. Conversion
then removes the obsolete team. Scramble teams are never converted because they
remain score-owner/history identities. A scheduled retained scramble team clears
its schedule only when all its members are explicitly placed in one requested
flight carrying the same starting hole and tee time; equal facts never infer the
relationship. The old granular team-create/member routes are retired, while the
member-readable team GET remains for compatibility. Score authority uses only
the resulting exact flight membership and never schedule equality.

The tournament-admin workspace includes one mobile pairing editor for one
expanded round at a time. It reads the private aggregate below the authenticated
user's query root and uses labelled inputs, selects, and add/remove/move/order
buttons rather than drag-only interaction. Scramble and foursomes teams are
edited independently from flights; individual rounds expose flights only. Existing group identities
and exact member order are preserved, new groups receive browser-generated
UUIDs, inactive stored members have removal-only cleanup, and non-draft rounds
remain readable but disabled. Incomplete drafts may be saved while unresolved
flight assignments and non-two-player score-owning teams remain clearly labelled;
opening readiness stays authoritative.

Each save sends the entire desired roster with the aggregate `updated_at` token,
CSRF, and exact schedule facts. The returned aggregate replaces the precise
pairing cache and round timestamps are authoritatively refetched. Duplicate
submission is blocked. A newer server aggregate—including an entrant-only change
with the same timestamp—never overwrites a dirty local draft; the admin must
explicitly discard and reload. Exact tee-time seconds/fractions survive unrelated
edits. Legacy individual groups require a separate exact conversion save before
ordinary editing, and an old scheduled scramble team requires an explicit flight
selection that copies its schedule. Clearing or changing that selection restores
the prior flight schedule, so no relationship is inferred and no orphan schedule
is persisted.

The idempotent development seed now demonstrates that model across all five
draft rounds. Each round has two deterministic four-player flights scheduled on
starting holes 1 and 10, with changing player rotations. Scramble rounds one and
two retain four two-player score-owner teams; round four uses the same exact team/
flight facts for foursomes with its 50% allowance. Individual rounds three and
five contain flights but no teams. Every team is wholly contained in one flight.
Rerunning the seed backfills only nonconflicting deterministic rows. It converts
the old seed's team-level starting holes to flight schedules only while the exact
known draft team, membership, and flight facts match, and it skips frozen rounds
without bypassing pairing locks. The seeded tournament remains in `draft` so the
hosted start action can be exercised; its round pairings become openable only
after the seeded exact admin starts the tournament.

## Fixed tournament handicaps

`GET /api/tournaments/{tournament_id}/players` returns both the roster and an
authoritative correction state. Tournament admins may call
`POST /api/tournaments/{tournament_id}/players/{player_id}/handicap-corrections`
with a numeric `handicap_index` and nonblank audit `reason` only while that state
is editable. The repository revalidates tournament-admin membership and uses the
same deterministic round-before-tournament lock order as opening a round.

PostgreSQL rejects direct tournament-handicap updates without explicit
correction context, appends immutable history for each changed value, and records
a permanent lock marker at the first round opening or snapshot. A global player
handicap change therefore affects future registrations only. The React roster
uses the server state as authority, handles an opening race as a permanent lock,
and accepts either comma or point input while displaying Norwegian decimal
commas.

## Round completion and locking

### Tournament completion API

After every configured round is locked, an exact tournament administrator can
explicitly complete an active tournament with:

```http
POST /api/tournaments/{tournament_id}/complete
Content-Type: application/json
X-CSRF-Token: <session CSRF token>

{"expected_tournament_updated_at":"<timestamp from the tournament read>"}
```

The request requires the active session cookie, the exact timestamp field and no
unknown fields. Success returns the existing tournament object with `completed`
status and `Cache-Control: private, no-store`. Completion is not automatic and
requires all configured rounds, not merely the counted standings rounds.
Completed retries are idempotent: no extra completion record, timestamp change or
live event. Active stale requests return `409 tournament_completion_stale`;
unlocked/missing rounds return `409 tournament_completion_not_ready`; draft or
archived source states return `409 tournament_completion_invalid_state`.
Authentication, CSRF, membership and missing-resource checks retain 401/403/404.

Completion records its administrator and database time. No score, handicap,
confirmation or team data changes. Finish corrections before locking the rounds;
there is no reopen/unlock operation. Completion never releases hidden final
results: the administrator's separate visibility control remains available.

Closed tournaments block new invitations, rotations and joins, including a
concurrent registration that loses the parent lock (its new identity data rolls
back). Existing members retain private access; harmless already-joined retries
retain their existing semantics. Invitation listing/revocation remains available.

### Tournament completion controls

Exact tournament administrators use **Administrasjon → Livsløp → Fullfør
turneringen**. The panel checks all configured rounds, including rounds excluded
from the standings count, and links each unlocked round to its lifecycle controls.
Draft tournaments explain the start/lock prerequisites; an incomplete or invalid
round plan blocks completion. The completion panel is read-only for completed and
archived tournaments; final visibility and invitation revocation remain available.

Once every round is locked, **Fullfør turneringen** opens a confirmation explaining
closed joining, retained member access, no reopening and unchanged final-result
visibility. **Avbryt** receives focus; Escape cancels. **Bekreft og fullfør
turneringen** submits once with the current tournament timestamp. Refreshing
authority or readiness discards the confirmation, even if the returned facts are
unchanged. Loading, read failures and pending mutations disable completion.

Successful, conflicting and uncertain responses trigger authoritative private
query refreshes. Failed reconciliation keeps completion disabled until the
explicit **Oppdater fullføringskontrollen** succeeds. A completion in another
administrator session removes the action through live updates. Mutation responses
never recreate private cache data after leaving the workspace or changing accounts.

The repeatable Chrome suite requires a fresh disposable migrated/seeded backend
and frontend on port 5173: from `frontend`, run
`GOLF_TOURNAMENT_COMPLETION_BROWSER=1 npm run test:browser:completion`.
It scores, confirms, completes and locks all seeded rounds, then completes the
tournament; never point it at retained or production data.

### Tournament archive API

Archiving is a separate exact-admin action for completed tournaments:

```http
POST /api/tournaments/{tournament_id}/archive
Content-Type: application/json
X-CSRF-Token: <session CSRF token>

{"expected_tournament_updated_at":"<timestamp from the tournament read>"}
```

The request requires the active session cookie and only that timestamp field.
Success returns the existing tournament object with `archived` status and
`Cache-Control: private, no-store`. A stale completed request returns
`409 tournament_archive_stale`; draft/active sources return
`409 tournament_archive_invalid_state`. Authentication, CSRF, exact membership and
missing-resource failures retain 401/403/404. An authorized archived retry returns
the current resource unchanged, even with the earlier expected timestamp, without
another audit record, timestamp change or live event.

Archive records the administrator and database time. It changes no scores,
snapshots, confirmations, teams, members or final visibility. Existing members
retain private reads and gross/net history; API collections still include archived
tournaments. New invitations, rotations and joining remain closed. Invitation
revocation and independent release/re-hide of the final nine remain available.
There is no deletion or reversal.

### Archive controls and tournament history

Exact tournament administrators use **Administrasjon → Livsløp → Arkiver
turneringen** after completion. Draft/active tournaments explain the completion
prerequisite. Confirmation explains that the tournament moves from Nåværende to
Arkiv without deleting results, removing member access or changing final-nine
visibility. **Avbryt** receives focus; Escape cancels. **Bekreft og arkiver
turneringen** submits once with the current tournament timestamp.

An authority/read refresh discards confirmation, even if the same facts return.
Success, conflicts and uncertain responses refresh authoritative private reads;
failed reconciliation blocks archive until **Oppdater arkiveringskontrollen**
succeeds. A second administrator's archive arrives through the existing management
live subscription and removes pending controls. Archived state links to history;
there is no undo. Leaving the workspace or switching accounts cannot cause a late
mutation response to recreate private cache data.

**Dine turneringer** has URL-backed **Nåværende**, **Arkiv** and **Alle** views.
Nåværende is the default and includes draft, active and completed tournaments;
only archived tournaments move to Arkiv. Use `/tournaments?view=archived` for
history or `?view=all` for both. Links show counts and retain browser back/forward
selection. Filtering happens over the unfiltered private membership collection;
management authority, other selectors and direct history links are unchanged.
No-membership and empty-view messages are distinct, and failed reads hide stale
cards until retry succeeds.

**Oppdater turneringer** always requests current data. Returning to the list or
focusing the window refreshes stale data; the shared query freshness window is
20 seconds. The list has no global live subscription, so another administrator's
archive can remain unseen until a manual refresh or a return/focus after that
window. This does not affect management's live updates.

For repeatable Chrome validation, use a fresh disposable migrated/seeded backend
with the frontend on port 5173. From `frontend`, run
`GOLF_TOURNAMENT_ARCHIVE_BROWSER=1 npm run test:browser:archive`.
This mutates all five seed rounds and completes/archives the seed tournament;
never run it against retained or production data.

### Round transitions

`GET /api/rounds/{round_id}/completion-validation` returns a repeatable-read,
deterministically ordered view of every required player or team scorecard. Exact
admins receive authoritative progress, confirmation, and lifecycle readiness.
While the final back nine is hidden for a non-admin, it counts only actual scores on holes
1–9, reports nine visible required holes, nulls completion, confirmation, and
readiness, and omits issue codes derived from hidden state.

`POST /api/rounds/{round_id}/complete` accepts only an open round, and
`POST /api/rounds/{round_id}/lock` accepts only a completed round. Both serialize
on the same round-row lock used by scoring, recompute per-owner readiness inside
the transition transaction, and publish one round SSE invalidation after commit.
Individual owners come from immutable opening snapshots; scramble owners come
from the round's frozen teams. Empty, incomplete, or unconfirmed owner sets are
rejected with stable conflict codes.

Corrections remain available while a round is completed. A correction removes
that scorecard's current confirmation, so the round cannot be locked until it is
confirmed again. Once locked, ordinary score changes remain rejected. Migration
4 also fails fast when upgrading a database that already contains an invalid
completed or locked round.

### Member round details

Open a round from the tournament page to see its flighter: names, start times,
starting holes and stored members. Missing schedules are explicitly marked as
not set. Scramble and foursomes show their score-owning teams separately, without
using legacy team schedule fields. Individual rounds show flights, not an empty
team list; any older individual grouping is labelled separately. Missing course
or tee setup, empty groups, loading and retryable errors have explicit states.

**Se rundens resultater** opens this exact round's net results. After opening,
**Åpne scorekort** opens this round's scorecard summary; the scoring workspace
decides the authorized owner and whether editing is allowed. Drafts omit that
link because the scoring workspace only accepts opened rounds. Flight times and
starting holes never confer scoring authority. No raw scores are loaded by this
detail page; progress respects the server's role-specific visibility projection.
Exact tournament
admins retain the round-specific **Administrer runden** link.

Live tournament events refresh the shared private queries. Failed reads hide
retained detail data, and conflicting round/pairings status or format prompts
refresh instead of combining inconsistent snapshots.

**Fremdrift per flight** appears below the setup. Drafts explain that progress
starts after opening. Opened rounds load the existing visibility-projected
completion read and join exact preserved player/team owners to stored flights.
Shared team cards count once, even with two teams in one flight. Each flight
shows registered/required hole entries across its cards and per-card progress;
full projections also show completed and confirmed card counts. These counts are
not the current physical hole or a claim about consecutive holes played.

For non-admin members, when the final back nine is hidden, the overview says that only holes
1–9 are visible. It shows only visible-hole entries: no completion, confirmation,
readiness or inferred full-round percentages, even for a completed/locked round.
Exact tournament admins retain full progress while the final is hidden.
Release, re-hide, reconnect and stream errors use the existing fail-closed private
completion cache. Score changes and confirmations refresh through SSE.

Old rounds without preserved flight assignments display an unavailable-mapping
message rather than guessing from legacy groups. Failed reads suppress retained
progress and offer retry; drafts, empty owner sets and pending reads have explicit
states. Current roster activity and team membership never replace historical IDs.

Repeat flight-progress browser checks on a fresh disposable seed with the same
local prerequisites as the round-details suite:

```bash
GOLF_FLIGHT_PROGRESS_BROWSER=1 npm run test:browser:progress
```

This suite opens rounds, submits seed scores, confirms/corrects/reconfirms a card,
and releases/re-hides the final for a live member view. It covers all formats and
320/390/1280px widths. Loading, empty, error and long-name states are injected.
Screenshots use `/tmp/golf-flight-progress-*`; never run against hosted data.

For the opt-in Chrome checks, use the same local prerequisites described below
and a freshly migrated/seeded disposable database, then run from `frontend/`:

```bash
GOLF_ROUND_DETAILS_BROWSER=1 npm run test:browser:rounds
```

This suite logs in with local seed identities, starts the tournament and opens
an individual round. It exercises all three formats and real SSE/navigation;
loading, empty, long-content and denied-read states use scoped response injection.
Screenshots go to `/tmp/golf-round-details-*`; failure artifacts use the shared
`/tmp/golf-lifecycle-browser-results` directory. Do not use hosted data.

### Administrator round controls

Open **Administrasjon → Livsløp → Administrer en runde**, or use
**Administrer runden** on a round detail page. Only the exact tournament admin
receives these controls. The management URL retains the selected round as
`?round={round_id}#lifecycle`; another tournament's round identifier is not
silently replaced with an actionable round.

- **Åpne runden** is available for a ready draft round in an active tournament.
  The readiness overview names missing/invalid entrants, teams and flights and
  links to the relevant setup section while retaining the selected round.
  Course and pairing links expand that round's existing editor. Confirmation
  explains that opening freezes configuration and captures handicap snapshots.
- **Fullfør runden** requires every necessary player/team card to be complete
  and confirmed. The overview shows scored/required holes and confirmation state.
  **Fyll ut scorekort** and **Bekreft scorekort** open the exact tagged owner in
  the existing score workflow; **Les scorekort** uses the separate read-only
  result-card route. Completion makes results count in tournament standings;
  corrections remain possible until locking.
- **Lås runden** requires a completed round whose cards remain confirmed.
  A subsequent correction removes confirmation, so its card must be confirmed
  again. Locking requires explicit confirmation and leaves the round read-only;
  there is no reopen, unlock or locked-score correction control.

Confirmation moves keyboard focus to **Avbryt**. Escape cancels and restores
focus; saving prevents duplicate submission and announces the server-confirmed
result. Read failures, inconsistent status, unavailable authority and pending
checks disable mutations. **Oppdater kontrollen** reconciles status, readiness
and related private data after failures, conflicts or uncertain responses.
Another administrator's changes and score corrections refresh the same data
through SSE. A newer live refetch may supersede reconciliation without reporting
a false failure; reads started before a mutation outcome cannot restore an older
actionable status.

Opening keeps any mounted unsaved manual course draft visible but disabled;
those local fields are not the persisted course revision and cannot be saved
after opening. Existing pairing drafts retain their explicit conflict handling.
Final-back-nine release/re-hide remains a separate administrator control:
completion and locking never change its visibility.

### Repeating the lifecycle browser checks

With Node 22.13+ and Google Chrome installed, use a freshly migrated and seeded
**disposable local database**, an API on port 3000 with development cookies, and
the frontend on `http://127.0.0.1:5173`. Run from `frontend/`:

```bash
GOLF_LIFECYCLE_BROWSER=1 npm run test:browser:lifecycle
```

The opt-in suite uses development seed identities and changes their tournament,
scores and round states. It requires all seeded rounds to begin draft and leaves
round four draft for injected loading/error/empty-state checks. Use a fresh
disposable database for a complete repeat; do not run it against hosted data.
Screenshots and failure artifacts go under `/tmp/golf-lifecycle-*`; traces are
disabled to avoid retaining authentication payloads. The normal frontend test
command runs the unit and React Testing Library tests without these mutations.

### Administrator-controlled final visibility

Migration `0018` replaces the former deadline with
`tournaments.final_round_back_nine_hidden` and a dedicated
`visibility_updated_at` concurrency token. New tournaments default to hidden.
During upgrade, only completed or locked finals whose schema-17 deadline had
already expired remain released; every other tournament stays hidden. The old
deadline column, helper functions, and confirmation-maintenance triggers are
removed.

`GET /api/tournaments/{tournament_id}/final-round-visibility` returns the focused
private resource to the exact tournament admin. `PATCH` accepts an absolute
`back_nine_hidden` value and `expected_visibility_updated_at`, revalidates the
active session and exact admin membership under the established
rounds-before-tournament lock order, and returns
`final_round_visibility_stale` for a superseded token. PostgreSQL independently
requires the transaction-local exact-admin workflow and owns the next timestamp.
Idempotent writes do not emit an event; changed writes emit one identifier-free
`visibility` event only after commit.

The shared read policy gives exact admins full projections. Every other role sees
only holes 1–9 for a hidden open, completed, or locked configured 18-hole final.
Release and re-hide are explicit admin actions; confirmation, completion,
locking, and elapsed time never change visibility. The management workspace
shows the control only for a configured 18-hole final and recovers stale saves by
refetching the focused resource.

## Leaderboards

Round leaderboards are available at
`GET /api/rounds/{round_id}/leaderboards/gross` and `/net`. They return all
preserved player owners for individual play or frozen round teams for scramble,
including unstarted cards. Live positions compare the selected partial total to
par for the holes actually scored. Equal scores use competition positions and
holes played affect display order, not the tie itself. Net totals allocate the
preserved or calculated playing handicap by each scored hole's stroke index.

Tournament leaderboards are available at
`GET /api/tournaments/{tournament_id}/leaderboards/gross` and `/net`. They include
all registered players, including withdrawn and zero-result entries. Each metric
independently selects the displayed best N from completed or locked history plus
the visible scored portion of the deterministic highest-numbered open round. An
unstarted open card contributes nothing; partial and full-but-open cards remain
explicitly provisional. When configured, the mandatory round always uses one of
N slots once it has a visible result. A completed mandatory result counts
regardless of score; if it is missing, no extra optional result fills that slot
for qualification. With N = 1, only the mandatory result can qualify.

Completed-only counted progress is the first ranking key until N. Equal progress
then compares the requested metric's displayed score-to-par, which may include a
selected provisional result. Provisional hole progress stabilizes display order
inside a sporting tie but does not break the tie. Names, UUIDs, and the other
metric never enter the sporting tie identity.

The response returns `required_counted_rounds`, nullable `mandatory_round_id`, and every player's complete
round-ordered contribution history. Each contribution preserves its round ID,
tagged player or team owner, owner name, provisional state, visible hole progress,
gross/net/par totals, metric score-to-par, displayed-selection state, and
mandatory flag. Aggregate totals cover only the selected subset.
`completed_rounds`, `counted_contributions`, and `eligible` remain completed-only
qualification facts even when a provisional result is selected. Individual
results stay with their snapshot owner; scramble and foursomes results are
attributed once to every frozen member of that exact round team. Current-team
data and provisional contributions come only from the highest-numbered open
round. `included_round_ids` remains completed/locked-only; provisional identities
match the separate `current_round_id`.

Completed status remains authoritative for qualification. A completed-round
score correction updates its contribution immediately even though the changed
card must be reconfirmed before locking; the selected open contribution remains
provisional until lifecycle completion. Current player handicaps are never read
for historical net totals. All leaderboard reads use one repeatable-read snapshot
and bounded bulk queries; inconsistent completed or open-round owner data fails
closed instead of producing plausible partial standings.
UI wording distinguishes these states without changing calculations: visible
open-round scores may contribute provisionally, completed/locked rounds qualify,
and the displayed best-N selection can change while play continues. Qualification
progress is labelled separately from included/excluded displayed contributions;
excluded results are retained, not discarded. A mandatory round reserves one
counted slot even when it is not among the best scores. A scorecard with every
hole entered is labelled **Alle hull ført**, not a completed round. Confirmation,
round completion, locking and independent final-nine release remain separate.
Both round and tournament leaderboard routes require membership in the target
tournament and are returned as private, non-cacheable responses.

Exact tournament admins receive full standings. For every other role, a hidden
open 18-hole final is calculated from holes 1–9 only. A hidden completed or
locked final is omitted entirely from tournament best-N selection, totals,
eligibility, positions, ties, and included-round identities.
Round totals and ranks are recomputed after redaction while retaining the full
18-hole handicap allocation denominator. A non-admin provisional final result
therefore contains at most holes 1–9, while the exact admin projection remains
full.

The React `/leaderboard` page stores tournament, round/tournament scope, round,
and gross/net selection in URL search parameters. Invalid or stale selections
are replaced with a valid active/latest default before a leaderboard query is
enabled, including validation that the selected round belongs to the selected
tournament. Round rows distinguish unstarted, partial, complete, and confirmed
cards; tournament rows retain registered players with zero completed rounds and
label the named mandatory round as completed, open, awaiting final release, or
missing without inferring a hidden player's result.

Every tournament row links to
`/tournaments/{tournament_id}/results/players/{player_id}?metric=...`. This
protected page reuses the canonical metric-specific tournament leaderboard and
shows every visibility-projected contribution in round order, including its
preserved player/team owner, gross/net/par totals, completed qualification, and
explicit included/excluded-from-displayed-total, provisional, and mandatory state. Each contribution
links to
`/tournaments/{tournament_id}/rounds/{round_id}/scorecards/{owner_type}/{owner_id}`;
scored round-leaderboard rows link directly to the same read-only route.

The direct-card page verifies that the round belongs to the path tournament and
that the tagged owner exists in the role-projected round leaderboard before
requesting the canonical member scorecard read. Metric, summary/hole view, and
visible hole are canonical URL state. The page uses SSE-driven authoritative
visibility refresh, but never requests score access, completion validation,
`/scoring`, confirmation, or a mutation. A history/card deep link is therefore refresh-safe
and membership-private without granting the viewer write authority.

Leaderboard responses cross a focused runtime decoder before entering TanStack
Query. The decoder checks tagged owners, finite states, identifiers, nullability,
numeric fields, response identity, and aggregate coherence across contribution
counts, selected sums, metric score-to-par, eligibility, and current-round facts.
The tournament page first refetches and awaits the exact rounds query, then loads
and validates standings against that same lifecycle snapshot. This prevents an
SSE open/completion transition from composing fresh standings with stale round
status.
Each protected page opens at most one EventSource for its selected tournament.
It remains an invalidation signal only; clients refetch the selected authoritative
state instead of calculating or merging score state in the browser. The stream
requires exact membership, revalidates the active session and membership before
each matching event, and emits only an event type plus the fixed `invalidate`
marker, with no identifiers or mutable state.
Initial connection and every reconnect invalidate the current user's private
workspace. Visibility events and stream errors first clear every role-projected
result query, preventing a previously released back nine from remaining visible
while disconnected or refetching; authorized `/scoring` queries are excluded.
A lagged server receiver closes the stream so native reconnection
triggers that same authoritative resync instead of silently leaving stale state.

## Development workflow

Follow `README.md` for setup and commands. Agents and contributors must also read
the root and applicable nested `AGENTS.md` files. Meaningful implementation work
is plan-gated through `docs/PLANS.md` and follows the loop in
`docs/AGENT_WORKFLOW.md`.
`docs/PLANS.md` contains only active and queued work; durable technical decisions
belong in `docs/ARCHITECTURE.md`, while this file owns current behavior and
operator-facing contracts. The production deployment, migration, backup,
restore, and rollback procedures are maintained in `docs/deployment_guide.md`.

## Known limitations

- The legacy global player/profile/handicap directory is retired. Tournament
  creation grants authority only over the new trip, not other users' trips.
  Scorecards and target-tournament SSE are
  membership-private. Later public tournament, scorecard, or leaderboard access
  requires an explicit share-token contract.
- Request throttling is process-local, so the supported production topology is
  one API replica. A future multi-replica topology requires a shared limiter.
- Tournament settings currently edit only the atomic pre-start best-N and
  optional mandatory-round configuration and expose the explicit
  tournament-start action. Explicit completion and archive APIs have administrator
  confirmation controls; the tournament list offers current/archive/all views.
  General tournament editing remains unimplemented. The Courses section supports
  draft-round configuration; non-draft rounds are deliberately read-only.
- Pairing roster reads, atomic admin replacement, the mobile draft editor,
  flight-aware opening readiness, and representative ready seed assignments
  exist together with membership-wide scoring authority. There is still no
  offline score queue or public leaderboard link.
