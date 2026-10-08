# Fantasy release acceptance — 2026-10-08

FANTASY-6 verifies the private Fantasy game locally against schema 37. Production
behavior, schema and deployment configuration are unchanged in this step. Three
new persisted Chrome scenarios extend the existing three; test-fixture repairs
and this report complete the approved implementation plan.

## Environment and validation

Validation used PostgreSQL 17 in the disposable rootless Podman container
`golf-fantasy-f3-pg`, a new `fantasy_release` database on port 55445, the API on
3000, Vite on 5173 and installed Google Chrome through Playwright. All identities,
claims and scores are local fixtures. No messages or claim links were sent.

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `cargo test --workspace --all-targets` | 249 passed |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Passed; existing vendored SQLx warnings remain |
| Database-enabled full suite, command below | 712 passed; 3 explicitly ignored performance probes |
| Focused `fantasy_runtime` after helper repair | 2 passed |
| Fresh owner migration and development seed twice | Passed, schema 37 |
| Production runtime initialization and grant refresh scripts | Passed against the disposable database |
| Actual restricted runtime login | Migration-history SELECT allowed; INSERT/UPDATE/DELETE and schema CREATE denied |
| `/api/health`, `/api/ready` | Passed using that runtime login |
| `npm run test` | 133 files, 977 tests passed |
| `npm run typecheck`, `npm run lint`, `npm run build` | Passed |
| Strict standalone e2e TypeScript and focused ESLint | Passed |
| Six real-Chrome scenarios, command below | Passed |

The three ignored probes are `twelve_matches`, `twenty_four_matches` and
`hundred_matches`: they require exclusive disposable PostgreSQL with
`pg_stat_statements`. They are unrelated match-list performance measurements,
not skipped Fantasy correctness tests. Initial default backend testing could
not bind local mock-provider sockets inside the sandbox; the permitted rerun
passed. The database ladder used four test threads to bound local resource use.

The existing `deploy/postgres/10-create-runtime-role.sh` and
`deploy/postgres/refresh-runtime-grants.sh` ran against this new database. Only
the refresh script's `--host=postgres` was substituted with `--host=127.0.0.1`
inside the local container. Chrome used the API authenticated as the created
`fantasy_release_app` database role, with `RUN_MIGRATIONS=false`.

Reproduce against an isolated database using its owner connection for these
commands; do not run the seed or browser fixtures against production:

```bash
DATABASE_URL="$FANTASY_TEST_DATABASE_URL" cargo test --workspace --all-targets --features database-tests -- --test-threads=4
DATABASE_URL="$FANTASY_TEST_DATABASE_URL" cargo run -p golf-api --bin migrate
DATABASE_URL="$FANTASY_TEST_DATABASE_URL" cargo run -p golf-api --bin seed
DATABASE_URL="$FANTASY_TEST_DATABASE_URL" cargo run -p golf-api --bin seed
```

Start the matching local API and Vite, then from `frontend/` run:

```bash
GOLF_FANTASY_BROWSER=1 npx playwright test --config playwright.lifecycle.config.ts fantasy.browser.ts fantasyPrivacy.browser.ts fantasyRelease.browser.ts fantasyReleaseFormats.browser.ts
npx eslint e2e/fantasy.browser.ts e2e/fantasyReleaseSupport.ts e2e/fantasyRelease.browser.ts e2e/fantasyReleaseFormats.browser.ts
npx tsc --noEmit --strict --noUncheckedIndexedAccess --noUnusedLocals --noUnusedParameters --target ES2022 --module ESNext --moduleResolution Bundler --skipLibCheck --lib ES2022,DOM --types vite/client,node e2e/fantasy.browser.ts e2e/fantasyReleaseSupport.ts e2e/fantasyRelease.browser.ts e2e/fantasyReleaseFormats.browser.ts
```

## Acceptance matrix

Paths below are relative to the repository. The full database/frontend reruns
include the previously implemented regression tests; these are current execution
results, not coverage inferred solely from source review.

| Behavior | Persisted/API and browser evidence |
| --- | --- |
| Individual stroke play, net snapshots, non-finish, source correction | `backend/tests/fantasy_results.rs`, `fantasy_sources.rs`; existing `frontend/e2e/fantasy.browser.ts` |
| Scramble and foursomes, distinct frozen handicaps, both partners | `fantasy_result_formats.rs`; new `fantasyRelease.browser.ts` |
| Stableford native placing, uncapped net penalties and physical ace distinction | `fantasy_result_formats.rs`; new `fantasyReleaseFormats.browser.ts` |
| Four-ball both-partner attribution, pending partner and non-counting ace | `fantasy_result_formats.rs`, `fantasy_formats.rs`; new four-ball/match Chrome scenario |
| Match win/draw/loss, early finish, negative captain, no hole awards | `fantasy_result_matches.rs`; new four-ball/match Chrome scenario |
| Nine teams / eighteen golfers, placement once per team | `fantasy_result_teams.rs`; new three-round Chrome scenario |
| Smaller fields and twenty-team field | Four-golfer Chrome fixtures; `fantasy_result_teams::larger_field_and_closure_read_cost`; pure field-size rules |
| Changed partners/picks/captain, two carry generations, missed zero, unselected golfer totals | New three-round Chrome scenario; `fantasy_result_teams.rs` |
| Invalid carry, valid current lineup precedence, deadline eligibility, no late backfill, claims | `fantasy_selections.rs`, `fantasy_membership.rs`; pure rules and frontend tests |
| Opening/deadline/revocation races and direct integrity constraints | `fantasy_races.rs` |
| Hidden final JSON noninterference, release/re-hide, return, logout, draft privacy | `fantasy_results.rs`, `fantasy_result_matches.rs`; `fantasyPrivacy.browser.ts` |
| Loading, 503 retry, 403 clearing, saved/empty/populated states | Existing `fantasy.browser.ts`; focused frontend tests cover uncertain replay and held callbacks |
| Source fingerprint compatibility and populated schema-34 to 37 upgrade | `fantasy_result_contract.rs`, `fantasy_runtime.rs`; repeated migration/seed and restricted-role operations |

All six supported formats have API/Chrome coverage across the combined fixtures.
This is not every format crossed with every field size or lifecycle condition.
Invalid carry is covered below the browser layer; the new browser fixture covers
missed selections. Twenty-team coverage is PostgreSQL-only.

## Independent expected totals

The nine-team fixture has eighteen golfers and **three rounds**: scramble,
foursomes, scramble. The fixture explicitly supplies new pairings each round.
Each team's first-hole gross score is `4 + team index`; the other holes are par.
Nine team placements award `10, 8, 6, 5, 4, 3, 2, 1, 0`, and the resulting base
points per partner are `10, 7, 4, 2, -1, -2, -3, -4, -5`.

- The manager changing picks/captain scores `44 + 14 + 19 = 77`.
- The manager reusing the first lineup through both subsequent rounds scores
  `44 + 41 + 38 = 123`, with the immediately preceding source round recorded.
- The enrolled manager without any submitted lineup scores zero each round.
- Golfer examples total 30, 24 and −15. The last golfer is never selected but
  still appears in the eighteen-player overall points board.

The four-player Stableford fixture uses handicap 36, par-three holes and two
received strokes per hole. Gross 3/net 1 earns eagle +3; a physical gross-one ace
earns +10. Gross 9/net 7 and a pickup each earn −5. Other holes are net par.
Each golfer receives 3 hole points plus 10 tied-first placement points: **13**.
Four picks plus the extra captain contribution give **65**.

The four-ball fixture gives both partners **11** for the winning side and **8**
for the second side, producing manager **49**. Next round the captain concedes
after one hole: the winner gets +3, captain −1 doubled to −2, and the other match
is a draw worth +1 each. The match round totals **3**, overall **52**. The numeric
ace recorded in the match contributes no additional Fantasy points.

## Browser inspection and test repairs

Rendered-state assertions and screenshots cover widths 320, 390 and 1280,
including long names, manager/golfer details, no horizontal overflow and at least
44-pixel button/select heights. They do not replay the entire interaction sequence
independently at each width. Representative mobile manager and desktop Stableford
screenshots were visually inspected for readable wrapping and complete details.
Runs generate `/tmp/golf-fantasy-*.png`; screenshots are temporary artifacts.

Three test-only issues were addressed during acceptance:

1. A setup helper's 100 ms deadline expired under full-suite contention. It now
   uses the database clock, a one-second setup window and at most three attempts,
   retrying only an invalid deadline that has actually elapsed. Authorization,
   conflict and database errors still fail; dedicated expiry-race tests are unchanged.
2. The new match fixture assumed response order matched request order. Match lists
   sort by UUID; the fixture now selects and asserts opponent identities.
3. The existing browser response recorder discarded body-read errors and could
   reject before its final await. It now captures errors immediately and checks
   status, GET method and result path before inspecting completion/JSON. Explicit
   `net::ERR_ABORTED` result reads with HTTP 409 are counted separately from
   decoded `fantasy_conflict` responses; malformed completed responses still fail.
   The original discarded error cannot establish whether that run was canceled.

Read-only review independently checked fixture arithmetic, identity selection,
the deadline helper and evidence boundaries. Production source remains unchanged.
The browser checks verify result-read conflict recovery; controlled 503/403
responses belong to the explicit error-state scenario. They do not treat arbitrary
HTTP errors as acceptable.

The final combined run passed all six scenarios in 38.3 seconds. The existing
selection/correction scenario recorded three decoded `409 fantasy_conflict`
result reads, recovered to a successful fresh result and had no remaining alert.
It recorded zero canceled 409 bodies. The three new scenarios recorded no
conflicts, console errors or failed requests.

## Release limits

This is local functional acceptance, not a concurrency/load capacity benchmark.
No production deployment, public DNS/TLS test, fresh backup/restore rehearsal,
physical Android test or native 200% browser zoom check ran. Existing vendored
SQLx diagnostics and the separate transaction-cancellation investigation are not
resolved by these passing Fantasy tests. Hosted release and device checks remain
with the operator using the [deployment guide](../../deployment_guide.md) and
[testing checklist](../../testing_checklist.md).
