# Active plans

This file is the source of truth for implementation work. Keep only one active
step. Move durable completed behavior to `Documentation.md` or `ARCHITECTURE.md`
instead of accumulating a historical log here.

## Planning status

Phase 3 reusable invitations is complete. The product requirements below are
accepted into the roadmap, but no implementation step is currently approved.
The recommended next step is Phase 4A.

## Active step

No active implementation step. A direct approval is required before Phase 4A or
another bounded step begins.

## Product decisions

- Accounts use a required, case-insensitive-unique username for login. Email is
  never a credential or an identity link; whether it remains required contact/
  recovery data is decided before Phase 4A implementation.
- The handicap registered when a player joins a tournament is the immutable
  tournament handicap. A later global player-handicap change applies only to a
  future tournament. Round snapshots derive course and playing handicaps from
  that fixed tournament value and the round's selected tee.
- Norwegian UI accepts both `14,4` and `14.4`, rejects more than one decimal
  place, and displays `14,4`. JSON remains numeric and PostgreSQL retains exact
  `NUMERIC(4,1)` values.
- Every round selects an exact course and tee. Provider data is imported into
  local course/tee/hole revisions so slope, course rating, par, yardage, and hole
  stroke index remain reproducible after the provider changes.
- Flights are explicit, round-specific groups independent of teams, tee times,
  and starting holes. An admin assigns players, teams, and one designated
  scorekeeper for each flight. That scorekeeper may write every eligible
  scorecard in the flight; tournament admins/scorers retain their override.
- A tournament stores one `counted_rounds` value from 1 through its configured
  round count. Each leaderboard selects a player's best N completed results for
  its own metric, so gross and net may count different rounds.
- Tournament players and round teams remain separate. Team membership can change
  every round, and team results continue to be attributed to the exact members
  of that round for individual tournament standings.
- Leaderboard rows link to read-only scorecards. A player drilldown shows every
  round contribution and its player/team owner; a team drilldown shows that
  round's shared card. Gross and net views use the same preserved strokes.

## Decision gates

- Approve username syntax and legacy backfill. Recommended: 3-32 ASCII letters,
  digits, `_`, and `-`, compared case-insensitively, with collision-safe suffixes
  for migrated users. Keep display names separate and Unicode-friendly.
- Decide whether email is optional or required for recovery/contact. Login and
  authorization must use neither email nor display name after the username cutover.
- Confirm whether the scramble maximum of 36 applies to the registered handicap
  index before tee conversion or to each rounded course handicap before the team
  formula. Recommended interpretation: cap the registered tournament handicap
  index at `36.0` for scramble calculations only, then calculate the course
  handicap for the selected tee. Individual formats keep the uncapped value.
- The first course milestone assumes one tee for the whole round. Per-player or
  per-team tees require a separate assignment and snapshot model and must be
  approved before Phase 4B if needed.
- The designated flight scorekeeper must have an active account linked to a
  player in that flight. Decide later whether a backup scorekeeper is useful;
  do not grant every flight member write access implicitly.
- Best-N provisional standings rank players with more counted results first until
  they reach N. Final eligibility requires N completed attributed results unless
  an explicit withdrawal policy is added.
- Approve one new scoring format and its team size, handicap allowance, score
  owner, completion rule, and tie behavior before implementation. Start with
  two-player foursomes because it can reuse a team-owned hole score. Four-ball,
  Stableford, and match play require distinct aggregation or scoring contracts.

## Upcoming work

### Phase 4A: Username accounts and fixed tournament handicaps

- **Goal:** align account creation/login and handicap preservation before adding
  more tournament management surfaces.
- **Files/modules:** a forward migration; backend auth, onboarding, invitation,
  tournament-player, round-opening, handicap/scoring services, seed and tests;
  frontend auth/onboarding/join APIs, forms, handicap utilities and tests.
- **Change:** add normalized usernames and migrate existing users without
  collisions; replace email login in every flow; apply the approved email contact
  rule; remove the ordinary tournament-handicap mutation path; retain an explicit
  audited admin correction only before any round has opened; apply the approved
  scramble-36 rule in the handicap domain service; centralize comma parsing and
  Norwegian display formatting.
- **Validation:** clean/upgrade migration tests, normalized-username concurrency,
  auth timing/error behavior, onboarding/invitation browser flows, immutable
  tournament handicap tests, scramble cap boundaries, round snapshot history,
  Rust/PostgreSQL/frontend ladders, and mobile/desktop browser checks.
- **Invariants:** authorization continues to use user/player UUIDs; email never
  links identities; a global handicap change cannot alter an existing tournament;
  completed history remains unchanged; formulas stay outside handlers.
- **Stop condition:** all account flows use username, all tournament handicap
  paths preserve one registered value, locale behavior is consistent, the cap is
  proven at its boundary, and existing databases migrate safely.

### Phase 4B: Private workspace and provider-backed courses

- Move tournament, entrant, round, team, and leaderboard reads behind active
  membership. Add `/manage/tournaments/:tournamentId` with admin-only sections
  for settings, entrants, invitations, rounds, courses, pairings, and lifecycle.
- Add a backend-only GolfCourseAPI client and normalized search/detail endpoints;
  keep its API key out of the browser and logs. Follow the official OpenAPI at
  `https://api.golfcourseapi.com/docs/api/`; respect provider rate limits with
  bounded search, caching, timeouts, and deliberate unavailable/exhausted states.
- Store provider name, opaque course ID, tee category/name, import timestamp, and
  immutable local course/tee/hole revisions. Do not invent an upstream tee ID.
  Validate exactly ordered holes, unique stroke indexes, par, rating, slope, and
  selected tee before a draft round can open.
- Add an atomic draft-only round configuration endpoint and a mobile course then
  tee picker showing rating, slope, length, hole completeness, loading, empty,
  error, retry, stale-result, and conflict states.
- **Stop condition:** an authorized admin can configure every draft round with a
  locally preserved provider course/tee snapshot; opened rounds never drift when
  provider data changes or becomes unavailable.

### Phase 5: Teams, flights, and pairing validation

- Add normalized round flights and flight-player membership with database
  uniqueness, one designated scorekeeper, and draft-only mutation guards.
- Reserve teams for formats with a shared team result. Convert any grouping-only
  individual-round teams to flights rather than preserving two pairing models.
- Build one mobile roster editor for unassigned players, round teams, flights,
  team membership, flight placement, starting hole, and tee time. Use accessible
  move/add/remove controls rather than drag-only behavior.
- Validate missing/duplicate players, unexpected team sizes, split teams across
  flights, missing scorekeepers, and incomplete assignments. Freeze teams,
  flights, and scorekeeper authority when the round opens.
- Extend both score-access listing and transactional mutation authorization so
  the designated scorekeeper receives every score owner in their flight.
- **Stop condition:** admins can configure and validate round-specific teams and
  flights, and tests prove one flight scorekeeper can score both teams without
  granting cross-flight or cross-tournament access.

### Phase 6: Foursomes and format-aware live scoring

- Split format-sensitive lifecycle, scorecard, and leaderboard modules before
  expanding the scoring-format enum. Replace binary format assumptions with
  exhaustive, typed policies without building a generic scoring framework.
- Implement two-player foursomes as the next team-owned format after its handicap
  formula is approved. Update onboarding, round configuration, readiness,
  snapshots, score access, completion, leaderboards, seed data, and UI together.
- Optimize the phone score selector for a flight scorekeeper moving quickly among
  all cards in the flight while preserving immediate save, sync, correction,
  audit, confirmation, SSE invalidation, and locked-round behavior.
- **Stop condition:** foursomes works end to end and every supported format has
  explicit owner, team-size, handicap, completion, and ranking tests.

### Phase 7: Best-N standings and scorecard drilldown

- Add `counted_rounds` with `1 <= counted_rounds <= number_of_rounds`, editable
  only while tournament configuration is mutable.
- Select each player's lowest N completed score-to-par results independently for
  gross and net. Return every round contribution with round ID, tagged player or
  team owner, metric totals, and counted/excluded state; keep deterministic ties
  and changing-team attribution.
- Add URL-backed, membership-scoped read-only player history and scorecard routes.
  Make round and tournament leaderboard rows navigable, label “Best N of M,” and
  visibly distinguish counted from discarded rounds. Read DTOs must omit score
  mutation actor identifiers that viewers do not need.
- **Stop condition:** mixed individual/team rounds select the correct best N per
  metric, incomplete competitors cannot gain an advantage, and every displayed
  contribution deep-links to the preserved gross/net card.

### Later product work

- Add four-ball/best ball only after deciding whether every individual ball is
  stored; then consider Stableford and match play as separate milestones.
- Add partner-repeat-aware team generation, handicap balancing, flight progress
  and missing-score alerts, optional backup scorekeepers, configurable tie-breaks,
  share links, offline scoring, account recovery, rate limiting, deployment,
  backups, and production database roles.
- Consider per-player tees only if a real round requires them; do not complicate
  the round-wide tee snapshot in advance.
