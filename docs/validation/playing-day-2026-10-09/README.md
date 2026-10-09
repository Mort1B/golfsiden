# Playing-day frontend consistency — 2026-10-09

## Scope and before-change evidence

Baseline: `9492790c2668778a3a0bbc91079b87ee50e67348`, clean `main`.
One frontend step, with no backend, migration, score formula, queue protocol,
request deduplication or confirmation-hook changes.

Before production edits, source inspection confirmed summary-before-controls on
stroke cards, selectors-before-input and missing tournament/round context in the
other writable formats, ambiguous server-only progress, permanently expanded
Fantasy editing, and missing loading/empty/retry states on the overview. Three
new actual-parent regressions failed (41 other tests passed): incomplete review
at hole 18/control order, complete review away from hole 18, and compact Fantasy
section/editing behavior. `/tmp/golf-playing-day-before.log` recorded that run.

## Implemented boundaries

- One scoring hierarchy, existing format-specific inputs/results/confirmation,
  quick card switching and view controls above summaries. Explicit review follows
  actual distinct-hole progress; keyboard activation focuses a retained region.
- Display-only progress overlays current pending/refreshing/nondurable input with
  exact account/tournament/round/protocol/owner scoping. Corrections do not inflate
  counts; local values never feed authoritative totals or confirmation.
- Fantasy sections retain their mounted children and provider-owned input. Saved
  valid lineups show a compact receipt and explicit editor, with focus management.
  Uncertainty remains visible across sections; privacy clearing still removes
  protected projections. No offline Fantasy writes or new storage.
- Overview explicit asynchronous states and one contextual resume action after
  connected round reads plus fresh existing access. Session replacement remounts
  the access check. Remembered round IDs are validated against current rounds.
- Scoped 14px secondary/16px instruction text, wrapping, 44px+ targets; narrow
  overview headers place status below long titles.

## Validation

Frontend commands from `frontend/`:

```sh
npm run test
npm run typecheck
npm run lint
npm run build
```

Final full unit/component run: **1,036 passed across 137 files**. Strict typecheck,
ESLint and production build passed. The first full run had one new test timing
failure (1,035 passed): Next was clicked before IndexedDB persistence released
its guard. The test now waits for that existing guard; production behavior was
not relaxed. Focused tests before that run passed 81/81. The final full run also
includes two new overview disconnect/session-remount cases.

All 18 changed production files remain below 400 nonblank/noncomment lines
(largest: ScorePage, 202). `git diff --check` passed.

Read-only review found two focus losses in the initial implementation (review
button and Endre valg unmounted). Both were repaired with retained-region/editor
focus and parent regressions. Final review reported no remaining findings.

Browser environment: installed headless Chrome, Vite at `127.0.0.1:5173`, existing
unchanged local API binary, disposable PostgreSQL database `playing_day_20261009`
in rootless Podman `golf-fantasy-f3-pg` on port 55445. Migration setup succeeded.
Real accounts, round setup, writes and scoring confirmations use that database.
Offline mode, held/failed reads, response loss and live disconnects are explicit
browser boundary injections. Overview empty/error states are intercepted reads.

```sh
GOLF_PLAYING_DAY_BROWSER=1 GOLF_FANTASY_BROWSER=1 GOLF_RELIABILITY_BROWSER=1 \
  npm run test:browser:lifecycle -- playingDay.browser.ts fantasy.browser.ts \
  fantasyPrivacy.browser.ts fantasyRelease.browser.ts fantasyReleaseFormats.browser.ts \
  reliability.browser.ts reliabilityFollowup.browser.ts

GOLF_OFFLINE_BROWSER=1 GOLF_FOUR_BALL_BROWSER=1 GOLF_STABLEFORD_BROWSER=1 \
  GOLF_SCORING_FLOW_BROWSER=1 npm run test:browser:lifecycle -- \
  offlineScoring.browser.ts offlineLifecycle.browser.ts fourBall.browser.ts \
  stableford.browser.ts stablefordOffline.browser.ts scoringFlow.browser.ts
```

Playing-day/Fantasy/recovery run: **21 passed** in 2.1 minutes. Includes all seven
new scenarios. The release tests reported zero unexpected console/failed-request
errors; four expected retryable Fantasy projection conflicts were captured by
the core Fantasy test and zero 409 bodies were unavailable. Recovery tests permit
only their injected failures and expected conflict/session statuses.

Existing scoring/offline run: **18 passed, 1 test-locator failure** in 2.7 minutes.
The failing scoring-flow assertion matched two loading messages during a route
transition; it now selects the scoring page's own loading state. No production
change was made for this failure. Final targeted rerun: **8 passed** in 1.3 minutes (all seven playing-day scenarios
with explicit Tab/Shift+Tab assertions plus the corrected scoring-flow test).
Thus **40 distinct affected Chrome scenarios passed**, without skipping any
scenario in the invoked suites. No pending validation remains within this scope.

```sh
GOLF_PLAYING_DAY_BROWSER=1 GOLF_SCORING_FLOW_BROWSER=1 \
  npm run test:browser:lifecycle -- playingDay.browser.ts scoringFlow.browser.ts
```

The seven new scenarios cover four writable formats with local correction/new
hole counts, server revalidation, review from hole 4 and explicit confirmation;
four real writable cards; Fantasy compact editing/keyboard section changes and
uncertainty; and overview loading/error/retry/empty/populated/contextual entry.
Existing browser suites exercise privacy, conflicts, exact retries, claims, lost
acknowledgement, server totals, pickups, confirmation leases, storage failure,
logout/account isolation and scored histories. Widths: 320, 390 and 1280px.
Layout helpers check horizontal overflow, scoring touch targets and trial clicks
after scrolling (fixed-navigation interception), with screenshot inspection.
Full-page screenshots naturally show the fixed menu at the captured viewport's
edge; scroll/click checks establish that controls can be reached above it.

Initial new browser run failed due to test selectors (wrong summary class and
format-specific plus-button names) and one acknowledged Fantasy refresh failure.
After selector corrections six scenarios passed. The Fantasy scenario explicitly
waits for the acknowledgement outcome and uses the existing read-only retry if
needed; it never retries a write. Original logs remain under `/tmp/`.

## Limits

No hosted gg26.no deployment, physical phone, screen reader or native 200% zoom
acceptance was performed. Automated Chrome keyboard/viewport checks are not
physical-device testing. Backend/DB test ladders were not rerun: no backend or
schema behavior changed. The database migration was environment setup, not a new
migration test. Broad unrelated browser suites were outside this bounded step.
The overview checks one candidate round and explains its access; it does not
search every round for another writable card. Existing online-only confirmation,
open-session offline return and memory-only Fantasy recovery limits remain.

## Retained evidence

- [Full frontend run](frontend-tests.log), [typecheck](typecheck.log),
  [lint](lint.log), [build](build.log), [migration setup](migration-setup.log).
- [Playing-day/Fantasy/recovery browser run](fantasy-browser.log),
  [first scoring run including the locator failure](scoring-browser-first.log),
  [final keyboard/scoring rerun](browser-final.log).
- [Offline stroke](playing-stroke-pending-320.png),
  [desktop review](playing-stroke-review-1280.png),
  [four-ball mobile](playing-four-ball-pending-320.png),
  [Stableford desktop](playing-stableford-pending-1280.png),
  [quick card rail](playing-quick-cards-320.png).
- [Compact Fantasy](playing-compact-320.png),
  [uncertainty on boards](playing-warning-on-boards-320.png).
- Overview [populated](playing-overview-populated-320.png),
  [retryable error](playing-overview-error-320.png),
  [empty](playing-overview-empty-320.png).
