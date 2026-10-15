# A1x — retained leaderboard browser-test maintenance — 2026-10-15

This restores the retained opt-in live leaderboard test to current score delivery
behavior. Baseline: `4f4da790ce638f942f21aa95bbe7af9887d5f37d`. Only browser-test
code and evidence/docs change. No application UI/API/SQL, schema, dependency,
scoring policy or synchronization implementation changes.

## Reproduction and repair

The original complete-suite attempt stops at the first stale status assertion,
line 71: expected `Synkronisert` or `Lagret`, received `Lagret på serveren` in
the idle server-value state. Its later scenarios did not run in that baseline.
[Baseline receipt](baseline.json) and [exact sanitized assertion](baseline-assertion.json)
preserve the one failed case and observed label. The second obsolete assertion
uses the same old regex. Both now accept only `Synkronisert` or
`Lagret på serveren`; local saving, queued, verifying, refreshing and failed
labels do not satisfy the acknowledgement assertion.

Read-only source review additionally identified a stale failed-save fixture:
it intercepts the retired unconditional score endpoint and expects an older
error/discard UI. Current scoring uses conditional PUT and durable local delivery.
This is test-harness drift, not an observed product defect. The necessary test
support now:

- Intercepts the exact conditional score PUT, waits for an actual 503 response
  and requires at least one delivery attempt. The route remains installed while
  automatic retries are possible.
- Requires one pending entry with the current failed-delivery status and rejects
  a server-acknowledged score label. Server score and tournament JSON must remain
  unchanged, including the rendered standings.
- Uses the actual two-step local discard controls, requires an empty queue and
  restored acknowledged server value, then holds the 503 gate for 2200ms across
  one current two-second queue heartbeat. Attempts, server score and standings
  must remain unchanged before removing the fault.

That final interval is a bounded late-retry control, not proof against an
arbitrarily delayed retry. Ordinary subsequent scoring still has to pass the
existing live-update and authoritative-result assertions. No IndexedDB edits,
mocked successful scores or weaker pending/local-save acceptance are introduced.

## Complete browser acceptance

The repaired complete case passes once, with no skipped/flaky/retried case:
separate same-account desktop scorer and mobile Chrome context; team scramble,
individual stroke play and foursomes gross/net updates; lower-open-round and
best-N exclusions; historical correction; locked-score 409; real API crash,
offline projection clearing and reconnect to an offline-committed change;
member-only front-nine final projection, release and re-hide.

The exact two-GET refresh assertions remain. The test's final page/console/network
failure list is empty under its explicit session-401, injected conditional-503
and planned-outage allowances. Native EventSource uses the owned API directly
to avoid Vite's dev-proxy stream retention during a real upstream crash.
This bypass is test transport wiring, not a production proxy change.

[Candidate receipt](candidate.json), [scenario measurements](candidate-measurements.json)
and [final postconditions](final-postconditions.json) record acceptance. One
aggregate Playwright case executes the entire sequence in 20.52s; this is
validation timing, not a production performance benchmark.

The existing layout helper captures seven states at 320, 390 and 1280px and checks
horizontal document width each time. All 21 screenshots are fresh for this run;
[paths, dimensions and SHA-256](screenshots.json) are retained. Mobile populated
390px, desktop failed-delivery 1280px and member re-hidden 320px screenshots were
visually inspected. This is one combined desktop/mobile scenario with per-state
viewport checks, not three independently repeated full runs or a new layout audit.

## Validation and review

| Check | Result |
| --- | --- |
| Original label assertion | Exact expected failure reproduced |
| Complete repaired browser case | One passed; zero skipped/flaky/unexpected |
| Frontend Vitest | 1,133 tests in 147 files passed |
| TypeScript / ESLint / production frontend build | Passed |
| Locked debug API build / fresh fixture migrate / seed | Passed for both phases |
| Owned PostgreSQL/API/Vite/browser cleanup | Completed; API/Vite ports free |
| Temporary database credentials | Removed |

Independent read-only [test review](review.md) found no material issues in the
acknowledgement or negative-control repair. Final evidence/tooling consistency
review is recorded separately. Backend/PostgreSQL test ladders are not repeated:
application backend, migrations, API contracts and dependencies are unchanged.
The real browser uses freshly migrated/seeded PostgreSQL 17.10 Alpine and the
current locked API build; it does not use mocked successful API results.

## Reproduction and limits

```bash
python3 docs/validation/leaderboard-browser-maintenance-2026-10-15/run.py candidate
```

Requires cached `postgres:17-alpine`, rootless Podman, repository dependencies,
installed Chrome and free loopback ports 3000/5173. The runner reuses the
[CID-owned fixture helper](../sqlx-verify-ca-2026-10-15/fixtures.py), creates a fresh
random-credential database with tmpfs storage, builds/migrates/seeds it and owns
Vite plus the Playwright process group. The test owns only its API child and
deliberately restarts it. Existing services are not stopped or reused. Phase
receipts are cleared before setup so failed setup cannot retain an older success.

Baseline reproduction copies this runner into the pinned checkout and selects
`baseline` before repairing the test. Raw Playwright reports/diagnostic logs stay
local in `/tmp`, mode 0600; tracked receipts contain sanitized outcomes and
synthetic request paths. Local screenshots remain in `/tmp`. Fresh development
seed accounts/data are disposable; never target retained or hosted data.

This confirms the retained scenario against local headless Chrome 155 with
Playwright 1.63.0, Node 22.22.2 and npm 10.9.7. It does not establish physical
iPhone/Safari behavior, every network failure/timer interleaving, Caddy/public
TLS, production capacity or general security sign-off. No new product defect
was established by this maintenance step.
