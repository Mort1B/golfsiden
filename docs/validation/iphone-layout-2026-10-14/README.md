# iPhone layout acceptance — 2026-10-14

Scope: existing flows on single-screen iPhone 15 Pro/Pro Max and newer, with
Chrome and Safari equal acceptance targets. Duo/foldable-specific layouts,
backend/API changes and a visual redesign are excluded.

## Reproduced issues and changes

- At 852×260, the existing 70px sidebar targets placed the first link at y=-9.5.
  Short landscape views now use compact targets, safe alignment and a vertically
  scrollable rail. The 700px rail breakpoint remains; no device detection is used.
- With simulated landscape left/right insets of 59px, the account control's
  right edge was 834px instead of staying within 793px. The body now owns all
  four content safe areas; fixed navigation owns its separate inset padding.
  `viewport-fit=cover` is paired with this spacing, including public forms.
- Review identified a portrait target regression when a 34px home-indicator inset
  is active. The browser reproducer measured 39px links. Links now explicitly
  retain a 44px minimum, independently of inset padding.
- At 200% root text size, navigation labels were ellipsized. They now wrap, and
  mobile content/scroll clearance grows with root text size.
- Shell/public minimum heights use `100svh` with a `100vh` fallback, subtracting
  the outer vertical safe area. Private child pages no longer add a second full
  viewport. Sign-in fields use 1rem text; textarea typography inherits the app
  font. Zoom remains enabled.

No React provider, route lifetime, input handler, score calculation, API or
persistence behavior changed. There are no JavaScript resize/focus workarounds.
Small viewport units follow [WebKit's viewport guidance](https://webkit.org/blog/12445/new-webkit-features-in-safari-15-4/);
edge spacing follows [WebKit's safe-area guidance](https://webkit.org/blog/7929/designing-websites-for-iphone-x/).

## Environment and results

Chrome 155.0.8059.79 ran locally on Fedora 44 with Node 22.22.2. WebKit ran in the
matching `mcr.microsoft.com/playwright:v1.63.0-noble` container, with the checkout
mounted read-only. The downloaded Ubuntu WebKit build could not run directly on
Fedora because libicu74/libjpeg-turbo8 were missing; no host packages were changed.
Browser execution required local sandbox escalation after Chrome's socket setup
was denied. The initial browser download also needed network escalation.

Real-flow tests used owned PostgreSQL 17 container `golf-iphone-20261014-pg` on
127.0.0.1:55447, migrated/seeded through schema 39, the current API on 3000 and Vite
on 5173. All users, tournaments and scores were synthetic. Shell/public state
checks use intercepted API fixtures and a local SSE fixture; they are not real
backend acceptance. No hosted infrastructure was accessed. The owned API/Vite processes were stopped
and the disposable database container removed after validation. Backend tests were
not rerun because no backend/schema code changed; current migrations and seed were
run to prepare the browser database.

| Check | Result |
| --- | --- |
| Frontend unit/component tests | 1,100 passed across 141 files |
| Frontend typecheck, lint, production build | Passed |
| Strict browser TypeScript (`tsconfig.browser.json`) | Passed |
| Final Chrome iPhone suite | 24 passed |
| Final WebKit iPhone suite | 23 passed; 1 Chrome-only CDP test skipped |
| Retained Chrome keyboard/navigation suite | 10 passed |
| Retained scoring/Fantasy preset/award suite | 13 passed; 1 status-assertion failure passed unchanged on focused rerun |
| Independent read-only review | Initial touch-target finding fixed; no remaining findings |
| Whitespace, 52 local doc links/anchors, affected source sizes | Passed |

One retained Fantasy preset case failed its assertion that every observed error
response was the injected 503. Its UI/recovery assertions had passed. An unchanged
focused rerun with a diagnostic trace passed; the original unexpected status was
not captured by that assertion and its cause remains undiagnosed. No production
change is attributed to this transient result. Logs: `/tmp/iphone-retained-flows.log`
and `/tmp/iphone-preset-recheck.log`; rerun trace: `/tmp/iphone-preset-recheck/`.

## Coverage and test boundaries

The new matrix includes 320×600; 390×844; 393×852; 402×874; 430×932; 440×956;
landscape counterparts; 852×260; and 1280×900. These are logical CSS viewports, not
physical device emulation or a guarantee about unreleased hardware.

- Shell, profile, tournament overview and score navigation are checked across
  every matrix size, including target dimensions and page overflow.
- Real stroke, scramble, four-ball and Stableford scoring/summary views retain
  the selected hole and leave server score totals unchanged during resizing.
  Focus and deliberate scrolling make the final enabled score control reachable
  at 393×360. This does not simulate the native software keyboard.
- Fantasy picks/captain remain selected through resizing; no lineup writes occur
  during the matrix sweep. Copy review, per-round presets, acknowledged refresh
  recovery, boards and rules fit. Two deliberate saves produce two lineup writes.
- Completed-round management/award form drafts survive resizing. A focused
  textarea is reachable by scrolling at 393×360. Award submission, net standings
  and player history render through the real API.
- Public home, login, onboarding tournament/round/player steps, missing invitation
  and populated join forms are checked. Profile loading, 503 error/retry, empty
  state and long names are covered. Public results and match scoring use fixtures.
- Chrome CDP injects landscape 59px side/21px bottom insets and portrait 59px top/
  34px bottom insets. These assertions verify CSS geometry, not native device UI.
- A 200% root-font scenario checks profile overflow and fully wrapped navigation
  labels. It is text enlargement, not native browser page zoom.
- Screenshots supplement geometry assertions. Representative Chrome scoring,
  Fantasy, onboarding, landscape inset and enlarged-text views, plus WebKit
  portrait scoring and short landscape navigation, were inspected.
  Focus/occlusion checks cover selected final controls and the retained keyboard
  traversal suite; they do not assert every control in every possible state.

The initial test failures included outdated mocked account/session responses,
missing public-link fragment syntax, navigation before auth reads settled, and a
Fantasy save paused for read reconciliation. Fixtures now use 401 for signed-out
sessions, settle public auth reads, and exercise the existing read-only recovery
before saving the remaining round. An extra scorer reachability assertion first
selected a button absent from summary mode; it now selects the final enabled
score control. Already-focused controls require an explicit test scroll after a
resize/top-of-page capture. None of these harness repairs changed runtime logic.

Event observers retain unexpected console, HTTP and request errors. The shared
fixture recognizes WebKit cancellation only for its SSE stream and the specific
public-results POST read; mutation cancellations are not accepted as successful
writes. Expected signed-out 401 and injected profile 503 responses are identified.
Real-flow observation permits navigation-aborted GETs in either engine. Initial
WebKit harness failures are preserved in `/tmp/iphone-webkit-full.log`.

## Reproduce

With a disposable seeded API on 3000 and Vite on 5173, from `frontend/`:

```bash
npm run test
npm run typecheck
npm run lint
npm run build
npx tsc -p tsconfig.browser.json --noEmit
GOLF_IPHONE_API=1 npx playwright test --config playwright.iphone.config.ts --project=chrome
GOLF_IPHONE_API=1 npx playwright test --config playwright.iphone.config.ts --project=webkit
npx playwright test --config playwright.routes.config.ts navigationAccessibility.browser.ts
GOLF_PLAYING_DAY_BROWSER=1 GOLF_FANTASY_BROWSER=1 GOLF_AWARDS_BROWSER=1 npx playwright test --config playwright.lifecycle.config.ts playingDay.browser.ts fantasyPreset.browser.ts roundAwards.browser.ts
```

WebKit requires a Playwright browser installation and compatible dependencies.
For this Fedora host, the equivalent isolated invocation was run from the repo
root after pulling the matching image:

```bash
podman run --rm --network host --security-opt label=disable \
  -e GOLF_IPHONE_API=1 -v "$PWD:/work:ro" -v /tmp:/tmp -w /work/frontend \
  mcr.microsoft.com/playwright:v1.63.0-noble \
  npx playwright test --config playwright.iphone.config.ts --project=webkit
```

The image's bundled browsers supply the WebKit executable. Chrome uses the
installed Chrome channel. The new API cases explicitly skip without
`GOLF_IPHONE_API=1`; do not treat a run without that flag as full acceptance.
Logs are `/tmp/iphone-*.log`; screenshots are `/tmp/iphone-{chrome,webkit}-*.png`.

## Outstanding physical-device acceptance

No physical iPhone or iOS Simulator is available on this Linux host. Installed
Chrome on iPhone and Safari on iPhone each still need checks on iPhone 15 Pro and
a newer/larger model: portrait/landscape, expanded/collapsed browser toolbars,
software keyboard and focused fields, safe areas, native 200% zoom and scrolling
to final controls. Record model, iOS, browser version and results separately for
each browser. Desktop Chrome/WebKit and CDP inset simulation do not replace this
acceptance. Duo/foldables are excluded. Hosting remains operator-owned.
