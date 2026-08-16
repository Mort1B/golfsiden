# Project documentation

## Current product state

Milestone 1 provides a Rust/Axum API, PostgreSQL schema and migrations, an
idempotent development seed, and a strict TypeScript React viewer application.
Milestone 2 now includes deterministic round opening, backend score entry,
correction, scorecard summary and confirmation, plus atomic round completion and
locking and live gross/net leaderboard APIs for individual stroke play and
two-player scramble. Users can browse tournaments, tournament players, rounds,
players, and round-specific teams. The mobile result view supports round and
tournament gross/net standings with shareable selections and live refetch.
The mobile score view supports authenticated hole entry, correction,
confirmation, and locked read-only cards for both scoring formats.
Tournament-scoped memberships now authorize entrant, round, team, lifecycle,
handicap, and score mutations. A first-time visitor can create their account,
player identity, draft tournament, complete round plan, admin membership,
initial invitation, and session through one mobile onboarding flow.
Shared invitation links now support minimal preview, atomic new-player
registration, exact linked-player acceptance, and tournament-admin issue,
rotation, and revocation from mobile-first React views.

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
- Opening a round captures the tournament entrant's current handicap and the
  calculated course and playing handicaps. Tournament handicap changes are
  audited and affect only rounds opened afterward; existing snapshots never
  change.
- Team membership is unique per player and round.
- Scores have exclusive player/team ownership.
- Locked-round score mutations require an explicit admin correction setting.
- Score mutations are auditable in PostgreSQL.
- The initial two-player scramble formula is isolated in the domain layer and
  uses 35% of the lower plus 15% of the higher course handicap.
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

Management and lifecycle mutations resolve the target tournament from the
resource and lock/revalidate both the session and admin membership inside the
write transaction. Global player/profile mutations and the temporary legacy
tournament-creation route remain platform-admin-only. `GET /api/me/tournaments`
returns only the active user's tournament roles and linked entrant identities.

## Creator onboarding

`POST /api/onboarding/tournaments` accepts nested creator account/player data,
tournament dates, and one to thirty contiguous round definitions. It rejects
unknown privileged fields, already-authenticated callers, duplicate normalized
emails, invalid date ranges, unsupported formats, oversized bodies, and bounded
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
caches are cleared when sessions change.

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

Flights are not represented yet. A future normalized round-flight model will
extend the same score-access resolver so a player may receive both team owners
in their flight. Equal starting holes or tee times are not treated as flights.

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

- Public read routes still expose the pre-onboarding viewer model. They will
  become membership-scoped during the frontend cutover; later public access will
  use explicit share tokens.
- Creator email ownership and request throttling are not implemented, so the
  public onboarding and registration endpoints are not ready for an
  internet-facing deployment.
- No tournament management workspace beyond the creation flow.
- Course and tee administration are not implemented.
- No flight model, offline score queue, or public leaderboard link.
