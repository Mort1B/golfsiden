# Remaining private-read authority inventory — 2026-10-14

Follow-up: [A1g numeric read acceptance](../numeric-read-session-2026-10-14/README.md)
repairs the three numeric access/card GETs with reproduced late-expiry evidence.
[A1h four-ball acceptance](../four-ball-read-session-2026-10-14/README.md) repairs
both four-ball card GETs after reproducing late confirmation-loading expiry.
[A1i Stableford acceptance](../stableford-read-session-2026-10-14/README.md) closes
both Stableford card GETs with late confirmation-loading expiry evidence.
[A1j match card acceptance](../match-card-session-2026-10-15/README.md) closes
both detail/scoring GETs with final note-loading expiry evidence.
Four P2 routes and three P3 course routes remain. The classifications, manifest
and source anchors below preserve the original A1f assessment revision.

Reviewed source revision: `21917b6931c4f94dfacdbb36ded7f390f6108834`.
A1f is a **documentation-only source assessment**. No endpoint was called, no
runtime race was reproduced and no runtime, test, dependency or schema source
was changed. This inventory follows the [original application review](../application-review-2026-10-14/README.md)
and [A1e readiness repair](../readiness-session-2026-10-14/README.md).

## Findings by severity

| Finding | Severity | Scope and consequence | Evidence strength |
| --- | --- | --- | --- |
| R1: final expiry check absent | P2 medium | 12 reads: numeric access/cards (3), match details/completion/table (4), four-ball (2), Stableford (2), invitations (1). Retained authority locks serialize later revocation/credential changes, but natural expiry during subsequent loading or waits is not checked before success. | Source-supported; no new race reproduction |
| R2: session identity discarded | P2 medium | Final-round-visibility GET takes only an extracted user ID. Its exact-admin membership lock cannot revalidate revocation, credential generation or natural expiry for that session. The payload is administrative visibility metadata, not hidden scores. | Source-supported; no new race reproduction |
| R3: course reads lack complete session binding | P3 low | Three course GETs use user-only admin authorization. Payloads are global bundled/registered course facts; provider fetching is conditional on catalog readiness. No current private-score disclosure or reachable provider-quota abuse is established. | Source-supported; provider-success impact conditional |

The next candidate at this snapshot was **A1g: repair only the three numeric
score-access/card GETs**, beginning with a failing late-expiry regression. Other families need
separate bounded steps. Cache/retry differences below are recorded contract
facts; they do not independently establish a data disclosure.

## Coverage and definitions

The [route manifest](routes.json) accounts for **46 explicit GET method/path
patterns** reachable from [the API router](../../../backend/src/api/mod.rs#L40):

- 14 previously repaired GETs: four gross/net leaderboards, three tournament
  list/detail routes, roster and round list/detail (3), pairing/team (2), and
  readiness (2). These are bookkeeping exclusions, not newly executed acceptance.
- 27 remaining tournament-scoped JSON GETs traced here: **11 bound, 16 incomplete,
  zero unresolved**. Incomplete routes comprise 13 P2 routes and three P3 routes.
- Five excluded GETs: health, readiness, authentication session, self-only profile
  and the tournament SSE stream. Public capability routes and non-GET methods
  are outside this inventory. Automatic HEAD/OPTIONS behavior is not counted as
  another explicit GET route.

**Bound** means the source retains session/user and exact target membership/role
authority across protected database loading and performs a final active-session
wall-clock check before commit. It does not require every family to share the
same isolation/retry implementation, make GETs side-effect-free, or prove fresh
runtime/concurrency acceptance. **Incomplete** identifies a concrete missing
part of that boundary. **Unresolved** would mean tracing could not determine the
boundary; none remained in this source snapshot.

The [HTTP session extractor](../../../backend/src/api/auth.rs#L48) makes a
[nonlocking active-session lookup](../../../backend/src/repositories/auth.rs#L85).
That is an entry check, not authority retained through later loading. Central
[share/exclusive session checks](../../../backend/src/repositories/auth.rs#L102)
check revocation, credential generation and `clock_timestamp()` and lock both
session and user. Holding those row locks does not stop time advancing. Final
checks occur before COMMIT; none freezes time through commit, JSON serialization,
network delivery or previously delivered data.

`RR` below means explicit repeatable read. `D` means no isolation override in the
read boundary: it inherits the pool/database default (normally READ COMMITTED),
which was not queried in this source-only step. A single payload SQL statement
can own its consistent data snapshot without requiring RR across unrelated
statements. Lock placement and final checks are assessed for each family.

### Cache legend

- **A:** router response middleware sets `private, no-store`, including handler
  errors and extractor rejections for the matching route.
- **A\*:** numeric scoring middleware fills a missing cache header; existing
  `no-store` on authorization errors is retained. Every response is non-storeable.
- **N:** router response middleware sets `no-store` on all responses. This prohibits
  storage; the absence of literal `private` is not a cache defect.
- **S:** handler success sets `private, no-store`; common `ApiError` denials use
  `no-store` for 401/403/429, but other errors/extractor rejections have no universal
  route policy.
- **H:** success and handler-converted custom errors are `private, no-store`;
  early Auth/typed-Path extractor failures bypass that custom error wrapper.

The [common error mapper](../../../backend/src/error.rs#L146) supplies the limited
401/403/429 policy. These differences do not prove private payloads were cached.

## Complete remaining route matrix

Every path below uses `GET`; path parameter names match the callable source.
The evidence column identifies the corresponding handler/repository trace below.

| Exact path | Authority / severity | Evidence family | Cache |
| --- | --- | --- | --- |
| `/api/tournaments/{tournament_id}/course-catalog` | Incomplete / P3 | [courses](#courses) | H |
| `/api/tournaments/{tournament_id}/course-presets` | Incomplete / P3 | [courses](#courses) | H |
| `/api/tournaments/{tournament_id}/course-provider/courses/{provider_course_id}` | Incomplete / P3 | [courses](#courses) | H |
| `/api/tournaments/{t}/fantasy/results` | Bound | [fantasy](#fantasy) | N |
| `/api/tournaments/{t}/fantasy/rounds/{r}/results` | Bound | [fantasy](#fantasy) | N |
| `/api/tournaments/{t}/fantasy/results/golfers/{p}` | Bound | [fantasy](#fantasy) | N |
| `/api/tournaments/{t}/fantasy/results/managers/{u}` | Bound | [fantasy](#fantasy) | N |
| `/api/tournaments/{t}/fantasy` | Bound | [fantasy](#fantasy) | N |
| `/api/tournaments/{t}/fantasy/rounds/{r}` | Bound | [fantasy](#fantasy) | N |
| `/api/tournaments/{t}/fantasy/rounds/{r}/owners/{kind}/{owner}` | Bound | [fantasy](#fantasy) | N |
| `/api/rounds/{round_id}/four-ball/scorecards/{team_id}` | Incomplete / P2 | [four ball](#four-ball) | A |
| `/api/rounds/{round_id}/four-ball/scorecards/{team_id}/scoring` | Incomplete / P2 | [four ball](#four-ball) | A |
| `/api/tournaments/{tournament_id}/invitations` | Incomplete / P2 | [invitations](#invitations) | N |
| `/api/rounds/{round}/match-play/matches` | Bound | [match listing](#match-listing) | A |
| `/api/rounds/{round}/match-play/matches/{id}` | Incomplete / P2 | [match details](#match-details) | A |
| `/api/rounds/{round}/match-play/matches/{id}/scoring` | Incomplete / P2 | [match details](#match-details) | A |
| `/api/rounds/{round}/match-play/completion` | Incomplete / P2 | [match details](#match-details) | A |
| `/api/tournaments/{tournament}/match-table` | Incomplete / P2 | [match details](#match-details) | A |
| `/api/tournaments/{tournament_id}/player-accounts` | Bound | [accounts](#accounts) | N |
| `/api/tournaments/{tournament_id}/result-share` | Bound | [result share](#result-share) | A |
| `/api/rounds/{round_id}/awards` | Bound | [awards](#awards) | A |
| `/api/rounds/{round_id}/score-access` | Incomplete / P2 | [numeric](#numeric) | S |
| `/api/rounds/{round_id}/scorecards/{owner_type}/{owner_id}` | Incomplete / P2 | [numeric](#numeric) | S |
| `/api/rounds/{round_id}/scorecards/{owner_type}/{owner_id}/scoring` | Incomplete / P2 | [numeric](#numeric) | A* |
| `/api/rounds/{round_id}/stableford/scorecards/{player_id}` | Incomplete / P2 | [stableford](#stableford) | A |
| `/api/rounds/{round_id}/stableford/scorecards/{player_id}/scoring` | Incomplete / P2 | [stableford](#stableford) | A |
| `/api/tournaments/{tournament_id}/final-round-visibility` | Incomplete / P2 | [visibility](#visibility) | S |

## Handler and repository traces

### Numeric

[Handlers](../../../backend/src/api/scorecards/mod.rs#L106) pass the session UUID to
`score_authorization::writable_owners`, `scorecards::get_read_authenticated` and
`scorecards::get_scoring_authenticated`.

- [Score-access](../../../backend/src/repositories/score_authorization.rs#L29): RR;
  active session/user share locks, stored round-parent lookup, exact membership
  share lock, deterministic owner resolution and four-ball partner-to-team mapping.
  Both the singles empty-result commit at line 46 and ordinary commit at line 66
  omit a final session check. No full-read retry.
- [Member card](../../../backend/src/repositories/scorecards.rs#L113): RR; stored
  round context, active session/user and exact membership locks, stored final
  visibility context, preserved owner/handicap/hole/confirmation assembly and
  redaction, then commit at line 145 with no final check. No full-read retry.
- [Scoring card](../../../backend/src/repositories/scorecards.rs#L149): RR; validates
  round state, [exact role/eligible owner](../../../backend/src/repositories/score_authorization.rs#L95),
  and full scoring-card assembly before commit at line 203, again no final check.
  This read alone already retries the complete attempt once for `40001`, then
  returns generic 503; it must preserve that behavior. The first two reads map
  database conflicts to generic 500 and fail closed.

Score-access reveals authorized owner IDs/eligibility, not score values. Member
cards preserve final visibility; scoring cards require the existing stricter
write-eligible owner access. R1 concerns success after natural expiry; it does
not show another user/role can bypass those rules. Success headers and the
scoring-only middleware are at [API lines 33–48](../../../backend/src/api/scorecards/mod.rs#L33)
and [118–159](../../../backend/src/api/scorecards/mod.rs#L118).

### Match details

[Handlers](../../../backend/src/api/match_play.rs#L67) pass session IDs to
`match_play::reads::get(false/true)`, `completion::get` and `reads::table`.
All four use RR and [stored round/match context and exact membership](../../../backend/src/repositories/match_play/mod.rs#L64).
Session/user and membership share locks remain through assembly; scoring also
checks both opponents through the score-authorization resolver.

[Card loading](../../../backend/src/repositories/match_play/reads.rs#L66)
commits at line 92, [completion](../../../backend/src/repositories/match_play/completion.rs#L18)
at line 45 and [table loading](../../../backend/src/repositories/match_play/reads.rs#L242)
at line 259 without the final `live` session check. R1 applies to note/card,
completion-match and table-award waits. There is no full-read `40001` retry;
conflicts fail closed as generic database errors. [Router middleware](../../../backend/src/api/match_play.rs#L43)
provides A cache coverage. This does not include the bound listing path below.

### Four ball

[Member/scoring handlers](../../../backend/src/api/four_ball.rs#L101) call
`four_ball::get(false/true)`. The [repository](../../../backend/src/repositories/four_ball/mod.rs#L20)
uses RR, stored round-parent context, active session/user locks and exact
membership; it proves the team belongs to the round and has two preserved
partners. Scoring authorizes both partners before loading holes, inputs and
confirmation. The [late card queries](../../../backend/src/repositories/four_ball/mod.rs#L160)
precede the unchecked [commit at line 59](../../../backend/src/repositories/four_ball/mod.rs#L59). R1 applies to late input/confirmation waits. No whole-read retry;
[all route responses](../../../backend/src/api/four_ball.rs#L59) have A coverage.

### Stableford

[Handlers](../../../backend/src/api/stableford.rs#L102) call
`stableford::get(false/true)`. Its [repository](../../../backend/src/repositories/stableford/mod.rs#L21)
uses RR, stored round-parent context, active session/user and exact membership
locks. The player must have a preserved round snapshot; scoring adds exact owner
authorization. [Holes, inputs and confirmation](../../../backend/src/repositories/stableford/mod.rs#L138)
load afterward, then [line 60](../../../backend/src/repositories/stableford/mod.rs#L60)
commits without a final check. R1 applies to those later waits. No whole-read
retry; [router middleware](../../../backend/src/api/stableford.rs#L60) provides A.

### Invitations

[Admin list](../../../backend/src/api/invitations/admin.rs#L47) passes the session
UUID and parsed tournament ID to [invitations::list](../../../backend/src/repositories/invitations/admin.rs#L16).
D isolation; [lock_admin](../../../backend/src/repositories/invitations/mod.rs#L210)
locks session/user and exact tournament-admin membership `FOR UPDATE`. A single
metadata query includes redemption counts, then commits at line 33 without a
final active-session check. R1 applies to expiry during membership or metadata
waits; later revocation/credential writers are serialized, not free to commit
behind those authority locks. No full-read retry. [Router middleware](../../../backend/src/api/invitations/mod.rs#L50)
provides N coverage. Listing returns metadata, not invitation token secrets.

### Visibility

[The GET handler](../../../backend/src/api/tournament_visibility.rs#L38) passes
`principal.user_id`, discarding session identity. [get_for_admin](../../../backend/src/repositories/tournament_visibility.rs#L36)
uses RR, loads stored tournament visibility metadata, locks exact-admin
membership through [the user-only helper](../../../backend/src/repositories/tournament_authorization.rs#L65),
and commits without session/user authority or a final check. No full-read retry.
R2 is the remaining user-only metadata boundary; no changed visibility mutation
or hidden-score disclosure was demonstrated. Cache policy is S.

### Courses

- [Catalog](../../../backend/src/api/course_catalog.rs#L53) calls the
  [user-only admin helper](../../../backend/src/repositories/tournament_authorization.rs#L35)
  (D), which commits before searching [bundled JSON](../../../backend/src/course_catalog.rs#L9).
  Neither session identity nor membership lock covers subsequent local assembly.
- [Presets](../../../backend/src/api/course_catalog.rs#L30) passes user identity to
  [list_for_admin](../../../backend/src/repositories/course_presets.rs#L52). RR and
  exact membership/admin share locks cover its query/assembly, but no session
  lock/check is present. SQL selects globally registered manual presets, not
  arbitrary private tournament revisions.
- [Provider detail](../../../backend/src/api/course_provider.rs#L29) commits that
  same user-only admin helper before [readiness/cache/provider work](../../../backend/src/api/course_provider.rs#L39).
  A future repair must avoid holding database locks across external I/O; authorize
  returned facts separately. The current [eight catalog entries](../../../backend/data/course_catalog.json#L1)
  are missing/incomplete, so none reaches the successful provider-fetch branch.

All three lack a final session check and full-read retry. R3 is lower-impact
because of their course-data scope; successful provider work/quota impact is
conditional, not demonstrated reachable behavior with this catalog. H cache
coverage comes from handler success and [catalog](../../../backend/src/api/course_catalog.rs#L89)/
[provider](../../../backend/src/api/course_provider.rs#L99) custom errors. Extractor
rejections bypass those wrappers. No external provider was contacted.

### Awards

[Handler](../../../backend/src/api/round_awards.rs#L34) passes session identity to
[round_awards::read](../../../backend/src/repositories/round_awards/mod.rs#L50).
D isolation; [require_round_admin](../../../backend/src/repositories/tournament_authorization.rs#L151)
locks the stored round and derives its tournament, then retains session/user and
exact-admin membership share locks. The round lock serializes award slots/history
with the award mutation protocol; eligibility names are separately read under D,
not claimed as one RR snapshot. After all queries, `recheck` calls active-session
wall-clock validation at line 71, then commits. Bound, no full-read retry, A cache.

### Accounts

[Admin list](../../../backend/src/api/player_claims/admin.rs#L29) passes session
identity to [player_claims::list](../../../backend/src/repositories/player_claims/admin.rs#L23).
D isolation; exact target-admin authorization retains session/user and membership
share locks. One SQL statement loads account and latest-claim metadata, then
[recheck_session](../../../backend/src/repositories/player_claims/mod.rs#L69) checks
active session and fresh database wall clock before commit. Bound, no full-read
retry, N [route cache coverage](../../../backend/src/api/player_claims/mod.rs#L40).

### Result share

[Status handler](../../../backend/src/api/result_sharing/management.rs#L46) passes
session identity to [result_sharing::status](../../../backend/src/repositories/result_sharing/management.rs#L52).
D isolation; exact-admin authorization retains session/user and membership locks,
then one query reads latest grant metadata, then authorization is repeated before
commit. The final session check follows payload loading. Bound, no full-read
retry, A [route cache coverage](../../../backend/src/api/result_sharing/mod.rs#L30).
[GrantMetadata](../../../backend/src/repositories/result_sharing/mod.rs#L11) contains
no capability token/hash. Public capability validation is outside this inventory.

### Fantasy

The [seven routes](../../../backend/src/api/fantasy/mod.rs#L15) reach
[game/selection/source handlers](../../../backend/src/api/fantasy/handlers.rs#L34)
and [four result handlers](../../../backend/src/api/fantasy/results.rs#L16), which
pass session UUIDs to `fantasy::game`, `read_round`, `source`, `results::overall`,
`round_results`, `golfer_breakdown` and `manager_breakdown`.
[Shared begin/authorize/recheck](../../../backend/src/repositories/fantasy/mod.rs#L49)
checks active session/user, exact tournament membership and optional exact admin,
locks tournament rounds in UUID order and reauthorizes. Game/selection/source
use D with those locks; final checks occur in [config](../../../backend/src/repositories/fantasy/config.rs#L81),
[lineups](../../../backend/src/repositories/fantasy/lineups.rs#L106) and
[sources](../../../backend/src/repositories/fantasy/sources.rs#L29). Stored
round/owner relationships and tournament+round targets are validated.

[Results](../../../backend/src/repositories/fantasy/results/mod.rs#L38) use RR,
round locks, scoped target validation, full fact loading/projection and final
recheck before commit. All seven are bound with N [cache coverage](../../../backend/src/api/fantasy/mod.rs#L52).
No repository automatically retries. Direct SQL `40001` becomes `409
fantasy_conflict`; membership errors nested inside `AuthorizationError::Database`
instead map generic 500. Both fail closed; no uniform retry parity is claimed.

Selection and results GETs can finalize due windows and trigger existing
post-commit invalidations. Their authority is bound, but they are not
side-effect-free reads. This inventory executed none of them. Any future retry design must account for their transaction/finalization and
notification behavior; the selected read-only wrapper is not a drop-in boundary. No Fantasy runtime behavior changes here.

### Match listing

[The listing handler](../../../backend/src/api/match_play.rs#L91) calls
`reads::list` or `list_for_player` for its optional typed `player_id` filter.
[The listing boundary](../../../backend/src/repositories/match_play/reads.rs#L164)
uses RR, stored round/tournament relationships, session/user and exact membership
share locks; it selects matching IDs by stored round and optional player filter,
then retains one role/context through every card. An unmatched player filter
returns no matching IDs; there is no separate player-existence validation. `live` checks active session after complete assembly at
line 221 before commit. Bound for both filtered/unfiltered forms; no full-read
retry, A cache. The sibling card/completion/table reads above do not inherit this
final check.

## Next bounded candidate

A1g should reproduce natural expiry during authority/fact waits on the three
numeric GETs, then reject success after expiry while retaining complete authority
and payload ownership. Preserve fixed handicap snapshots, hidden-final redaction,
format/owner checks, exact role/flight permissions, error precedence, ordering and
existing scoring-card retry behavior. Empty/viewer/singles score-access results
must also be checked. Specify any bounded full-read retry for the other two
GETs and private error/cache behavior explicitly. Do not alter saves,
confirmations, conditional delivery, format-specific siblings or shared mutation
authority merely to close these selected reads.

Validation for that future implementation needs PostgreSQL races, valid concurrent
reads/cancellation, JSON parity/no writes/SSE, existing mutation/lifecycle
compatibility, backend/PostgreSQL ladders, actual mobile/desktop browser evidence,
and independent review. It is a proposal here; no repair was started.

## Validation and limits

- A static scan and independent review account for all 46 explicit GET routes,
  with no manifest omissions, extras or duplicates. Every recorded route source
  location identifies its declaration; all 27 remaining routes appear in the matrix.
- Independent read-only source reviews cover all 27 routes (16 format/Fantasy/
  match routes and 11 other routes). Two P3 documentation findings were corrected:
  match-list filtering was distinguished from player-existence validation, and
  final unchecked commits were distinguished from late fact-loading lines.
  Both corrections were re-reviewed; no remaining inventory findings were reported.
- Local document links and nonblank source-line citation targets resolve; the
  route source matches the reviewed revision. The eight bundled catalog entries
  were parsed locally and none is usable. No upstream verification was attempted.
- `git diff --check` passes; the scoped diff contains only documentation and
  JSON inventory evidence. The primary's validation log is
  `/tmp/golf-a1f-validation.log`; reviewers independently recomputed route counts.
- Backend, PostgreSQL, frontend and browser ladders were not rerun because this
  step changes no application/test/schema/dependency or user-facing runtime
  behavior. The approved plan calls for source traces, independent review,
  document-link and diff checks. No database or provider was accessed and no
  request, build, scanner or migration was run.

This source inventory is neither a fresh dependency audit, broad scoring/security
sign-off nor a performance benchmark. All remaining runtime races, hosted checks,
legacy-write expiry and other severity-rated review findings remain separate
work. The earlier A1e test/browser results are historical evidence linked above,
not checks executed for this documentation-only step.
