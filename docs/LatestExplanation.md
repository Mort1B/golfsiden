# Authoritative Fantasy results

FANTASY-4 adds private round and overall Fantasy leaderboards, with separate
manager and golfer standings and detailed golfer/manager breakdowns. Results use
preserved net scores, current accepted match outcomes and the locked selections
from FANTASY-3. Schema remains 37. The Fantasy screens are the next step; these
APIs alone do not make the game playable in the application.

Non-match adapters cover individual stroke play, Stableford, scramble, foursomes
and four-ball. Teams receive placement once, then both partners receive the shared
hole and placement points. Nine two-person teams therefore have nine placing
units and 18 golfer rows. Stableford uses native net placement and uncapped actual
strokes for Fantasy holes. Four-ball distinguishes unresolved partner input from
explicit pickup and checks both raw inputs for a physical ace. Match play awards
only +3/+1/-1, including early concessions and corrected outcomes.

Captains double the entire manager contribution, including negative scores;
golfer standings always retain base points. One test has base results of 22 and
−58, plus two non-finishers with zero recorded points: captaining the −58 golfer
produces a manager total of −94 while that golfer remains at −58. All scheduled
rounds count, with future rounds, explicit nonparticipation, pending data,
provisional results and settled results distinguished. An entirely future entry
has no earned rank. Current non-finish dispositions retain recorded points and
mark unplayed holes as omissions; stale dispositions return pending.

Result reads use consistent bulk facts and the existing temporal selection
closure. Scoped missing targets roll back any lazy closure. Pre-lock reads keep
other managers' picks private. Hidden final-round facts are filtered before
points, settlement, ranking and response revision calculation. Tests compare
whole member responses through concealed score and match changes, confirmations
and corrections. Canonical source-token compatibility with FANTASY-3 is tested
against its frozen query, so batching does not invalidate existing dispositions.

Review also corrected the existing Fantasy round notifications to use tournament
scope and round resource identity. New result routes map malformed UUID paths to
the normal JSON error contract. Regression tests cover both fixes, scoped-read
rollback and non-finish omission states. Read-only scoring/owner and API/privacy
reviews have no outstanding source findings.

Validation:

- Default backend tests: 249 passed.
- Focused PostgreSQL run: 36 tests passed across ten Fantasy suites, including
  17 new result tests and existing selection, race, source and runtime-role tests.
- Strict all-target/all-feature Clippy passed. Existing vendored SQLx warnings
  remain; the new test fixture's collection lint was corrected.
- Full PostgreSQL backend ladder: 712 tests passed, including existing sporting
  regressions and the populated schema upgrade test. Migration replay and CLI seed
  twice passed on disposable PostgreSQL 17; schema remains 37.
- Formatting, whitespace checks, 92 local Markdown links and the production-file
  size limit passed. Final read-only contract/documentation review found no
  discrepancies.

Informational local debug measurements on disposable PostgreSQL 17 were 59 ms for
the first result read and 32 ms for the repeat with 9 teams/18 manager entries;
20 teams/40 entries measured 108/52 ms. Each fixture has three scheduled rounds,
one played round, one submitted lineup and otherwise missed selections. These
single measurements do not establish production capacity or worst-case
carry-forward latency. Projection facts load in bulk, while first closure still
resolves and writes per entry.

No frontend or browser validation ran because this step changes no frontend code
or screens. No production deployment or new restore rehearsal was performed.
The [next candidate](PLANS.md#next-candidate) is FANTASY-5: the complete mobile-first
Fantasy UI, including live query invalidation and round/overall manager and golfer
standings.
