# Navigation accessibility repair

This bounded repair closes UI-1, UI-2 and UI-3 from the
[friends deployment assessment](../friends-2026-09-23/README.md).
Production changes are confined to `frontend/src/styles.css`.

## Reproduction and result

All three original failures were reproduced in Google Chrome 153.0.8010.36 on
Linux 7.2.5-200.fc44.x86_64 against the production frontend at baseline commit
`3489d752b0b010418619b8457b1491ec56aa4363`:

| Case | Before | After |
| --- | --- | --- |
| Profile keyboard focus, 320×600 | Second card bottom 599.92px overlapped navigation beginning 534px | Native Tab/Shift+Tab reaches all three cards with visible focus, unobscured centers and at least 4px clearance |
| Shared back control |40×40px |44×44px, with matching header column |
| Small section-count contrast |4.4705:1 |5.5627:1 using existing secondary color #5b655f on #f4f6f2 |

The private shell uses root scroll padding for browser-managed focus scrolling.
Mobile bottom clearance includes the safe-area inset. At 700px the spacing changes
for the left navigation rail. Public pages keep their ordinary scrolling; no
JavaScript focus listener, router state or scoring behavior changed.

## Validation

- Full frontend ladder: 751 unit/component tests across 116 files, typecheck, lint
  and production build passed; strict browser-test TypeScript passed.
- Focused Chrome cases:10 passed, including forward/reverse traversal at 320, 390,
  699, 700 and 1280px, 600px-high viewports, score controls, 44px targets and contrast.
- A Chrome DevTools 24px bottom safe-area override verifies 100px scroll padding
  and visible focused cards, reset to 8px at the desktop breakpoint and `auto`
  after sign-out. This is synthetic inset evidence, not a physical-device test.
- Complete affected Chrome route suite: 52/52 passed. Existing scenarios
  cover loading/error/empty/long content, management, score queues, match notes,
  identity changes, public routes, privacy and page returns.
- Independent read-only review found no production-diff issues. Its initial test
  concern was addressed: the keyboard test now proves all three fixture cards
  receive focus in both directions instead of checking a fixed number of Tabs.

Commands from the repository root: `npm --prefix frontend run test`, `typecheck`,
`lint`, and `build`; `frontend/node_modules/.bin/tsc -p frontend/tsconfig.browser.json --noEmit`.
From `frontend/`: `npx playwright test --config playwright.routes.config.ts`.
The focused subset appends `navigationAccessibility.browser.ts`.

Evidence: [mobile keyboard](profile-keyboard-320.png),
[desktop keyboard](profile-keyboard-1280.png),
[simulated safe area](profile-safe-area-320.png). Screenshots were visually
inspected. Browser fixtures monitor console, request failures and response
statuses; these browser checks use controlled API data and real Chrome rendering.
Local detailed logs are `/tmp/golf-focus-baseline.log`,
`/tmp/golf-focus-final-focused.log` and `/tmp/golf-focus-full-browser.log`.

## Remaining limits

Native 200% browser zoom and physical Android remain unverified. The preceding
assessment's headless Control++ attempt did not change zoom; no Xvfb, xdotool or
adb is installed in this environment. Safe-area emulation is not soft-keyboard or
physical-device coverage. Backend/PostgreSQL ladders and Caddy/HTTPS were not
rerun because this change only affects shared CSS and browser regressions.

**READY for this bounded accessibility repair.** Overall deployment sign-off
remains **NOT READY** pending the separate security assessment, deployment/recovery
rehearsal and remaining browser/device gates. No deployment was performed.
