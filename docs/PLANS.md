# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. The next bounded candidate is L3 below; implementation awaits approval.

## Priorities

| Priority | ID | Finding | Repair order |
| --- | --- | --- | --- |
| Low | L3 | Match-only results remove tournament selection | 1 |

Low means a recoverable UX/error-quality issue. Severity reflects demonstrated
impact, not the amount of code to change.

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

- **Validation follow-up:** investigate the intermittent existing
  `returnLoading.browser.ts` offline/frozen-page return case. One full run timed
  out before read-only display; three unchanged isolated repeats passed. Its
  cached enabled-input assertion may precede completion of the online refresh,
  allowing a subsequent return to coalesce with it. Confirm the cause before
  changing lifecycle behavior or tightening the test's recovery barrier.

1. **Performance work:** measure representative workloads before scoping changes,
   including the existing frontend bundle warning and bounded match-card reads.
2. **Security review:** perform the separately scoped wider application/operational
   review after the above correctness repairs.

No automatic opponents/byes/brackets, team match play, extra holes, new scoring
rules, public match sharing, cold offline launch or background sync is included.
