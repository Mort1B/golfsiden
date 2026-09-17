# Name the correct course-layout requirement

L2 repairs course-configuration errors for formats requiring 18 holes. Previously,
Stableford returned a four-ball error, while singles match reached a generic
PostgreSQL constraint error. Four-ball and Stableford also attempted to insert a
course revision before rejecting the layout and rolling the transaction back.

The repository now rejects incompatible validated tee facts before insertion,
after locked session/admin authorization and draft/version checks. Its three
typed errors map to private HTTP 409 responses:

| Format | Error code |
| --- | --- |
| Four-ball | `four_ball_requires_18_holes` |
| Stableford | `stableford_requires_18_holes` |
| Singles match | `singles_match_requires_18_holes` |

Manual, saved-course and provider facts share this boundary. Rejection preserves
round configuration, timestamps and the full course/tee/hole hierarchy and emits
no SSE. Individual stroke play, scramble and foursomes still accept nine holes.
There is no schema or scoring-policy change.

The browser uses these same codes for local manual/saved checks and displays
Norwegian guidance naming the format. For example, selecting a nine-hole saved
tee for Stableford explains that Stableford requires exactly 18 holes and asks
the administrator to choose an 18-hole tee. The choice stays selected; correcting
it saves normally and restores focus to the round's edit button. The manual form
continues to hold restricted formats at 18 holes.

## Validation

- Backend regression proof: five failures before the fix, with the legacy control
  passing. A nontransactional sequence probe proves rejection happens before an
  attempted course INSERT, rather than merely relying on rollback. The focused
  configuration suite passes all 16 tests after repair.
- Frontend regression proof: six new assertions failed before the fix; all 18
  focused tests pass afterward, including correct codes/messages, no request for
  invalid local facts, and successful correction.
- Backend ladder: formatting, all 208 tests and Clippy with all targets/features
  and warnings denied passed. PostgreSQL 17 validation passed all 578 tests with
  database tests enabled. Migration application and reapplication succeeded;
  all 32 migrations are marked successful. Seed created eight players/five rounds.
- Frontend ladder: all 608 tests, strict type checking, lint and production build
  passed. The existing 500 kB bundle-size warning remains queued separately.
- Chrome: three new format scenarios and both existing saved-course scenarios
  passed. New scenarios verify real API 409 responses and unchanged rounds,
  locally rejected saved selections, successful 18-hole saves, receipt/focus,
  fixed manual hole counts, error text, long names, no horizontal overflow and
  reachable 44-pixel controls at 320×600, 390×844 and 1280×900. Existing scenarios
  cover loading, empty, failure/retry and populated states. Screenshots were
  inspected; checks reported no unexpected console errors or failed requests.
- The initial browser run exposed an incorrect test locator, corrected to the
  select's accessible role/name. Type checking also caught an untyped viewport
  tuple in the new test; the corrected final checks pass.
- Read-only backend and frontend review found no issues. `git diff --check`
  passed. Modified production files remain below the 400-line limit.

Evidence logs: `/tmp/l2-backend-red.log`, `/tmp/l2-backend-green.log`,
`/tmp/l2-backend-full.log`, `/tmp/l2-clippy.log`, `/tmp/l2-pg-full.log`,
`/tmp/l2-migrate.log`, `/tmp/l2-migrate-reapply.log`, `/tmp/l2-seed.log`,
`/tmp/l2-frontend-red.log`, `/tmp/l2-frontend-green.log`,
`/tmp/l2-frontend-full.log`, `/tmp/l2-typecheck.log`, `/tmp/l2-lint.log`,
`/tmp/l2-frontend-build.log`, `/tmp/l2-browser.log` (existing scenarios) and
`/tmp/l2-browser-final.log` (new scenarios). Screenshots use
`/tmp/l2-course-<format>-<width>.png`.

## Limits and readiness

**READY WITH KNOWN LIMITATIONS.** Live provider HTTP configuration was not
exercised: the bundled catalog has no usable provider rows. Provider adapter to
real PostgreSQL repository rejection/acceptance was verified for all three
formats, and the existing provider API suites passed. Browser nine-hole saved
facts are an explicitly intercepted read fixture; all rejected direct API writes
and successful corrected writes use the real backend. No production service or
external provider was changed.

L2 is complete. L3 tournament selection in match-only results remains next;
queued lifecycle-test investigation, performance measurements and wider security
review are unchanged.
