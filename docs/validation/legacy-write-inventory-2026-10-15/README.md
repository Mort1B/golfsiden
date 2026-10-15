# A1o — legacy team/round write authority inventory

Reviewed source revision: `5b79760247b221232fc64653a061be363de17866`.
Date: 2026-10-15. This is a documentation-only source assessment. No application,
test, schema or dependency source changed, no request was sent and no new runtime
race was reproduced. It follows the [application review](../application-review-2026-10-14/README.md)
and is separate from the completed [private-read inventory](../private-read-inventory-2026-10-14/README.md).

## Result and severity

Six explicit mutation method/path pairs in `api/rounds` are reachable: three have
a final session-expiry check and three lack it. No selected reachable route has
unresolved authority ownership. The three granular team-write paths are retired;
their six retained repository helpers have no callers in current backend source
or tests. Those helpers are not evidence of a reachable HTTP vulnerability.

| Finding | Severity | Reachable effect | Evidence strength |
| --- | --- | --- | --- |
| W1: direct round creation lacks a final expiry check | P2 medium | An initially active exact admin can create a draft round if its session naturally expires during membership or later INSERT work. | Source-supported; no new race reproduction |
| W2: pairing replacement lacks a final expiry check | P2 medium | An initially active exact admin can replace a draft roster and receive its assembled layout after expiry during membership, validation, writes or response loading. | Source-supported; no new race reproduction |
| W3: course attachment lacks a final expiry check | P2 medium | An initially active exact admin can create an immutable course hierarchy and attach it to a draft round after expiry during membership or subsequent SQL. | Source-supported; no new race reproduction |

These share one missing boundary, but require separately bounded repairs. P2 is
appropriate because the effects are persistent administrative writes under
expired authority. The actor must first have an active session, valid CSRF and
exact tournament-admin membership. No arbitrary-user, cross-tournament,
completed-revocation or locked-round bypass is established. No new P0/P1 finding
was identified in this selected source scope; that is not an application-wide
security sign-off or a performance assessment.

## Scope, reachability and authority definition

The [machine-readable manifest](inventory.json) records the six live routes,
three retired paths, ten retained non-HTTP write helpers, three preflights and
their actual backend callers. Counts exclude GET, generated HEAD/OPTIONS and
helpers as HTTP routes. The production router merges
[rounds](../../../backend/src/api/mod.rs#L59) and
[teams](../../../backend/src/api/mod.rs#L65); the
[round route owner](../../../backend/src/api/rounds/mod.rs#L43) registers all six
selected mutations. Score/confirmation, Fantasy, awards, format settings,
invitation/profile and tournament aggregate/lifecycle writers remain outside
this legacy route scope. Child rows created by a selected write are traced only
to establish its atomic effects and guard ownership.

Every live route uses [MutationSession](../../../backend/src/api/auth.rs#L74):
active cookie extraction plus CSRF verification. This is admission, not retained
transactional authority. The final write transaction calls
[require_tournament_admin](../../../backend/src/repositories/tournament_authorization.rs#L120),
which derives the user from the actual session, checks revocation, credential
generation and database wall-clock expiry with
[session/user share locks](../../../backend/src/repositories/auth.rs#L102),
then obtains the exact admin membership share lock. There is no global-admin
bypass. [require_round_admin](../../../backend/src/repositories/tournament_authorization.rs#L151)
first locks the stored round and derives its tournament, then performs that
session/admin check. Expiry while waiting for that initial round lock is
therefore already checked; the findings concern waits after the session query.

Retained session/user/membership locks serialize later conflicting revocation,
credential and membership writes. Natural expiry needs no row update and can
pass while the membership query or subsequent work waits. A route is **bound**
here when it rechecks the same session after dependent transaction work and
immediately before calling commit. **Incomplete** means that final check is
absent; it does not mean the route has no authorization. Even a bound route does
not freeze time during COMMIT, deferred constraints or network delivery.

## Reachable route traces

### W1 — direct round creation: incomplete, P2

`POST /api/tournaments/{tournament_id}/rounds` uses a
[session-aware preflight](../../../backend/src/api/rounds/mod.rs#L149) before
content/body and semantic validation, then calls
[create_authorized](../../../backend/src/api/rounds/mod.rs#L162) with the same
session UUID. The [write transaction](../../../backend/src/repositories/rounds.rs#L136)
reacquires actual session and exact-admin authority, executes the bound-parameter
INSERT/RETURNING, then [commits](../../../backend/src/repositories/rounds.rs#L169)
without another active-session query. Thus losing authority between preflight
and transaction entry is already rejected; expiry during the membership wait
inside that final guard or during the INSERT is the remaining window.

Round insert guards enforce the
[parent's nonclosed state](../../../migrations/0019_tournament_completion.sql#L128),
[tournament dates](../../../migrations/0033_tournament_details.sql#L54) and
[overall eligible configuration](../../../migrations/0032_match_overall_configuration.sql#L32).
They check domain facts, not this session's expiry. If a Fantasy game exists,
the [insert trigger](../../../migrations/0035_fantasy_selections.sql#L183) also
creates its round window; its
[window guard](../../../migrations/0037_fantasy_guards.sql#L17) checks window facts,
not request authority. Those effects belong to the same transaction. An event
is [published only after success](../../../backend/src/api/rounds/mod.rs#L188).

### W2 — aggregate pairing replacement: incomplete, P2

`PUT /api/rounds/{round_id}/pairings` passes the actual session from the
[handler](../../../backend/src/api/rounds/pairings.rs#L115) through
[replace](../../../backend/src/repositories/round_pairings/mod.rs#L86) to
[execute](../../../backend/src/repositories/round_pairings/replace/mod.rs#L16).
Under the round lock it reauthorizes the session/admin, checks draft state and
the optimistic timestamp, validates identities/roster/legacy references, then
[replaces groups and memberships](../../../backend/src/repositories/round_pairings/replace/writes.rs#L15).
It updates the round timestamp and reloads the full layout before
[commit](../../../backend/src/repositories/round_pairings/replace/mod.rs#L47).
No later active-session check follows those operations.

The [pairing trigger](../../../migrations/0002_round_opening.sql#L99) locks stored
old/new round parents in deterministic order and rejects nondraft changes;
[team/member constraints](../../../migrations/0001_initial_schema.sql#L132) and
[flight triggers](../../../migrations/0011_round_flights.sql#L65) preserve scope
and roster invariants. They do not validate request-session expiry. This remains
the only supported roster writer; it does not invoke the retired team helpers.

### W3 — course configuration: incomplete, P2

`PUT /api/rounds/{round_id}/course-configuration` obtains/validates a selected
revision before passing the actual session to
[configure](../../../backend/src/api/rounds/configuration.rs#L141).
The [final transaction](../../../backend/src/repositories/round_configuration.rs#L76)
locks the round, reauthorizes session/admin, repeats draft/version and format
checks, calls [insert_in_transaction](../../../backend/src/repositories/course_revisions.rs#L48)
to insert course, tee and holes, and updates the round attachment. It
[commits](../../../backend/src/repositories/round_configuration.rs#L123) without
another active-session query. Expiry during that hierarchy work is not checked.

[Configuration protection](../../../migrations/0002_round_opening.sql#L27) freezes
nondraft round configuration, and
[revision guards](../../../migrations/0010_course_revisions.sql#L39) enforce
immutability and deferred completeness. Neither owns request-session expiry.
The new hierarchy and round update are atomic; the handler notifies after the
repository returns. This finding does not establish a partial revision leak.

### Open, complete and lock: bound, no new finding

The three POST routes pass the actual session to
[open_authorized](../../../backend/src/api/rounds/lifecycle.rs#L44),
[complete_authorized](../../../backend/src/api/rounds/completion.rs#L46) and
[lock_authorized](../../../backend/src/api/rounds/completion.rs#L62).
The [opening wrapper](../../../backend/src/repositories/round_lifecycle/mod.rs#L77)
rechecks active authority after opening and all snapshot work. The
[completion wrapper](../../../backend/src/repositories/round_completion.rs#L219)
does so after the common transition returns, including the separate singles-match
branch. Both retain their existing round/session/admin locks and final
wall-clock check immediately before commit. These are source-confirmed A2
boundaries; the [A2 reproduction and acceptance](../round-lifecycle-session-2026-10-14/README.md)
are historical evidence, not new checks in this step.

## Preflights and retained helpers

The pairing [preflight](../../../backend/src/repositories/round_pairings/mod.rs#L61)
and configuration [preflight](../../../backend/src/repositories/round_configuration.rs#L51)
take the extracted user ID, not the actual session. Their committed
repeatable-read admin/draft checks are provisional; final mutation transactions
independently require the real active session. Early stale/validation errors and
validation work are not postflight-session-bound. Configuration can also call
[fetch_course](../../../backend/src/api/rounds/configuration.rs#L129) between
preflight and final authorization. Conditional provider/cache/quota work could
occur before final rejection; this turn did not exercise it. The bundled catalog
still has zero usable entries, as recorded in the pinned
[course-read acceptance](../course-read-session-2026-10-15/README.md). Do not
infer a successful remote configuration request from that helper call. No
separate severity is assigned to these observations without demonstrated impact.

[teams::routes](../../../backend/src/api/teams.rs#L22) registers only GET. The
[retirement test source](../../../backend/tests/round_pairings.rs#L452) expects
team create POST to return 405 and member POST/DELETE to return 404. The retained
`teams::{create,assign_player,remove_player}` accept no identity and would require
caller-owned authority if reused. Their `_authorized` variants resolve the
stored round/team parent and exact session/admin authority, but have no final
expiry check after dependent work. All six have zero callers in backend source
and tests, including imported aliases. They are retained/unreferenced, not six
more HTTP findings. The domain triggers still apply to direct SQL.

The plain [rounds::create](../../../backend/src/repositories/rounds.rs#L115) also
has no callers. Plain [round_lifecycle::open](../../../backend/src/repositories/round_lifecycle/mod.rs#L70)
and [round_completion::{complete,lock}](../../../backend/src/repositories/round_completion.rs#L184)
are used by trusted test fixtures, not production handlers. They intentionally
accept no session; only their authorized wrappers serve the selected HTTP routes.
The manifest distinguishes these from reachable final authority owners and
records the caller-owned course-insertion/singles-transition inner helpers.

## Existing coverage and validation limits

These tests were read, **not run** for A1o:

| Family | Existing source coverage | Missing evidence for these findings |
| --- | --- | --- |
| Create | [Admission/preflight and commit/event cases](../../../backend/tests/round_creation_authorization.rs#L190); [revocation/membership loss after successful preflight](../../../backend/tests/round_creation_authorization.rs#L453) | Expiry after the final transaction's first session query, including pending insert effects |
| Pairings | [Stale/invalid/nonadmin effects](../../../backend/tests/round_pairings.rs#L374), [opening contention](../../../backend/tests/round_pairings.rs#L769), [membership loss before final guard](../../../backend/tests/round_pairings.rs#L822), [same-token concurrency](../../../backend/tests/round_pairings.rs#L903) | Expiry during membership/later validation, replacement or response loading |
| Course configuration | [Failed attachment rollback](../../../backend/tests/round_configuration.rs#L617), [same-token concurrency](../../../backend/tests/round_configuration.rs#L652), [membership/session loss before configure](../../../backend/tests/round_configuration.rs#L673) | Expiry after the final transaction's first session query with hierarchy/attachment pending |
| Lifecycle | [Membership and late-expiry scenarios](../../../backend/tests/lifecycle_expiry/scenarios.rs#L130) for three formats and all three actions | No new reproduction or acceptance claimed here |
| Retired team writes | [405/404 retirement assertions](../../../backend/tests/round_pairings.rs#L452) | No reachable mutation caller to assess |

Validation for this documentation-only step reconciles source routes/callers,
manifest counts, nonblank source anchors, local document links, scoped diff and
independent read-only review. Backend, PostgreSQL, migrations/seed, frontend and
browser ladders are unaffected and were not rerun. No database, provider,
dependency scanner or hosted environment was accessed. Source inspection does
not establish timing/frequency, a runtime exploit or a benchmark result.

Static route/count/reference/link checks and `git diff --check` passed across the
eight owned documentation/JSON files. All manifest source references match the
pinned unchanged revision. One unchanged historical A1f visibility anchor was
checked at that report's original revision, where it names `get_for_admin`,
rather than repointed at a different current function. Independent read-only
review confirmed the route/helper counts, timing distinctions, guard ownership,
severity and bounded next candidate; it reported no actionable findings.

## Bounded next candidate

Repair **W1 only**: add a final active-session wall-clock check to direct round
creation after INSERT/RETURNING and before commit, retaining its existing
session/admin locks and error mapping. First reproduce membership-wait and late
insert expiry against the current implementation in PostgreSQL/API tests, then
prove 401, complete rollback (including conditional child rows) and no event,
with valid-session controls preserving one creation/event. Preserve preflight,
CSRF, validation order, parent/domain guards and response contracts; add no
mutation retry. COMMIT/network time remains outside a final-check guarantee.

The direct POST has no production frontend consumer in the source caller
inventory. [tournamentApi.rounds](../../../frontend/src/api/tournaments.ts#L56)
only reads rounds. Existing-account creation uses the full-plan
[POST /api/tournaments](../../../frontend/src/api/tournamentCreation.ts#L13),
and new-account creation uses
[POST /api/onboarding/tournaments](../../../frontend/src/api/onboarding.ts#L125).
Those aggregate writers are excluded and unchanged. A1p therefore selects
backend/API/PostgreSQL acceptance without a new frontend/browser ladder; this
is a caller-scope decision, not a waiver of browser checks for UI changes.

The [plan](../../PLANS.md) specifies this candidate's scope and validation.
W2 and W3 require subsequent separate authorization. No repair, helper cleanup,
provider-preflight change, UI work or performance optimization began in A1o.
