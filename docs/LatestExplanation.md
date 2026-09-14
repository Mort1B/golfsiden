# Application review and prioritized repair plan

The 2026-09-14 review of commit `b8af787` found **one high, two medium and three
low findings** across the completed scoring formats and shared application paths.
The user requested review and planning; no production source, tests or migrations
were changed. [PLANS.md](PLANS.md) contains six separate bounded repair candidates,
starting with preservation of unsaved scoring input.

## High findings

### H1 — unsaved input can disappear after lock or access denial

A failed IndexedDB write leaves the intended score only in component-local state.
Switching to a read projection or replacing the scorer with an authorization error
unmounts that state and releases its navigation guard. This affects the shared
legacy scorer (individual stroke, scramble, foursomes), four-ball and Stableford.
Match play has a separate local-only recovery path.

The review reproduced the legacy case in real Chrome at 390x844: prepare a
confirmed completed card with server score 4, force pending-store `put` to throw,
correct to 5, and verify the failed-device-save warning, disabled logout and empty
queue. Lock the round externally. The warning/recovery disappears, logout becomes
enabled and the durable queue remains empty; the only copy of 5 is lost without
discard. Backend score integrity remains intact, but local user intent is lost.

Evidence: `frontend/src/features/scoring/useScoreWorkspaceData.ts:74–105`,
`frontend/src/pages/ScorePage.tsx:43–75`,
`frontend/src/features/scoring/useHoleScoreSync.ts:28–54`,
`frontend/src/features/scoring/stableford/StablefordExperience.tsx:26–35`, and
`frontend/src/features/scoring/fourBall/FourBallExperience.tsx:23–28`.

Repair must preserve only account/owner/hole-scoped local intent when authority
changes; retaining unauthorized canonical score data is not an acceptable fix.
Regression coverage must include numeric and pickup states, both four-ball partners,
lock, denied scoring, metadata failures, discard and identity changes.

## Medium findings

### M1 — compatible score writes and confirmations can outlive their session

Authenticated legacy save and confirmation check the session before waiting for
the membership lock, then commit without a fresh expiry check. Newer conditional
and dedicated-format commands have explicit final session checks.

Both legacy endpoints were reproduced against the isolated API and PostgreSQL.
Each fixture's session was changed to expire after 1.5 seconds. A separate
transaction held its membership row for four seconds while the HTTP request waited.
Both requests returned 200 after more than two seconds, and database reads confirmed
the new score or confirmation had persisted. These were deliberate disposable
fixture mutations, not production data changes.

Evidence: `backend/src/repositories/scorecards/mutations.rs:66–88` and `:168–178`,
`backend/src/repositories/score_authorization.rs:81–87`. The existing conditional
regression at `backend/tests/scorecards/conditional.rs:482–521` provides the repair
test pattern. Recheck current session validity before committing both authenticated
legacy operations, including no-op responses; expiry must roll back effects and
prevent SSE notification.

### M2 — private projections remain visible after denied refresh

Direct scorecards, player history and round result pages treat authorization errors
with prior data like ordinary background failures. The installed TanStack Query
retains prior data when a refetch fails. SSE checks membership on relevant events;
a healthy idle stream does not guarantee immediate cache clearing.

Chrome reproduced a direct private card displaying gross 7 after its card request
returned a controlled 403 during page-return refresh. The healthy stream stayed
connected and the page displayed “Viste data beholdes” alongside the prior score.
This demonstrates the UI response to a known denial; it is not evidence of a
server endpoint returning new private data to an unauthorized caller.

Evidence: `frontend/src/pages/DirectScorecardPage.tsx:67–88`,
`frontend/src/pages/PlayerHistoryPage.tsx:52–62`, and
`frontend/src/pages/LeaderboardPage.tsx:161–167`.

Classify 401/403/404 as authoritative denial, hide/remove the affected private
projection and require successful authorization before redisplay. Keep permitted
transient-error behavior distinct and preserve H1's local-only recovery.

## Low findings

### L1 — generic allocator has a signed-minimum defect

`backend/src/domain/scoring.rs:107–110` converts `i32::MIN.unsigned_abs()` back to
i32 before allocating. An executable probe compiled the current function body
unchanged: the 18-hole allocation summed to +2,147,483,646 instead of −2,147,483,648,
and a one-hole debug call panicked on negation. The same probe checked every i16
handicap across 18 holes: all 65,536 sums were correct. Current stored Playing
Handicaps are i16, so no reachable current score-result failure was established.
Use widened arithmetic or a typed overflow result and test endpoint conservation.

### L2 — invalid course selections receive the wrong format explanation

`backend/src/repositories/round_configuration.rs:85–91` reports
`InvalidFourBallLayout` for an invalid Stableford layout as well. The mapping in
`backend/src/api/rounds/configuration.rs:254–255` says “four-ball requires 18 holes.”
Singles match play omits this explicit check and falls through the database
constraint mapping. The schema still rejects invalid layouts; the failure is
error-contract quality. Add format-aware validation and preserve atomic rollback.
This finding is source-confirmed; no new course-selection HTTP reproduction ran.

### L3 — match-only results remove the tournament selector

`frontend/src/pages/LeaderboardPage.tsx:87` returns the match result component
before the shared selector; `MatchResultsPage.tsx:25` offers only links within the
same tournament. Multi-tournament users cannot switch trips from global results
after selecting a match-only trip. Keep tournament selection above format dispatch.
This finding is source-confirmed; no new multi-tournament browser reproduction ran.

## Review scope and validation

The primary reviewer inspected shared arithmetic, overall selection and application
integration. Independent read-only reviewers covered backend persistence/API/
concurrency, frontend queues/private results, and match/Stableford/four-ball domain
rules and projections. Findings were checked against current code and the preserved
product invariants, rather than inferred from previously passing tests.

New focused evidence:

- One Chrome diagnostic reproduced failed-device-save loss after external lock.
- One Chrome diagnostic reproduced cached private-card display after a 403 refresh.
- Two PostgreSQL/API diagnostics reproduced expired-session legacy save and confirm.
- The source-extracted Rust probe reproduced the i32 boundary defect and verified
  allocation conservation across all 65,536 current i16 handicap values.
- Final documentation/diff checks and read-only plan review cover severity, bounded
  repair ownership, invariants and acceptance criteria.

The four diagnostic scenarios pass when they observe the existing defect; they
are not repaired regression tests or evidence that these paths are correct.
Temporary probes and logs are under `/tmp/golf-review-browser/`,
`/tmp/golf-review-browser.log`, `/tmp/golf-review-private-denial.log`,
`/tmp/golf-review-session-expiry.log` and `/tmp/golf-review-allocator.log`.
They are not published as application tests. Reproduction steps and source evidence
above are retained so each implementation step can add a regression that fails
before its repair.

No new valid-input defect was confirmed in inspected match ledger ordering,
concessions/rulings, early finish/draw, exact point awards, match-relative handicaps,
Stableford pickup/equivalent arithmetic or four-ball independent gross/net selection.
This is a bounded review conclusion, not proof that the entire application has no
other defects. Team ownership, snapshot and visibility invariants remain repair
constraints; no automatic team generation or pairing changes are proposed.

The full backend/frontend/PostgreSQL ladders, exhaustive browser matrix, migration/
seed reruns, Docker deployment, external golf-rule research, performance profiling
and operational security audit were not rerun in this review-only step. Production
code and schemas were unchanged; focused reproductions were selected to establish
the findings. Earlier release tests are historical evidence, not new passes.

## Outcome

**READY for remediation planning.** The high finding warrants repair first;
this review does not mark the existing defects fixed or issue a clean application
release verdict. Implement H1, then M1 and M2, then the low findings individually
with the prescribed ladders, review and documentation. Performance measurement
and the broader security review remain later separate work.
