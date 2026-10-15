# Latest iteration — bounded team-member loading

A8 replaces the shared team-list loader's per-team query loop with one query for
all selected teams' members. Empty lists retain one data SELECT; populated lists
use two regardless of team count. For example, 100 teams previously required
101 data SELECTs and now require two. Authorization and transaction work remain
additional and unchanged.

The team query keeps native name/UUID order. One bound UUID array limits member
loading to those selected teams, with the original position/null/name/player-ID
ordering inside each team. Lookup-only grouping keeps that SQL ordering and
includes teams with no members. No new API fields, schema, indexes or frontend
behavior are introduced.

Both repository wrappers and all team mutations are byte-identical to baseline.
The protected read still owns repeatable read, session/user and membership
locks, one complete serialization retry and the final wall-clock session check,
even when the result is empty. The internal wrapper retains its existing
isolation. HTTP cache/error behavior and live events are unchanged.

The original loader fails the new query-budget test for 12 and 100 teams on all
three actual read paths: internal repository, protected repository and Axum
HTTP handler. The repaired loader passes 0/1/12/100-team budgets. Complete
response hashes match across revisions and every response matches the original
SQL oracle. Focused PostgreSQL/API coverage also retains empty teams, nullable
schedules, null/duplicate member positions, native SQL collation, player UUID
ties, sibling rounds and foreign-tournament denials. All 14 existing session
regressions pass. Backend formatting, 264 unit tests and strict Clippy pass.

The full PostgreSQL ladder passes 933 tests, with five manual/exclusive probes
ignored; the new exclusive measurement was run explicitly. Migration and seed
checks pass against PostgreSQL 17.10 with all 39 migrations. The final serial
local HTTP median for 100 teams falls from 31.35 to 12.74 ms. Empty and single-
team HTTP medians are slower; this warm synthetic debug-build probe establishes
neither an overall product speedup nor hosted/frontend latency. It includes
application handling and JSON collection, excluding TCP, TLS, proxy and browser
work.

Independent design/source review found no production issue. Review caught and
closed a stale-output issue in the rerun harness. A serial comparison also
caught reused baseline build artifacts; those current results were discarded,
the package was rebuilt and the repaired query-budget probe passed again.
The [A8 report](validation/team-loading-2026-10-15/README.md) retains commands,
source/probe hashes, exact response parity, final samples, failed-attempt context
and cleanup evidence. Final independent review recalculated the retained results
and found no issues. The next candidate in [the plan](PLANS.md) is the remaining
P3 course-read authority boundary; no queued implementation began.
