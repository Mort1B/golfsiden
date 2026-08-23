# Active plans

This file is the source of truth for implementation work. Keep only one active
step. Move durable completed behavior to `Documentation.md` or `ARCHITECTURE.md`
instead of accumulating a historical log here.

## Planning status

Phase 4A, the first Phase 4B private-workspace step, and the sign-in username
constraint repairs and the initial read-only tournament-management workspace are
complete. No active implementation step is approved. The backend-only provider
client and normalized course search/detail contract are the next Phase 4B step.

## Product decisions

- Accounts use only a required, case-insensitive-unique username and password for
  registration and login. Account requests and UI contain no email. Existing
  account email data may be used once for collision-safe username migration, but
  is not retained as a credential, recovery requirement, or identity link.
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
- Players can view live gross/net standings for the current round and live
  tournament-wide standings that include the active round. While the final round
  is open, holes 10-18 are excluded from every non-admin leaderboard projection;
  only that tournament's admins see full live standings. When every required
  final-round scorecard is complete and confirmed, a 24-hour embargo starts from
  trusted database time. Completion or locking does not reveal the hidden final
  scores early; they become visible automatically when the embargo expires.

## Decision gates

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

### Phase 4B: Private workspace and provider-backed courses

- Add `/manage/tournaments/:tournamentId` with admin-only sections for settings,
  entrants, invitations, rounds, courses, pairings, and lifecycle.
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

### Phase 7: Live best-N standings, final-nine blackout, and scorecards

- Add `counted_rounds` with `1 <= counted_rounds <= number_of_rounds`, editable
  only while tournament configuration is mutable.
- Select each player's lowest N completed score-to-par results independently for
  gross and net. Return every round contribution with round ID, tagged player or
  team owner, metric totals, and counted/excluded state; keep deterministic ties
  and changing-team attribution.
- Include the current open round as an explicitly provisional contribution in
  both round and tournament standings, with holes played exposed so uneven live
  progress is not presented as final. Re-evaluate the provisional best N whenever
  an authoritative score change is received.
- Apply a role-aware visibility policy in the backend domain/repository boundary.
  For non-admins, remove holes 10-18 of the final round before gross, net,
  position, tie, and tournament aggregate calculations while that round is open
  and throughout its 24-hour final-score embargo. Tournament admins receive the
  complete projection. SSE remains payload-free invalidation and cannot leak
  hidden strokes.
- Persist `final_scores_hidden_until` from database time when all required final-
  round scorecards first become complete and confirmed. A correction that removes
  confirmation before expiry clears the deadline; after every corrected card is
  complete and reconfirmed, start a new 24-hour embargo. Round completion and
  locking must preserve, not shorten, the current deadline.
- Mark role-dependent leaderboard and scorecard responses private/non-cacheable;
  include session identity in client query ownership and clear privileged cached
  projections when the session changes.
- Add URL-backed, membership-scoped read-only player history and scorecard routes.
  Make round and tournament leaderboard rows navigable, label “Best N of M,” and
  visibly distinguish counted from discarded rounds. Read DTOs must omit score
  mutation actor identifiers that viewers do not need. Non-admin read-only cards
  obey the same final-nine redaction; authorized scoring views still show the
  scorekeeper's writable flight cards so those scores can be entered and corrected.
- Test tournament-admin versus scorer/player/viewer projections, both metrics,
  round and tournament APIs, direct scorecard reads, cache/session changes,
  completion and locking without reveal, correction/reconfirmation clock reset,
  exact before/at/after deadline behavior with controlled time, and attempts to
  infer hidden totals from response fields. Return the visibility deadline so the
  client can show availability and schedule an authoritative refetch at expiry
  without a background reveal job.
- **Stop condition:** mixed individual/team rounds select the correct best N per
  metric, live totals include the active round, incomplete competitors cannot gain
  an advantage, final-round back-nine strokes are absent from every non-admin read
  projection until 24 hours after the latest complete-and-confirmed final result,
  and each visible contribution deep-links to its preserved gross/net card.

### Later product work

- Add four-ball/best ball only after deciding whether every individual ball is
  stored; then consider Stableford and match play as separate milestones.
- Add partner-repeat-aware team generation, handicap balancing, flight progress
  and missing-score alerts, optional backup scorekeepers, configurable tie-breaks,
  share links, offline scoring, account recovery, rate limiting, deployment,
  backups, and production database roles.
