# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. The next candidate is **M2 — hide private projections after authoritative
denial**, awaiting a new implementation instruction. L1–L3 remain queued.

## Priorities

| Priority | ID | Finding | Repair order |
| --- | --- | --- | --- |
| Medium | M2 | Cached private results remain visible after a denied refresh | 1 |
| Low | L1 | Generic handicap allocator mishandles `i32::MIN` | 2 |
| Low | L2 | Course selection reports an incorrect or generic format error | 3 |
| Low | L3 | Match-only results remove tournament selection | 4 |

Medium means an authority or private-display contract fails under a concrete
transition. Low means a bounded domain edge outside current snapshot inputs or recoverable UX/error
quality. Severity reflects demonstrated impact, not the amount of code to change.

## Medium — M2: hide private projections after authoritative denial

**Trigger:** direct scorecards, player history and round results keep cached data
when a refetch returns 401/403/404. A healthy SSE connection need not immediately
clear the cache. Chrome reproduced a direct card still displaying gross 7 after
its card endpoint returned 403.

**Scope and repair:** `frontend/src/pages/DirectScorecardPage.tsx`,
`PlayerHistoryPage.tsx`, `LeaderboardPage.tsx` and focused private-query/error
helpers. Distinguish authority denial from transient network/server failures.
Immediately suppress and invalidate/remove affected private projections on denial;
a retry may render data only after current authorization succeeds. Preserve
permitted transient-error recovery and H1 local-only input recovery. Inspect the
same shared rendering helper's consumers without broad UI refactoring.

**Validation:** successful card/history/round-result read → denied background
refetch while SSE stays healthy; prior names, scores and links disappear. Exercise
401/403/404 separately, successful recovery and transient 500/offline behavior.
Keep hidden-final and same-user session refresh behavior intact. Run the frontend
ladder and representative mobile/desktop Chrome privacy cases.
**Stop:** fail-closed private rendering only; no new public scope or SSE redesign.

## Low — L1: make the generic allocator safe at the signed minimum

**Evidence:** `backend/src/domain/scoring.rs:107` casts `unsigned_abs()` back to
i32. For `i32::MIN`, 18 allocations sum to +2,147,483,646 instead of −2,147,483,648;
one-hole debug allocation panics. All 65,536 i16 handicaps conserved their expected
18-hole sum in the review probe, and production snapshots use i16. This is a
latent helper-contract defect, not an observed current score-result failure.

**Scope and repair:** retain a widened/unsigned magnitude until the signed result
is representable, or return a typed overflow error. Preserve allocation order and
legacy signed-rounding behavior. Do not change historical snapshots or broaden
this into general handicap-policy work.
**Validation:** test i32 minimum/maximum, ordinary plus handicaps, 1/9/18-hole
allocation, invalid arguments, sum conservation and debug/release parity. Run the
backend ladder and relevant scoring-format regressions.
**Stop:** allocator boundary and direct tests only.

## Low — L2: report the correct 18-hole course requirement

**Evidence:** `backend/src/repositories/round_configuration.rs:85` returns
`InvalidFourBallLayout` for Stableford too; the API reports “four-ball requires
18 holes.” Match play is absent from that explicit validation and reaches a
generic constraint response. PostgreSQL still rejects invalid layouts.

**Scope and repair:** introduce a format-aware layout error across the repository,
API mapping and affected frontend error mapping. Reject non-18-hole selections
for four-ball, Stableford and singles before attempting round configuration;
preserve transaction rollback and permitted 9-hole legacy formats.
**Validation:** submit nine-hole manual and relevant saved/provider tee selections
for each supported 18-hole format, assert actionable correct errors and unchanged
round/course configuration. Run affected backend/PostgreSQL ladders; run frontend
checks/browser validation if user-facing mapping changes.
**Stop:** format-specific validation/error quality only; no course-provider expansion.

## Low — L3: retain tournament selection in match-only results

**Evidence:** `frontend/src/pages/LeaderboardPage.tsx:87` returns `MatchResults`
before the shared tournament selector. Its navigation links remain within that
trip. A multi-tournament account must leave global results to switch away from a
selected match-only tournament.

**Scope and repair:** keep the global tournament selector above the format-specific
result body. Preserve match-only query suppression, private player scope, mixed
overall links and route canonicalization.
**Validation:** an account with match-only and stroke/mixed trips switches in both
directions from `/leaderboard`, including direct entry and back/forward navigation,
at mobile and desktop widths. Run the frontend ladder and focused Chrome checks.
**Stop:** selector/discovery repair only, with no standings redesign.

## Later queue

1. **Performance work:** measure representative workloads before scoping changes,
   including the existing frontend bundle warning and bounded match-card reads.
2. **Security review:** perform the separately scoped wider application/operational
   review after the above correctness repairs.

No automatic opponents/byes/brackets, team match play, extra holes, new scoring
rules, public match sharing, cold offline launch or background sync is included.
