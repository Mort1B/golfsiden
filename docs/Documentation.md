# Project documentation

## Current product state

Milestone 1 provides a Rust/Axum API, PostgreSQL schema and migrations, an
idempotent development seed, and a strict TypeScript React viewer application.
Milestone 2 now includes deterministic round opening, backend score entry,
correction, scorecard summary and confirmation, plus atomic round completion and
locking and live gross/net leaderboard APIs for individual stroke play,
two-player scramble, and two-player foursomes. Signed-in tournament members can browse their tournaments,
tournament players, rounds, round-specific teams, and gross/net standings. The
mobile result view supports shareable selections and live refetch inside that
private workspace. Global player reads are retired.
The mobile score view supports authenticated hole entry, correction,
confirmation, locked read-only cards, and every writable card in the signed-in
player's exact flight for all three scoring formats.
Tournament-scoped memberships now authorize entrant, round, team, lifecycle,
handicap, and score mutations as well as tournament workspace reads. A first-time
visitor can create their account, player identity, draft tournament, complete
round plan, admin membership, initial invitation, and session through one mobile
onboarding flow.
Shared invitation links now support minimal preview, atomic new-player
registration, exact linked-player acceptance, and tournament-admin issue,
rotation, and revocation from mobile-first React views.
Accounts now use username and password only. Tournament admins can correct a
registered handicap with an audit reason until the first round opens; the
handicap is permanently fixed for that tournament afterward.
An exact tournament admin can now start a ready draft tournament from its
management workspace. Tournament start and round opening are separate lifecycle
actions: start activates the tournament and freezes pre-start settings, while
every round remains draft until it is explicitly opened.

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
- `.codex/agents/`: project-specific specialist agent definitions.
- `docs/PLANS.md`: the only active implementation plan.

## Preserved domain behavior

- Tournament players retain identity and accumulated results across changing
  round teams.
- Opening a round captures the tournament entrant's fixed handicap and the
  calculated course and playing handicaps. A tournament admin may make audited
  corrections before the first opening, but no correction is possible after any
  round has opened or snapshot has existed.
- Team membership is unique per player and round.
- Scores have exclusive player/team ownership.
- Locked-round score mutations require an explicit admin correction setting.
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

`GET /api/rounds/{round_id}/scorecards/{owner_type}/{owner_id}` returns every hole
in order with partial gross/net totals, the preserved playing handicap,
completeness, and current confirmation. Individual net uses the opening snapshot.
Scramble net applies 35%/15% to the members' rounded course handicaps and applies
the round allowance once. Foursomes net reads the immutable team Playing Handicap
captured at opening; it never recalculates from rounded member snapshots. This
read requires an active session plus any exact membership role in the round's
tournament. Round lookup, session revalidation, membership `FOR SHARE`, and card
assembly share one repeatable-read transaction; successful responses are
`private, no-store`.

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
transaction before it assembles the owner list.

## Authentication and scoring access

Accounts authenticate with a canonical lowercase username and password. A
username contains 3-32 ASCII lowercase letters, digits, underscores, or hyphens;
the API trims and lowercases accepted input. Account email is neither required
nor stored. The optional `players.email` field remains profile contact data and
does not participate in authentication or identity linking.

Login creates a revocable server session and returns an opaque token only in an
`HttpOnly`, `SameSite=Lax` cookie. PostgreSQL stores only the token's SHA-256
hash. Auth responses are not cacheable, Argon2 password verification runs off
the async executor, and score mutations plus logout require the session-derived
CSRF token. Score writes lock and revalidate the session in their existing round
transaction, so logout cannot complete before an already-authorized write.

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

Management and lifecycle mutations resolve the target tournament from the
resource and lock/revalidate both the session and admin membership inside the
write transaction. The former global player/profile/handicap routes and legacy
`POST /api/tournaments` platform-admin route are no longer registered. Player
discovery is available only from a target tournament's private roster, and
product-facing tournament creation uses creator onboarding. `GET
/api/me/tournaments` returns only the active user's tournament roles and linked
entrant identities.

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
The creator chooses how many rounds count; the wizard defaults to all configured
rounds, preserves a smaller explicit best-N choice while rounds are edited, and
submits `counted_rounds` within `1..=number_of_rounds`.
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

## Invitation onboarding

`POST /api/invitations/{invitation_id}/preview` authenticates the fragment token
from a JSON body before revealing only the tournament name, dates, and invitation
expiry. New visitors use `/register`; the server preflights the link before
Argon2, then atomically creates the account, player, both handicap histories,
entrant, player membership, append-only redemption, and session. Authenticated
visitors use the CSRF-protected `/accept` route, which relies only on the exact
session-linked player and never infers identity from email.

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
surfaces. Settings exposes “Tellende runder” while the tournament and every
existing round remain draft. `PATCH /api/tournaments/{tournament_id}/counted-rounds` requires the
current tournament timestamp, CSRF, and exact tournament-admin membership. It
locks the round set before the tournament, rejects stale writes, and permanently
freezes the value when the tournament starts or a legacy round has already
opened. PostgreSQL independently requires the admin workflow context and uses
the durable opening marker even if child rows are later removed. Success returns
the authoritative private tournament and publishes one payload-free invalidation
after commit.

The Lifecycle section exposes `Start turneringen` only to the exact tournament
admin. `POST /api/tournaments/{tournament_id}/start` requires CSRF plus the
current tournament `updated_at`. Under deterministic locks it revalidates the
active session and exact `admin` membership, requires rounds numbered exactly
`1..=number_of_rounds`, requires every round to remain draft, and requires at
least one registered, non-withdrawn player whose account is active. Course, tee,
pairing, and flight readiness are deliberately deferred to round opening.

A successful start changes only the tournament from `draft` to `active`, returns
the authoritative private/non-cacheable tournament, updates the identity-scoped
query cache, and publishes one payload-free tournament invalidation after commit.
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

The approved configuration fallback for a missing or incomplete provider course
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
eight entries to manual entry while remaining ready for a future verified row.

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
successful commit emits one payload-free round event. Errors, including
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

`GET /api/rounds/{round_id}/completion-validation` returns a repeatable-read,
deterministically ordered view of every required player or team scorecard. It
reports holes scored, required holes, confirmation state, and separate
`ready_to_complete` and `ready_to_lock` flags for every existing round state.

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
all registered players, including withdrawn and zero-result entries, but aggregate
only completed or locked rounds. Each metric independently selects the configured
best N score-to-par contributions. Players with fewer counted results rank behind
players with more until they reach N; equal counted-result counts then compare
only the selected metric's score-to-par total. Competition ties ignore names and
the other metric, while case-insensitive name and UUID stabilize display order.

The response returns `required_counted_rounds` and every player's complete
round-ordered contribution history. Each contribution preserves its round ID,
tagged player or team owner, owner name, gross/net/par totals, metric
score-to-par, and counted state. Aggregate totals cover only the subset selected
for that requested metric. `completed_rounds`, `counted_contributions`, and
`eligible` distinguish provisional players from those with N results. Individual
results stay with their snapshot owner; scramble and foursomes results are
attributed once to every frozen member of that exact round team. Current-team
data still comes from the highest-numbered open round, but open-round scores are
not contributions in this boundary.

Completed status is authoritative for tournament totals. A completed-round score
correction therefore updates the leaderboard immediately even though the changed
card must be reconfirmed before locking. Current player handicaps are never read
for historical net totals. All leaderboard reads use one repeatable-read snapshot
and bounded bulk queries; inconsistent completed-round or owner data fails closed
instead of producing plausible partial standings. Supplied open-round facts also
retain fail-closed validation even though they do not yet affect tournament
totals.
Both round and tournament leaderboard routes require membership in the target
tournament and are returned as private, non-cacheable responses.

The React `/leaderboard` page stores tournament, round/tournament scope, round,
and gross/net selection in URL search parameters. Invalid or stale selections
are replaced with a valid active/latest default before a leaderboard query is
enabled, including validation that the selected round belongs to the selected
tournament. Round rows distinguish unstarted, partial, complete, and confirmed
cards; tournament rows retain registered players with zero completed rounds.

Leaderboard responses cross a focused runtime decoder before entering TanStack
Query. The decoder checks tagged owners, finite states, identifiers, nullability,
numeric fields, response identity, and aggregate coherence across contribution
counts, selected sums, metric score-to-par, eligibility, and current-round facts.
Each protected page opens at most one EventSource for its selected tournament.
It remains an invalidation signal only; clients refetch the selected authoritative
state instead of calculating or merging score state in the browser. The stream
requires exact membership, revalidates the active session and membership before
each matching event, and emits only an event type with no identifiers or data.
Initial connection and every reconnect invalidate the current user's private
workspace. A lagged server receiver closes the stream so native reconnection
triggers that same authoritative resync instead of silently leaving stale state.

## Development workflow

Follow `README.md` for setup and commands. Agents and contributors must also read
the root and applicable nested `AGENTS.md` files. Meaningful implementation work
is plan-gated through `docs/PLANS.md` and follows the loop in
`docs/AGENT_WORKFLOW.md`.

## Known limitations

- The legacy global player/profile/handicap directory and platform-admin
  tournament creation are retired. Scorecards and target-tournament SSE are
  membership-private. Later public tournament, scorecard, or leaderboard access
  requires an explicit share-token contract.
- Request throttling is not implemented, so the public onboarding and
  registration endpoints are not ready for an internet-facing deployment.
- Tournament settings currently edit only the pre-start counted-round value and
  expose the explicit tournament-start action; general tournament editing and
  later completion/archive controls remain unimplemented. The Courses section
  supports draft-round configuration; non-draft rounds are deliberately read-only.
- Pairing roster reads, atomic admin replacement, the mobile draft editor,
  flight-aware opening readiness, and representative ready seed assignments
  exist together with membership-wide scoring authority. There is still no
  offline score queue or public leaderboard link.
