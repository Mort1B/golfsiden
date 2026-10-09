# Reliability verification — 2026-10-09

This bounded frontend repair covers Fantasy draft/submission lifetime, write versus
refresh outcomes, and committed player registration with lost response delivery.
Backend routes, scoring, SQL, authorization and single-use claims are unchanged.

## Verification before implementation

The worktree started clean on `main` at `fe20058`.

- `FantasyPage` branches on game/membership/roster query state; `FantasyRound`
  branches on round reads. Both can unmount `MyFour`, whose draft and uncertain
  request were component-local. Parent regressions reproduced loss after the
  actual `handleTournamentLiveSignal(error/open)` clearing/reconnect path.
- `useFantasyAction` put write, success callback and invalidation into one catch
  path and treated resolving invalidation as a successful display refresh. A
  parent save followed by a rejected round read lost acknowledgement feedback.
- Claim registration commits the account, preserved player link, consumed grant
  and session before the HTTP handler supplies cookie/body. `ClaimExperience`
  had no session reconciliation. Both cookie-present and cookie-absent parent
  tests failed to recover. The backend rejects reissuing a linked player's claim;
  generic advice to request another link was therefore wrong for this case.

The initial two-file reproduction run had **5 failed / 11 passed** tests before
production edits: draft retention, uncertain-request retention, failed refetch
following acknowledgement, and the two lost registration response branches.
A later review found activation could mount new queries after invalidation's
initial snapshot; its new parent regression failed before the final repair.

## Implemented boundaries and regressions

| Behavior | Evidence |
| --- | --- |
| Query clearing/errors cannot erase local selections or exact uncertain request | `FantasyPage.reliability.test.tsx`, actual `FantasyPage` → `FantasyRound` → `MyFour` with query client and live invalidation |
| Round switching during held save preserves original read/replay target | Parent test plus browser round-switch retention; recovery state contains identifiers/input/own receipts only |
| Canonical account, logout and CSRF changes reset recovery | Parent tests; old mutation callbacks remain fenced by canonical user/CSRF and mounted lifetime |
| Navigation cannot silently discard unresolved input | Parent router test; explicit unsent-input discard releases the guard; uncertain requests cannot be discarded as unsent |
| Acknowledgement survives parent unmount and failed refetch | Parent tests for lineup and admin configuration; read-only retry does not increase write count |
| Activation checks newly enabled reads | Parent activation → failing round/results → read-only retry regression |
| Exact rejected replay resolves nonacceptance | Parent lost response → same request → `fantasy_closed` → explicit draft discard → successful navigation; backend receipt lookup precedes closed-window rejection |
| Claim recovery accepts only prepared player/chosen username | `ClaimPage` parent tests for cookie present/absent, mismatched identity and failed session read |
| Claim recovery cannot overwrite concurrent login | Held recovery and held ordinary-registration tests, including unmount |
| No automatic/offline Fantasy writes | Existing MyFour offline regression and unchanged explicit online-only dispatch; no browser-storage queue added |

Focused parent coverage is 13 Fantasy page tests and 17 claim page tests. The
existing MyFour, settlement, query, API and broader frontend tests also ran.

## Executed checks

| Check | Result |
| --- | --- |
| `npm run test` | **134 files, 996 tests passed** |
| `npm run typecheck` | Passed |
| `npm run lint` | Passed |
| `npm run build` | Passed |
| Strict standalone TypeScript and ESLint for changed e2e files | Passed |
| `cargo build -p golf-api --bin golf-api` | Passed; existing vendored SQLx warnings |
| Existing migrations applied to fresh disposable database | Passed, schema 37 |
| New Chrome reliability suite | **3 passed** |
| Existing Fantasy/release/privacy and player-claim Chrome suites | **9 passed** |
| Combined final Chrome run | **12 passed, 1.1 minutes** |
| Read-only lifecycle/privacy review and `git diff --check` | Resolved review findings; whitespace clean |

Commands run from `frontend/` (browser fixtures must use a disposable database):

```bash
npm run test
npm run typecheck
npm run lint
npm run build
npx tsc --noEmit --strict --noUncheckedIndexedAccess --noUnusedLocals --noUnusedParameters --target ES2022 --module ESNext --moduleResolution Bundler --skipLibCheck --lib ES2022,DOM --types vite/client,node e2e/reliability.browser.ts e2e/fantasy.browser.ts
npx eslint e2e/reliability.browser.ts e2e/fantasy.browser.ts
GOLF_RELIABILITY_BROWSER=1 GOLF_FANTASY_BROWSER=1 GOLF_PLAYER_CLAIMS_BROWSER=1 npx playwright test --config playwright.lifecycle.config.ts reliability.browser.ts fantasy.browser.ts fantasyPrivacy.browser.ts fantasyRelease.browser.ts fantasyReleaseFormats.browser.ts playerClaims.browser.ts
```

## Browser environment and limits

The local API used the fresh `reliability_20261009` database in the existing
rootless PostgreSQL 17 test container `golf-fantasy-f3-pg` (port 55445). Vite ran on
5173 and the local API on 3000. Installed desktop Google Chrome ran through
Playwright at 320, 390 and 1280 CSS-pixel widths. Fixture names are synthetic;
no messages or real claim links were sent. No hosted service was changed.

The new Fantasy scenario commits a real lineup, deliberately loses its response,
clears/reopens the actual frontend live subscription, and verifies identical request ID, revision, ordered picks and captain on explicit
replay. It then commits revision 2, injects a 503
round read, verifies retained acknowledgement, and retries only reads. Three
lineup writes total are asserted: initial request, exact replay, next revision.
The SSE boundary is deliberately driven by browser-dispatched `error/open`
events on a captured EventSource after closing it; this is not a claim of a
physical network-loss/reconnect timing test.

Each registration scenario commits through a separate real API request context.
The browser receives a malformed successful body, once with the real Set-Cookie
and once without it. With the cookie it recovers the exact prepared player;
without it, the test proves the browser is unauthenticated, follows prefilled
ordinary login, and reaches that same prepared player. Both assert exactly one
registration request and a consumed preview returning 410. Parent tests also
exercise rejected transport promises; Fantasy mutation tests cover 5xx response
classification.

Console/page errors and failed requests were checked, not only screenshots.
The new scenarios allow only injected read failures, the deliberate lost lineup
request, expected unauthenticated session reads, and canceled reads/live streams.
Existing suites additionally validate bounded `409 fantasy_conflict` reads and
intentional denied/missing/empty cases. They cover scoring/settlement, hidden
results, four-ball, Stableford, match outcomes, changing partners and carry-forward.

Representative visually inspected artifacts:

- [Failed refresh with retained receipt, 320px](fantasy-refresh-failed-320.png)
- [Uncertain exact submission, 1280px](fantasy-uncertain-1280.png)
- [Claim recovery without cookie, 320px](claim-no-cookie-320.png)

Draft/recovery retention is intentionally in-memory within one tournament/account/
session; it does not promise recovery after forced tab closure, browser eviction,
reload confirmation or session replacement. Browser unload prompts depend on the
browser. Physical phones, native 200% zoom, public-host DNS/TLS/deployment and
actual packet-loss timing remain unverified. The full Rust/database test ladders
and seed were not rerun: this step changes no backend source, migration or seed;
the current API build and real PostgreSQL-backed browser fixtures ran instead.
