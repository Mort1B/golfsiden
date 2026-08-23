# Project documentation

## Current product state

Milestone 1 provides a Rust/Axum API, PostgreSQL schema and migrations, an
idempotent development seed, and a strict TypeScript React viewer application.
Milestone 2 now includes deterministic round opening, backend score entry,
correction, scorecard summary and confirmation, plus atomic round completion and
locking and live gross/net leaderboard APIs for individual stroke play and
two-player scramble. Signed-in tournament members can browse their tournaments,
tournament players, rounds, round-specific teams, and gross/net standings. The
mobile result view supports shareable selections and live refetch inside that
private workspace. Global player reads remain a separate legacy surface.
The mobile score view supports authenticated hole entry, correction,
confirmation, and locked read-only cards for both scoring formats.
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
- SSE messages invalidate client queries; clients refetch authoritative data.

## Round opening

`GET /api/rounds/{round_id}/pairing-validation` reports stable readiness issue
codes, missing or ineligible entrants, and deterministic team sizes. A round
requires a configured course and tee, complete hole/stroke-index ranges, complete
active-player assignments, and valid group sizes. Scramble groups require exactly
two active players.

`POST /api/rounds/{round_id}/open` repeats validation while holding the round and
tournament transaction locks. It captures exact decimal handicap inputs, inserts
one immutable snapshot per active entrant, changes `draft` to `open`, commits, and
only then publishes an SSE invalidation. Concurrent opens cannot duplicate
snapshots. Database triggers freeze pairings and scoring configuration after draft
and require all status/snapshot changes to use the lifecycle transaction.

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
the round allowance once.

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
the current session may write. The browser filters the scoring selector with this
server result and never reproduces role or membership policy.

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
can write only their own individual card or their exact team in that round. Both
members of a two-player scramble team can therefore enter, correct, and confirm
the shared card, with their own account retained in the audit attribution.
Viewers, unlinked accounts, cross-tournament roles, and non-members cannot write.
Team identity remains specific to one round.

Tournament detail, roster, round, team, readiness, completion-validation, and
round/tournament leaderboard reads require an active session plus any membership
role in the target tournament. The tournament collection is selected directly
through the current user's memberships. Existing resources outside that scope
return `403`, missing resources return `404`, and successful private reads use
`Cache-Control: private, no-store`. Multi-query reads authorize and assemble the
response in one repeatable-read transaction while holding the membership row
`FOR SHARE`, so membership removal cannot commit partway through a response.

Management and lifecycle mutations resolve the target tournament from the
resource and lock/revalidate both the session and admin membership inside the
write transaction. Global player/profile mutations and the temporary legacy
tournament-creation route remain platform-admin-only. `GET /api/me/tournaments`
returns only the active user's tournament roles and linked entrant identities.

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

Authenticated tournament administrators enter the read-only management index at
`/manage/tournaments/{tournament_id}`. The route confirms both the canonical
tournament detail and the current user's tournament-specific `admin` membership
before enabling roster or round reads. Signed-out visitors retain the complete
return URL through login. Invalid identifiers, missing tournaments, non-admin
memberships, loading, retryable failures, empty collections, and populated data
have distinct states. Client gating controls presentation only; every private
read and invitation mutation remains protected by backend membership policy.

The workspace provides semantic anchors for settings, entrants, invitations,
rounds, courses, pairings, and lifecycle. It reports only facts already preserved
by the existing private APIs and links to the existing invitation and round
surfaces. It does not infer course revisions, load teams per round, or expose
unsupported mutation controls. Returning from invitation
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
complete unique stroke-index permutation. Hole distance is optional.

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

The current management workspace remains read-only for course configuration;
the mobile picker and manual-entry form are the next frontend step. Because no
bundled catalog row is presently verified as usable, provider success is tested
through the normalized adapter and identical final repository transaction. The
production HTTP path continues to fail closed until a real row is reverified.

Flights are not represented yet. A future normalized round-flight model will
extend the same score-access resolver so a player may receive both team owners
in their flight. Equal starting holes or tee times are not treated as flights.

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
only completed or locked rounds. Individual results stay with their snapshot
owner; each scramble result is attributed once to every member of that exact
round team. Players with more attributed completed rounds rank before players
with fewer, then by the selected total. Current-team data comes from the
highest-numbered open round, independent of its scoring format.

Completed status is authoritative for tournament totals. A completed-round score
correction therefore updates the leaderboard immediately even though the changed
card must be reconfirmed before locking. Current player handicaps are never read
for historical net totals. All leaderboard reads use one repeatable-read snapshot
and bounded bulk queries; inconsistent completed-round or owner data fails closed
instead of producing plausible partial standings.
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
numeric fields, and response identity. The single application EventSource
remains an invalidation signal only; clients refetch the selected authoritative
leaderboard instead of calculating or merging score state in the browser.

## Development workflow

Follow `README.md` for setup and commands. Agents and contributors must also read
the root and applicable nested `AGENTS.md` files. Meaningful implementation work
is plan-gated through `docs/PLANS.md` and follows the loop in
`docs/AGENT_WORKFLOW.md`.

## Known limitations

- Global player reads, scorecard read visibility, and SSE event visibility still
  need explicit private/public policy decisions. Later public tournament or
  leaderboard access will use explicit share tokens.
- Request throttling is not implemented, so the public onboarding and
  registration endpoints are not ready for an internet-facing deployment.
- The tournament management workspace is read-only. Provider-backed course/tee
  administration and settings, pairing, and lifecycle editors are not implemented.
- No flight model, offline score queue, or public leaderboard link.
