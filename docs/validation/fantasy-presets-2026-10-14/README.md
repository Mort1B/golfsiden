# Fantasy all-round presets — 2026-10-14

FANTASY-PRESET adds the all-round overview, explicit copy preparation,
independent overrides and ordered saves. Scope is the frontend Fantasy workspace
and documentation. API contracts, schema 37, sporting results and Fantasy scoring
are unchanged. Optional golf awards and scoring changes remain queued.

## Environment and checks

Local validation used disposable PostgreSQL 17 in rootless Podman container
`golf-preset-20261014-pg`, database `fantasy_preset` on 127.0.0.1:55446, the existing
API on port 3000, Vite on 5173 and installed Chrome through Playwright. The API was
built locally, applied existing migrations at startup, and returned healthy/ready.
All accounts, claims, lineups and scores were synthetic local fixtures.

| Check | Result |
| --- | --- |
| Initial new copy regression against the old UI | Failed on missing copy control, as expected |
| `npm run test` | 138 files / 1,052 tests passed; 16 new preset tests |
| `npm run typecheck` | Passed |
| `npm run lint` | Passed |
| `npm run build` | Passed |
| Strict standalone e2e TypeScript below | Passed |
| New preset Chrome suite | 3 passed, 44.9 seconds |
| Retained Fantasy/reliability Chrome suites | 14 passed, 1.4 minutes |
| Independent read-only review | No confirmed correctness, privacy or invariant findings |
| Diff whitespace, local documentation links and production-file sizes | Passed |

From `frontend/`, with the isolated API/Vite running:

```bash
npm run test
npm run typecheck
npm run lint
npm run build
npx tsc --noEmit --strict --noUncheckedIndexedAccess --noUnusedLocals --noUnusedParameters --target ES2022 --module ESNext --moduleResolution Bundler --skipLibCheck --lib ES2022,DOM --types vite/client,node e2e/fantasyPreset.browser.ts
GOLF_FANTASY_BROWSER=1 npx playwright test --config playwright.lifecycle.config.ts fantasyPreset.browser.ts
GOLF_FANTASY_BROWSER=1 GOLF_RELIABILITY_BROWSER=1 npx playwright test --config playwright.lifecycle.config.ts fantasy.browser.ts fantasyPrivacy.browser.ts fantasyRelease.browser.ts fantasyReleaseFormats.browser.ts reliability.browser.ts reliabilityFollowup.browser.ts
```

## Coverage and observations

- New component regressions cover destination revisions, ordered saves, captain
  overrides, exact lost-response replay, acknowledged refresh failure, changed
  closure/eligibility/field size/revision before the next write, replacement
  consent and stale previews, privacy erasure, account/CSRF replacement, and an
  advisory device clock. Additional cases assert initially closed destinations,
  a newly added round remaining unprepared, per-round discard, explicit stale
  draft rebase, offline blocking and repeated clicks during a held write.
- New Chrome fixtures prepare three rounds before sporting team assignment,
  override one captain, inspect each accepted receipt and verify a second manager
  cannot read private picks. Keyboard Enter/Escape verifies preview focus and
  return to its trigger.
- The lost-response browser case accepts round two on the server then aborts its
  response. Observed write order is round 1, round 2, exact round-2 replay, round 3;
  no confirmed round is sent again. Only the deliberately aborted save fails.
- A separate browser case holds an unselected round read, returns an injected
  503, retries successfully and excludes an actually expired destination from
  copying. Only injected 503 responses occur in that case. It displays
  unprepared, loading, read-error and closed states.
- Retained tests exercise carry-forward across changing teams, scoring formats,
  hidden results, pre-lock privacy, denied-access clearing, parent unmounts,
  refresh-only recovery, competing sessions, navigation and player-claim recovery.
  Existing bounded result-read conflict recovery remains exercised.
- Screenshots and layout assertions cover 320, 390 and 1280 pixels for reviewed,
  saved, uncertain, loading and read-error states. They assert no horizontal
  overflow and at least 44-pixel buttons/selects; they do not repeat the entire
  interaction sequence independently at every width. Representative mobile
  review/error and desktop saved/uncertain screenshots were visually inspected.
  Temporary screenshots are `/tmp/golf-fantasy-preset-{state}-{width}.png`.
  No unexpected console/page errors occurred in the new scenarios.

The first happy-path browser run paused when the initial SSE connection opened
during saving. That connection event deliberately clears private projections;
the batch must stop. Successful multi-save fixtures now await the actual live
connection event, without weakening production privacy behavior. A held-write
component regression separately proves a real disconnect signal pauses later
writes while retaining acknowledgement. The extra repeated-click test initially
looked for the pre-dispatch ready count; it now asserts the disabled save control
while a round is unresolved. Both repairs concern test expectations/coordination.

## Limits

Backend and PostgreSQL full ladders were not rerun because no backend, migration,
API or scoring source changed; current database-backed behavior was exercised
through Chrome. This is functional local acceptance, not a load test or hosted
deployment. Initial connection/reconnect can intentionally pause a batch and
requires deliberate review/resume. Drafts remain memory-only and cannot survive
browser closure. Public DNS/TLS, physical Android and native 200% zoom were not
tested. Hosting stays with the operator.
