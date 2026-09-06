# Correct provisional tournament ties and reduce live refresh work

A real two-session browser test reproduced the reported class of failure. On the
fresh seed, scramble updates worked, but the first individual even-par score
produced an invalid tournament result. The score was persisted and round results
updated. Tournament ranking incorrectly set `tied: true` by comparing the last
ranked player with the next unstarted player: both had zero completed qualification
count and zero score-to-par. The frontend correctly rejected this inconsistent
response, so the page could retain old totals alongside an error.

The backend now checks that the next player has a selected contribution before
using that player to declare a tie. Unstarted players remain unranked. Genuine
sporting ties, gross/net selection, qualification, best-N rules, and preserved
team ownership are unchanged. A new domain regression failed on the original
code; the fix passes that regression, a genuine-tie control, and a PostgreSQL/API
regression for both gross and net even-par boundaries. The browser regression
also failed on the original server response and passes with the fix. This is a
confirmed defect, although it cannot prove the user's original device observation
had exactly this cause.

Two correctly unchanged totals were also reproduced: a score in an older open
round does not enter tournament standings while a higher-numbered round is open;
and an excluded best-N contribution can change while the selected total stays
the same. The latter response and contribution refreshed successfully. Mandatory
round slots and administrator-controlled final visibility remain intact.

The measured optimization narrows ordinary `score` invalidations to the current
user's leaderboard, round completion-validation, and read/scoring-card query
families. Saves and confirmations do not change tournament setup, roster, teams,
or score-access metadata. A settled score event previously triggered three GETs
in the receiving tournament-results session: tournament list, rounds, and
leaderboard. It now triggers two (rounds and leaderboard), a 33% reduction in that
scenario, measured for scramble, individual, and foursomes rounds. The rounds read
is retained for runtime lifecycle validation. The active-observer regression
failed before the optimization and passes afterward. Structural events retain
broad refreshes; visibility, disconnect, and reconnect still clear projected data.
No SSE payload, authorization contract, database schema, or scoring policy changed.

Validation completed:

- Backend: formatting, 116 regular tests, strict Clippy across all targets/features.
  The first sandboxed run could not bind provider-test sockets; rerunning with
  local socket access passed without changing those tests.
- PostgreSQL: full database-enabled workspace ladder, 358 tests passed. Migration
  and seed commands passed on disposable databases; no migration was added.
- Frontend: 389 tests across 65 files, strict type checking, ESLint, production
  build, browser TypeScript, and diff whitespace checks passed.
- Chrome: separate same-account desktop and mobile sessions exercised real UI
  saves in scramble, individual, and foursomes; gross/net REST and rendered totals;
  same-session results navigation; failed saves; older-open and best-N exclusions;
  completed corrections; locked rejection; actual API crash/native reconnect;
  and a separate member's mandatory final hide/release/re-hide. Layout checks ran
  at 320px, 390px, and 1280px, including failed-save and disconnected states.
  Unexpected console/network failures are checked and the settled two-GET budget
  is asserted. Measurements: `/tmp/golf-leaderboard-baseline.json` and
  `/tmp/golf-leaderboard-optimized.json`; screenshots:
  `/tmp/golf-leaderboard-live-*.png`.
- Both existing lifecycle browser regressions passed, covering organizer readiness,
  confirmations/corrections, permissions, final visibility, loading, retry, empty,
  and long-content states at mobile and desktop widths.
- Read-only review found no remaining issues in the ranking fix, regression tests,
  or concrete invalidation dependency set.

The phone tests use Chrome mobile emulation, not physical iPhone/Safari. Native
SSE was connected directly to the disposable API for the crash test because Vite's
development proxy retained its downstream stream after the upstream process died.
Production Caddy restart behavior was not re-tested in this step. The existing
approximately 611 kB minified bundle warning remains. Backend loading already uses
bulk reads; a possible duplicate current-round calculation was identified but not
changed without profiling evidence that would justify further backend scope.

Release verdict: **READY WITH KNOWN LIMITATIONS** for the tested runtime and
browser matrix. The live-update defect and measured request optimization are
complete; this does not imply a general throughput benchmark or physical iOS
certification.
