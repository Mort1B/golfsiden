# Optional round awards — 2026-10-14

ROUND-AWARDS implements optional closest-to-pin and longest-drive adjustments,
each −1 on the winner's personal net golf result. Both may go to one winner.
Gross scores, hole inputs, handicap snapshots and Fantasy remain unchanged.
Stableford preserves native points and adds separate competition points.

## Environment and checks

Local acceptance used PostgreSQL 17 in owned rootless Podman container
`golf-awards-20261014-pg` on 127.0.0.1:55446, locally built API on 3000, Vite on
5173 and installed Chrome through Playwright. All accounts and sporting records
were synthetic. The runtime API used a restricted login created and granted
through the actual production initialization and permission scripts; only the
local database host was substituted. Schema CREATE and migration-history INSERT
were denied while award-table DML was granted. Health and readiness passed.

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `cargo test --workspace --all-targets` | 254 passed |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Passed |
| Full backend ladder with `--features database-tests -- --test-threads=4` | 725 passed, 3 existing performance probes ignored |
| `cargo build -p golf-api --bins` | Passed |
| Empty database through schema 38 | Passed |
| Populated schema 37 → 38 | Passed; preserved tournament/round/player/participant counts and row hashes |
| Development seed twice on clean schema 38 | Passed |
| `npm run test` | 140 files / 1,083 tests passed |
| `npm run typecheck`, `npm run lint`, `npm run build` | Passed |
| Strict standalone award-browser TypeScript | Passed |
| New awards Chrome scenarios | 4 passed |
| Retained Chrome scenarios | 12 passed |
| Chrome keyboard-only award submission at 390px | Passed; one persisted award |
| Whitespace, 105 local document links/anchors and affected production-file sizes | Passed |
| Independent backend/frontend read-only review | Findings resolved; no remaining confirmed functional defects |

Database migration and seed checks ran the built `migrate` and `seed` binaries
with disposable owner connections. Both clean/upgrade databases recorded all 38
migrations. The restricted runtime database was the clean seeded installation.
The three ignored PostgreSQL tests are existing exclusive `pg_stat_statements`
performance probes (`hundred_matches`, `twelve_matches`, `twenty_four_matches`),
not award acceptance tests. Initial sandbox socket permission failures required
running the full ladder with local-network permission. Vendored SQLx warnings
remain; no new strict-lint warnings were accepted.

From `frontend/`, against the disposable API/Vite:

```bash
npm run test
npm run typecheck
npm run lint
npm run build
npx tsc --noEmit --strict --noUncheckedIndexedAccess --noUnusedLocals --noUnusedParameters --target ES2022 --module ESNext --moduleResolution Bundler --skipLibCheck --lib ES2022,DOM --types vite/client,node e2e/roundAwards.browser.ts
GOLF_AWARDS_BROWSER=1 npx playwright test --config playwright.lifecycle.config.ts roundAwards.browser.ts
GOLF_LIFECYCLE_BROWSER=1 GOLF_STABLEFORD_BROWSER=1 GOLF_RESULT_SHARING_BROWSER=1 GOLF_FANTASY_BROWSER=1 npx playwright test --config playwright.lifecycle.config.ts roundLifecycle.browser.ts stableford.browser.ts resultSharing.browser.ts fantasyReleaseFormats.browser.ts fantasyPreset.browser.ts
```

## Coverage and review

- Domain tests cover absent/individual/both awards, different winners, team
  partner isolation, Stableford native versus competition values, counted-round
  selection and final-round ties. Hidden finals remain excluded.
- PostgreSQL/API tests cover exact admin/session identity, distinct-admin
  contention, stale revisions, exact replay, conflicting request identity,
  required-nullable removal, correction acknowledgment, preserved-field foreign
  keys, direct audit/current guards and one post-commit event. Gross JSON remains
  identical. Fantasy result JSON and source tokens remain identical even when an
  award breaks a sporting tie. Public adjusted results retain their allowlist.
- Frontend tests cover strict award/result decoding, BIGINT revision strings,
  receipt identity/body/actor mismatches, privacy clearing, session replacement,
  stale draft review, offline and duplicate-click blocking, exact replay after
  failed reads/unmount, and read-only recovery after acknowledgment. Replay HTTP
  400 retains original uncertainty. Optional award-read failure cannot disable
  lifecycle locking. Review identified and resolved three recovery/receipt gaps;
  regressions cover each.
- Real Chrome completes individual and team rounds with neither award, then
  applies both to one winner, replaces one and removes the other. It checks
  personal 72→70, unchanged teammate/shared team results, unchanged Fantasy,
  hidden-member result equality and admin-route denial. Revealed member history
  shows adjusted results without admin controls. A locked Stableford scenario
  verifies preserved native points and separate competition points.
- A real accepted PUT with a deliberately lost response is followed by failed
  GETs. Provider-level recovery replays exactly the original body while the
  editor is unavailable. A subsequent failed refresh retains acknowledgment;
  read-only retry restores the view without another PUT.
- Screenshots/layout assertions cover empty, populated, history, result and
  recovery/error states at 320, 390 and 1280 pixels, including long names. They
  check horizontal overflow and controls of at least 44 pixels. The interaction
  sequence is not independently repeated at every width. Retained lifecycle and
  Stableford scenarios also cover loading/error/empty states, visibility,
  scorecard recovery and public sharing. Fantasy presets and format regressions
  remain green. Representative mobile populated/recovery and desktop Stableford
  screenshots were visually inspected. Native select ArrowDown, Tab to reason,
  typing, Tab to save and Enter submitted exactly one award in Chrome.

The new browser listener checks page errors, non-resource console errors and
failed requests except live-stream traffic and `ERR_ABORTED`. Intentional 503
reads and the deliberately aborted PUT are verified by state assertions. Aborted
navigation requests are not classified individually, so this is not a claim that
every aborted network request was inspected. The locked Stableford fixture was
corrected to read the permitted scorecard endpoint after locking; production
behavior was unchanged by that fixture repair.

Temporary evidence includes `/tmp/awards-{backend-test,database-test,clippy}.log`,
`/tmp/awards-{clean,upgrade,seed-first,seed-second,runtime-grants}.log`,
`/tmp/awards-all-tests.log`, `/tmp/awards-browser-final.log`,
`/tmp/awards-retained-browser.log`, `/tmp/awards-browser-evidence.log`,
`/tmp/awards-keyboard.log` and `/tmp/golf-awards-*.png`.

## Limits

This is local functional acceptance, not hosted deployment, load testing or a new
backup/restore rehearsal. Public DNS/TLS, physical Android and native 200% zoom
were not tested. Hosting remains operator-owned. Award recovery is memory-only
and requires an online session. Match-play and unfinished-round awards are not
supported. The revised Fantasy scoring scale remains a separate queued step.
