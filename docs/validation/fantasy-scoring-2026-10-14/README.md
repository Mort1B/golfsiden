# Versioned Fantasy scoring — 2026-10-14

Scope: version 2 gives actual aces +20 and non-ace net albatross-or-better +5.
Version 1 keeps +10/+10. Every other category, placement, match outcome, captain
rule, sporting score and selection contract remains unchanged.

## Environment and checks

Local acceptance used owned PostgreSQL 17 container `golf-scoring-20261014-pg`
on 127.0.0.1:55446, API on 3000, Vite on 5173 and installed Chrome. All accounts,
scores and sessions were synthetic. Before building schema 39, the schema-38
binaries were preserved in `/tmp/fantasy-scoring-schema38`. The old API prepared
three games through real endpoints: completed/scored legacy, unlocked with saved
picks, and deadline-expired without materialization. Their browser sessions and
before-results were retained locally, not committed.

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `cargo test --workspace --all-targets` | 256 passed |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Passed |
| Full backend tests with `--features database-tests -- --test-threads=4` | 732 passed, 3 existing performance probes ignored |
| `cargo build -p golf-api --bins` | Passed |
| Empty database through schema 39 | Passed, 39 migrations |
| Populated schema 38 → 39 | Passed, 39 migrations; unchanged tournament/round/player/lineup row hashes |
| Development seed twice on schema 39 | Passed |
| Restricted runtime API health/readiness and upgrade reads | Passed |
| Focused frontend Fantasy suite | 95 passed |
| `npm run test` | 1,100 tests across 141 files passed |
| `npm run typecheck`, `npm run lint`, `npm run build` | Passed |
| Strict standalone browser TypeScript | Passed |
| New scoring Chrome scenarios | 3 passed, 54.2 seconds |
| Retained Fantasy/awards Chrome scenarios | 13 passed, 4.1 minutes |
| Independent read-only backend/frontend review | No remaining findings |
| Whitespace, 105 local documentation links/anchors and affected production-file sizes | Passed |

The three ignored PostgreSQL probes (`hundred_matches`, `twelve_matches`,
`twenty_four_matches`) require exclusive `pg_stat_statements` measurement and are
not scoring acceptance. Existing vendored SQLx warnings remain. Sandbox socket
restrictions required escalated local PostgreSQL/Chrome runs; those checks then
completed. A new migration test initially reused invalid/duplicate synthetic
user/session identities; its fixture was corrected before the successful full run.

The production runtime initialization/grant scripts were exercised on the owned
upgrade database, replacing only the local host address in the grant invocation.
The API ran with a distinct restricted login: schema CREATE and migration-history
INSERT were false, Fantasy SELECT was true, and health/readiness passed. The real
legacy game's complete parsed overall response remained identical, including its revision;
its ace/albatross holes stayed +10. The unlocked game's version became 2 and its
result revision changed. The expired but unmaterialized game stayed version 1.

## Commands and fixture boundary

Backend and PostgreSQL commands follow `docs/AGENT_WORKFLOW.md`, with the owned
local database URL. `target/debug/migrate` ran against `scoring_clean` and
`scoring_upgrade`; `target/debug/seed` ran twice against the clean database.

From `frontend/`, against the disposable API/Vite:

```bash
npm run test
npm run typecheck
npm run lint
npm run build
npx tsc --noEmit --strict --noUncheckedIndexedAccess --noUnusedLocals --noUnusedParameters --target ES2022 --module ESNext --moduleResolution Bundler --skipLibCheck --lib ES2022,DOM --types vite/client,node e2e/fantasyScoring.browser.ts e2e/fantasyReleaseFormats.browser.ts
GOLF_FANTASY_BROWSER=1 GOLF_FANTASY_LEGACY_FIXTURES=/tmp/scoring-browser-setup/fixtures.json npx playwright test --config playwright.lifecycle.config.ts fantasyScoring.browser.ts
GOLF_FANTASY_BROWSER=1 GOLF_AWARDS_BROWSER=1 npx playwright test --config playwright.lifecycle.config.ts fantasy.browser.ts fantasyPrivacy.browser.ts fantasyRelease.browser.ts fantasyReleaseFormats.browser.ts fantasyPreset.browser.ts roundAwards.browser.ts
```

The legacy browser case requires sessions and games prepared before migration;
without `GOLF_FANTASY_LEGACY_FIXTURES` it explicitly skips. The file is an array
with `kind` (`legacy`, `unlocked`, `expired`), Playwright `state` file path,
`tournament`, `round`, `base` Fantasy API path and pre-upgrade `before` overall
JSON. The local setup used `releaseFixture` on schema 38 with four players and
one team round, saved a four/captain, then opened/scored/completed only the legacy
game and set a short future deadline only for the expired game. The resulting
schema-38 database was cloned before applying schema 39. Do not weaken production
guards to manufacture legacy games after upgrade. Automated PostgreSQL tests
in `backend/tests/fantasy_scoring.rs` construct their own schema-38 fixtures and
remain self-contained.

## Coverage

- Pure tests dispatch both versions through every hole category, actual-ace
  precedence, net-one distinction, shared partners and captain multiplication.
  Balance fixtures show ace = four net albatrosses = 20 birdies, and ace exceeding
  five birdies plus first-place points. These are numerical consequences, not
  probability estimates. Existing four-ball and match-only tests remain green.
- Migration tests cover eligible saved/disabled/empty games, expired lazy windows,
  materialized locks and opened rounds; exact lineup replay; frozen result JSON
  and source-token preservation; every envelope's version/revision; default v2,
  forbidden explicit new v1, immutable versions and rejected unknown versions.
- A lock-wait test lets a deadline expire before migration obtains its locks.
  Another pauses a real authorized opening, starts migration, then commits the
  opening first. Both preserve version 1. Version-2 stroke/Stableford tests check
  hole, captain, round, overall and both breakdown totals.
- Frontend tests require 1/2 metadata on game and all result envelopes, reject
  wrong category awards and unknown/missing versions, and suppress mismatched
  overall/round/golfer/manager/admin results. Loading, failure and denial never
  invent rules. Disabled-game rules remain readable. Read-only mismatch recovery
  preserves the existing provider/session boundary.
- New Chrome scenarios display legacy +10 rules and unchanged standings/details;
  upgraded unlocked rules; expired legacy rules; v2 +20/+5 holes, golfer totals
  35/14/8/5 with ranks 1/2/3/4, captain contribution 70 and manager total 97.
  Held reads, injected HTTP 500 and mixed-version responses exercise loading,
  error and mismatch recovery. Keyboard refresh sends no writes.
- Thirty screenshots plus two cropped rule panels cover 320/390/1280 pixels,
  long names, manager/golfer details and both versions. Layout assertions check
  no horizontal overflow and controls at least 44 pixels. Rules and retry controls
  were keyboard-operated. Representative mobile legacy/error/mismatch and desktop
  v2 panels were visually inspected. Full interactions are not rerun at each width.
- Retained tests cover changing partners/captains, carry-forward, preset recovery,
  private selections, concealed results, Stableford/four-ball/matches and net-only
  round awards. One retryable `fantasy_conflict` occurred in the retained suite;
  it recovered, with no canceled 409 body read reported by that observer.

No unexpected console/page errors or non-aborted request failures occurred in
the new scenarios; HTTP 500 was deliberately injected in the recovery case.
Existing browser helpers exclude live-stream and `ERR_ABORTED` failures, so this
is not a claim that every navigation abort was individually classified.

Temporary logs are `/tmp/scoring-{backend-test,database-test,clippy,build}.log`,
`/tmp/scoring-{clean,upgrade,seed-first,seed-second,upgrade-api,runtime-check}.log`,
`/tmp/scoring-frontend-*.log`, `/tmp/scoring-browser.log` and
`/tmp/scoring-retained-browser.log`. Images are `/tmp/golf-fantasy-scoring-*.png`.

## Limits

This is local functional acceptance, not hosted deployment, load calibration or
a new backup/restore rehearsal. Public DNS/TLS, physical Android and native 200%
zoom remain unverified. Hosting is operator-owned. Deploy matching schema-39 API
and frontend builds. Existing legacy round/detail response hashes change because
version metadata is added; their scoring values and source tokens do not.
