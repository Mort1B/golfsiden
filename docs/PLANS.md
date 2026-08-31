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
complete, as is the exhaustive lifecycle, pairing, completion, authorization,
and scorecard format-policy boundary, and the complete two-player foursomes
format with preserved WHS team handicaps, and the phone-first writable-card
selector optimization, and the persisted draft-only `counted_rounds`
configuration boundary, and metric-specific completed best-N tournament
contributions are complete. No active implementation step is approved.

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

### Validation repair

- Restore strict all-feature Clippy by reducing or boxing the three oversized
  `round_lifecycle` error variants reported at `opening.rs:22` and `mod.rs:60,71`.
  Preserve public error mapping and lifecycle behavior, add no feature semantics,
  and require the complete Rust/database ladder before returning to Phase 7.

### Phase 7: Live best-N standings, final-nine blackout, and scorecards

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
  flight member's writable cards so those scores can be entered and corrected.
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

- Add partner-repeat-aware team generation, handicap balancing, flight progress
  and missing-score alerts, configurable tie-breaks,
  share links, offline scoring, account recovery, rate limiting, deployment,
  backups, and production database roles.
- After the remaining roadmap, performance optimization, and security review are
  complete, add four-ball/best ball only after deciding whether every individual
  ball is stored; then consider Stableford and match play as separate final-stage
  milestones.
