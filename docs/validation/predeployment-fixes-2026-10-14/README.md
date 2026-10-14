# Pre-deployment fixes — 2026-10-14

This sequence addresses R1–R3 from the
[assessment](../predeployment-review-2026-10-14/README.md). The user authorized all
three fixes together; each is bounded, validated and published separately.
Hosting and physical iPhone acceptance remain operator-owned.

## R1: precise browser error evidence

The preset recovery test no longer treats every error response as its injected
503. Its observer permits only the expected GET endpoints/status/error-code
combinations. A body that cannot be read is accepted only when Playwright records
an explicit canceled GET; unexpected body errors, codes, statuses, writes and
other transport failures stay fatal. Body inspection promises are collected and
awaited, including those added while earlier checks finish.

The retained scenario verifies both overall and round boards after retry, along
with its existing loading, error, closed-destination and draft assertions. A
separate scenario waits for initial live/read stabilization, injects a conflict
on each projection endpoint, requires readable `fantasy_conflict` evidence for
both, and verifies both boards finish refreshing and render again. Pure tests
also reject an unrelated 409 and unread responses without cancellation evidence.
No application behavior, production retry policy or scoring rule changed.

Validation passed: 1,102 tests across 142 files; typecheck, lint, strict browser
TypeScript and production build; 20/20 repeated Chrome cases (ten original
recovery plus ten delivered-conflict cases); two retained copy/uncertain-response
cases. Independent read-only review has no remaining findings. The final logs
are `/tmp/golf-r1-final-{tests,build,browser}.log` and
`/tmp/golf-r1-retained.log`. Commands from `frontend/`:

```bash
npm run test
npm run typecheck
npm run lint
npx tsc -p tsconfig.browser.json --noEmit
npm run build
GOLF_FANTASY_BROWSER=1 npx playwright test --config playwright.lifecycle.config.ts fantasyPreset.browser.ts --grep 'overview recovers|both projection' --repeat-each=10 --timeout=45000 --output=/tmp/golf-r1-final-browser
GOLF_FANTASY_BROWSER=1 npx playwright test --config playwright.lifecycle.config.ts fantasyPreset.browser.ts --grep 'presets three|pauses partial' --output=/tmp/golf-r1-retained
```

Local API/Vite use an owned schema-39 PostgreSQL 17 fixture container
`golf-fixes-20261014-pg` on port 55449. Migration and development seed passed.
All accounts/results are synthetic. The backend was unchanged for R1, so its
complete ladder was not repeated; the assessment records its preceding green run.

An intermediate observer waited for `response.finished()` before reading bodies;
a superseded repetition run stalled and was stopped. The final observer reads
JSON directly, then checks actual request cancellation if reading fails. The
independent review also required proof that injected conflicts were delivered,
which is now asserted per endpoint. Intermediate runs are not acceptance evidence.
