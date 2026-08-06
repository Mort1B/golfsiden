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
The mobile score view supports immediate hole entry, correction, confirmation,
and locked read-only cards for both scoring formats. Administrative forms and
authentication are planned but not implemented.

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
- Opening a round captures the current handicap index and calculated course and
  playing handicaps for every active entrant. Later player handicap changes do
  not change these snapshots.
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
`owner` is tagged as `player` or `team`; `submitted_by` is temporarily explicit
until authentication supplies the actor. Same-value retries preserve the original
submitter, timestamp, confirmation, audit count, and SSE state. Changed strokes
append an audit row and invalidate any current scorecard confirmation.

`GET /api/rounds/{round_id}/scorecards/{owner_type}/{owner_id}` returns every hole
in order with partial gross/net totals, the preserved playing handicap,
completeness, and current confirmation. Individual net uses the opening snapshot.
Scramble net applies 35%/15% to the members' rounded course handicaps and applies
the round allowance once.

`POST` to that scorecard path plus `/confirm` requires all holes and records
`confirmed_by` and `confirmed_at`. Confirmation records represent current state;
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
while locked rounds are read-only. `VITE_SCORER_USER_ID` temporarily supplies the
seeded submitting user for attribution and leaves the UI read-only when missing
or malformed. This public build-time value is not an access-control boundary.

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

- No authentication or authorization enforcement.
- No admin UI for tournaments, players, courses, rounds, or teams.
- No offline score queue or public leaderboard link.
