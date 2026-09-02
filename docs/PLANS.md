# Active plans

This file is the source of truth for implementation work. Keep only one active
step. Move durable completed behavior to `Documentation.md` or `ARCHITECTURE.md`
instead of accumulating a historical log here.

## Planning status

Phase 4A, the private-workspace reads, username constraint repairs, initial
read-only tournament-management workspace, backend course-provider boundary,
curated eight-course local catalog, and the Phase 4B immutable local course-
revision persistence boundary, and the admin-only atomic draft-round course-
configuration API, mobile course/tee picker, and manual-entry fallback are
complete, as is the corrected normalized PostgreSQL flight persistence boundary
with membership-wide score authority, and the transactional pairing-roster
API plus explicit legacy individual-group conversion and flight-aware pairing
validation/opening and deterministic representative seed assignments for all
five rounds, the mobile draft-round pairing roster editor, and runtime exact-
flight score authority and the exhaustive round-leaderboard format boundary are
complete, as is the exhaustive round lifecycle, pairing, completion, authorization,
and scorecard format-policy boundary, and the complete two-player foursomes
format with preserved WHS team handicaps, and the phone-first writable-card
selector optimization, and the persisted draft-only `counted_rounds`
configuration boundary, metric-specific completed best-N tournament
contributions, the strict-Clippy lifecycle error-size repair, the optional
mandatory-round configuration and reserved best-N selection, and the Phase 7A
trusted final-score embargo clock are complete.
The exact-admin tournament-start lifecycle and hosted management action are also
complete, including draft-only creation and concurrent lifecycle coverage.

The legacy global player/profile/handicap HTTP and frontend directory plus the
platform-admin tournament-creation route are retired. Scorecard reads and
tournament-targeted live invalidation are membership-private and identity-owned.
Direct arbitrary-player roster registration is also retired; creator onboarding
and invitation redemption are the only HTTP participation entry points and keep
membership, entrant, initial handicap audit, and redemption facts aligned.
Direct round creation now revalidates the active session and exact tournament-
admin membership before inspecting content type or polling its bounded request
body, then reauthorizes in the insertion transaction. Exact tournament
membership is also mandatory for score-access reads, and the backend
participation boundary is covered by a two-tournament player/team isolation
fixture. Frontend target-coherent runtime decoding and tournament-keyed transient
state now prevent cross-target cache or UI reuse during same-account navigation.
Phase 7B1 embargo-safe read projections and Phase 7B2a provisional open-round
best-N standings are complete, including browser-dispatchable identifier-free
live invalidation and authoritative 3-to-4-hole refresh validation. Phase 7B2b
membership-private player history and result-card drilldowns are also complete.
There is no active implementation step; partner-repeat-aware team generation and
handicap balancing are the next bounded candidates.

## Product decisions

- Accounts use only a required, case-insensitive-unique username and password for
  registration and login. Account requests and UI contain no email. Existing
  account email data may be used once for collision-safe username migration, but
  is not retained as a credential, recovery requirement, or identity link.
- Administration is tournament-scoped. Creating a tournament grants that creator
  the `admin` membership only for that tournament. It grants no authority to
  view or mutate another tournament, and a global account role is never an
  administration bypass. Any future additional-admin or ownership-transfer flow
  requires a separate explicit product decision.
- A player participates only through an explicit registration in each tournament.
  The account/player identity may persist for login and later registrations, but
  it must not appear in another tournament's roster, pairings, score access,
  handicaps, or results until a separate `tournament_players` registration and
  tournament membership exist there. There is no member-visible global player
  directory.
- The handicap registered when a player joins a tournament becomes the fixed
  tournament handicap. A tournament admin may make an audited correction only
  while every round is still draft and no round snapshot has ever been captured.
  After the first round opens it is immutable. A later global player-handicap
  change applies only to a future tournament.
- In scramble rounds, cap each registered tournament handicap index at `36.0`
  before converting it for the selected tee. Individual formats retain the full
  registered index. Round snapshots preserve the exact input and derived course/
  playing handicaps used by that format.
- Norwegian UI accepts both `14,4` and `14.4`, rejects more than one decimal
  place, and displays `14,4`. JSON remains numeric and PostgreSQL retains exact
  `NUMERIC(4,1)` values.
- Everyone in a round uses the same selected course and tee. Provider data is
  imported into local course/tee/hole revisions so slope, course rating, par,
  yardage, and hole stroke index remain reproducible after the provider changes.
- If no usable provider course exists, a tournament admin manually defines the
  course and selected tee. Tee name/category, course rating, slope, ordered hole
  pars, and a unique `1..=hole_count` stroke-index permutation are required;
  each hole's yard distance is optional. Manual and provider imports create the same
  immutable local revision before the round can open.
- Flights are explicit, round-specific groups independent of teams, tee times,
  and starting holes. Every authenticated player linked to an exact flight member
  may write every eligible scorecard in that flight; tournament admins/scorers
  retain their override.
- A tournament stores one `counted_rounds` value from 1 through its configured
  round count. Each leaderboard selects a player's best N completed results for
  its own metric, so gross and net may count different rounds.
- A tournament admin may optionally designate one tournament round as mandatory
  before the tournament starts. That round consumes one of the configured N
  counted slots for both gross and net; each metric independently chooses its
  remaining best `N - 1` contributions. A player who does not complete the
  mandatory round is not finally eligible merely by completing other rounds.
- Starting a tournament is an explicit exact-admin action in the management
  workspace. It moves the tournament from `draft` to `active` after authoritative
  readiness checks and freezes pre-start tournament configuration. Starting the
  tournament does not itself open or score an individual round; round opening
  remains a separate admin action.
- Tournament players and round teams remain separate. Team membership can change
  every round, and team results continue to be attributed to the exact members
  of that round for individual tournament standings.
- Leaderboard rows link to read-only scorecards. A player drilldown shows every
  round contribution and its player/team owner; a team drilldown shows that
  round's shared card. Gross and net views use the same preserved strokes.
- Players can view live gross/net standings for the current round and live
  tournament-wide standings that include the active round. While the final round
  is open, holes 10-18 are excluded from every non-admin leaderboard projection;
  only that tournament's admins see full live standings. When every required
  final-round scorecard is complete and confirmed, a 24-hour embargo starts from
  trusted database time. Completion or locking does not reveal the hidden final
  scores early; they become visible automatically when the embargo expires.
- Two-player foursomes uses one alternate-shot team card. Its Playing Handicap is
  50% of the partners' combined unrounded Course Handicaps, applied once and
  rounded only at the end under WHS allowance rules; the final team value is
  preserved at round opening. Completion and ties follow the existing team-card
  and competition-position contracts.
- After foursomes, finish the remaining roadmap, phone/performance optimization,
  and a dedicated security review before adding any further play modes. Four-ball,
  Stableford, and match play are final-stage product work, not intermediate
  extensions.

## Decision gates

- Best-N provisional standings rank players with more counted results first until
  they reach N. Final eligibility requires N completed attributed results unless
  an explicit withdrawal policy is added.
- Every later scoring format still requires explicit approval of team size,
  handicap allowance, score owner, completion rule, and tie behavior. No later
  format begins before roadmap completion, optimization, and security review.

## Upcoming work

### Later product work

- Add partner-repeat-aware team generation, handicap balancing, flight progress
  and missing-score alerts, configurable tie-breaks,
  share links, offline scoring, account recovery, rate limiting, deployment,
  backups, and production database roles.
- Decide whether PostgreSQL should enforce at most one open round per tournament;
  current reads deterministically select the highest-numbered open round.
- After the remaining roadmap, performance optimization, and security review are
  complete, add four-ball/best ball only after deciding whether every individual
  ball is stored; then consider Stableford and match play as separate final-stage
  milestones.
