# Friends deployment: functionality, design and Chrome assessment

Assessed application commit: `38e3eef43ecbdc9e6d00605aedcac8b248d07144`.
Date: 2026-09-23. Google Chrome 153.0.8010.36 on Linux
7.2.5-200.fc44.x86_64. Application source, scoring rules and schemas were unchanged.

## Verdict

**NOT READY for deployment sign-off.** The tested browser journeys provide broad
functional evidence, but the separate security assessment and a current deployment
restore exercise remain unresolved. Native 200% browser zoom and physical Android
Chrome were not verified. Three reproducible usability defects also remain below.
This assessment does not authorize deployment or claim those untested boundaries
are safe. There is no completed security report in the current documentation from
which to infer either a clean verdict or additional confirmed vulnerabilities.

## Confirmed findings

| ID | Severity | Scenario and reproduction | Expected / observed | Evidence |
| --- | --- | --- | --- | --- |
| UI-1 | Medium | Member on `/profile`, 320×600, with three tournament cards. Tab from the first card to the second. | Focused content should remain readable above fixed navigation. The second card spans y469.92–599.92 while navigation begins at y534; its title/role is partly covered and its center hit-tests to `NAV`. Manual scrolling can reveal it. | [Screenshot](profile-focus-covered.png), [focus measurements](focus-measurements.json) |
| UI-2 | Low | Administrator on `/manage/tournaments/<id>` at 320, 390, 699, 700 and 1280px. Measure `.back-button`. | Repository navigation target minimum is 44×44 CSS pixels; actual target is 40×40. | [Measurements](design-measurements.json), `frontend/src/styles.css` `.back-button` |
| UI-3 | Low | Member on `/tournaments/<id>` with populated sections. Inspect `.section-heading span` counts. | 12.8px text should meet 4.5:1 contrast. `#69746d` on `#f4f6f2` measures 4.4705:1. | [Contrast measurements](contrast-measurements.json), `frontend/src/styles.css` |

UI-1 is a failure of the repository's unobscured-content requirement. The evidence
shows partial coverage; it is not described as total focus obscuration under
WCAG 2.4.11. UI-3 uses the unrounded threshold described by
[W3C's contrast guidance](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html).
The 44px requirement for UI-2 comes from `frontend/AGENTS.md`.

Next bounded repair: shared navigation/focus clearance, the back target and these
section-count colors. Preserve layout language and all route/scoring behavior;
add focused browser regressions and rerun the affected frontend checks. Do not
combine this with new features or the separate security remediation queue.

## Tested topology and limits

- Fresh disposable PostgreSQL 17, migrated by the owner; distinct `golf_app`
  runtime login, runtime grants, and migration-table writes revoked. Seed-dependent
  suites used a separate newly created database for each file.
- Production Vite assets served by Caddy 2.10.2, retaining the repository's CSP,
  security headers, SPA fallback and immediate reverse-proxy flushing. Temporary
  configuration changed only local addresses/upstreams and certificate setup.
- Broad browser suites used local HTTP port 5173, API development mode and insecure
  development cookies because existing fixture helpers fix that origin.
- A separate HTTPS port 54443 used API production mode, `RUN_MIGRATIONS=false`,
  secure cookies, trusted proxy configuration and the restricted database role.
  The new `deploymentBoundary.browser.ts` requires secure/HttpOnly/SameSite cookie
  attributes, response security headers, a native `score` event through the same
  HTTPS origin, a persisted score and exact second-session total changing from
  5 to 9 gross with two holes registered. Reload retains that total.
- The HTTPS certificate is Caddy's disposable internal certificate. The explicitly
  enabled local-certificate test option ignores certificate trust errors. Public
  DNS, ACME, trusted certificates, production ports and full Compose image assembly
  are not established by this smoke test. The API binary was built from the
  assessed checkout. No production or existing security-review data was used.

## Acceptance evidence

| Boundary | Evidence and role coverage | Status |
| --- | --- | --- |
| Account → tournament | Profile updates/password invalidation, new-account and existing-account invitation joining, exact login return, account switching, denied/nonmember targets, administrator-assisted recovery | Passed final cases |
| Organizer setup → playable round | UI tournament creation; UI invitation issue and inline existing-account acceptance; saved-course selection with focus restoration; manual two-player team/flight assignment confirmed by API; start and explicit open confirmation | Passed |
| Score → results | Stroke/team, Stableford, four-ball and singles-match suites; persisted scores, confirmation, result/history views, gross/net labels; separate-session native HTTPS score event and exact result update | Passed final cases |
| Connectivity → authority | Durable offline queues, reload/replay, both conflict choices, unknown acknowledgement, blocked local persistence, logout isolation, locks, held responses and overlapping page returns | Passed final cases |
| Lifecycle → privacy | Round completion/locking/corrections; tournament completion/archive; hidden/released final projections; private 401/403/404 erasure and public capability rotation/revocation | Passed final cases |
| Layout → interaction | 320×600, 390×844, 699×900, 700×900, 1280×900; populated/long/empty/loading/error states, keyboard disclosures and focus, saved-course/admin/profile discovery; screenshots visually reviewed | Three findings; zoom/device limits below |

The standalone organizer journey was executed with disposable accounts through the
actual UI. API fixture setup in the format suites is not counted as UI setup
evidence. [Desktop management screenshot](management-desktop.png) and the retained
measurement JSON supplement state, request and persistence assertions.

## Validation commands and results

All browser commands below run from `frontend/` unless stated otherwise. The
`GOLF_*` opt-in flags were enabled for their matching suites; skipped cases are
not counted as passes. Browser groups overlap, so their counts must not be summed
into a claimed unique-test total.

| Command / group | Result |
| --- | --- |
| `npm --prefix frontend run test` (repository root) | 751 passed across 116 files |
| Frontend `typecheck`, `lint`, `build` | Passed |
| `tsc -p frontend/tsconfig.browser.json --noEmit` (root binary) | Passed |
| `npx playwright test --config playwright.routes.config.ts` | 42 passed; mocked API production-preview route/return/privacy evidence |
| `handicapSummary.browser.ts`, compatible preview harness | 2 passed |
| `tournamentTieBreak.browser.ts -g server-ranked`, compatible preview harness | 1 passed |
| Broad core Caddy matrix | 31/60 initially passed; one mock-CSP mismatch and 28 fixture failures after the local API process stopped. Completed reruns are listed below. |
| `offlineLifecycle`, `offlineScoring`, `stableford`, `stablefordDraft`, `stablefordOffline`, `tournamentContext` with matching flags | 26/26 passed on rerun; unchanged production code |
| Fresh-database seeded files | 22/22 final cases passed: 21 through Caddy; one mocked tie-presentation case through preview. Flight-progress 1/1 and round-lifecycle 2/2 reran on fresh databases after wording corrections. |
| `GOLF_DEPLOYMENT_BROWSER=1 GOLF_DEPLOYMENT_LOCAL_CERT=1 npx playwright test --config playwright.lifecycle.config.ts deploymentBoundary.browser.ts` | Final reviewed test: 1/1 passed |
| `node /tmp/golf-design-{audit,journey,contrast,focus}.mjs` (four separate diagnostic commands) | Completed; UI/measurement evidence and findings above |

Core flags: `GOLF_CONTEXT_BROWSER`, `GOLF_OFFLINE_BROWSER`,
`GOLF_STABLEFORD_BROWSER`, `GOLF_FOUR_BALL_BROWSER`, `GOLF_MATCH_BROWSER`,
`GOLF_L3_BROWSER`, `GOLF_M2_BROWSER`, `GOLF_H1_BROWSER`,
`GOLF_RECOVERY_BROWSER`, `GOLF_RESULT_SHARING_BROWSER`.
Core files additionally include `fourBall`, `fourBallVisibility`, `matchPlay`,
`matchPlayerHistory`, `leaderboardNavigation`, `privateResults`, `scoreRecovery`,
`passwordRecovery`, `resultSharing`, `resultSharingStates`, `handicapSummary`.

Seeded file/flag mapping: `coursePresets/GOLF_COURSE_PRESETS_BROWSER`,
`courseLayout/GOLF_COURSE_LAYOUT_BROWSER`, `organizerSummary/GOLF_ORGANIZER_BROWSER`,
`roundDetails/GOLF_ROUND_DETAILS_BROWSER`,
`tournamentCreation/GOLF_TOURNAMENT_CREATION_BROWSER`, `profile/GOLF_PROFILE_BROWSER`,
`passwordRecovery/GOLF_RECOVERY_BROWSER`, `tournamentTieBreak/GOLF_TIE_BREAK_BROWSER`,
`flightProgress/GOLF_FLIGHT_PROGRESS_BROWSER`, `roundLifecycle/GOLF_LIFECYCLE_BROWSER`,
`tournamentCompletion/GOLF_TOURNAMENT_COMPLETION_BROWSER`,
`tournamentArchive/GOLF_TOURNAMENT_ARCHIVE_BROWSER`,
`resultSharing/GOLF_RESULT_SHARING_BROWSER`. Each uses the lifecycle configuration
and its matching `*.browser.ts` filename with the flag set to `1`.

## Diagnostic failures and evidence boundaries

- The initial task-owned API process exited cleanly without a logged panic. Its
  cause was not established. Subsequent fixture requests returned 502, so those
  failures do not prove a frontend defect. The affected cases were rerun against
  a restarted process; this is not a claim that production process stability was
  exhaustively verified.
- Handicap and one tie-presentation test redirect EventSource to an artificial
  cross-origin local stream. Caddy correctly blocks that under `connect-src 'self'`.
  They passed in the compatible preview harness; production CSP was unchanged.
- Flight-progress assertions used obsolete `fullført` wording; the captured DOM
  already showed the correct counts with `scorekort med alle hull ført`. Lifecycle
  tests expected `Lagret`/`Synkronisert` instead of `Lagret på serveren`. Only those
  test expectations changed; hidden-state assertions now also reject current
  wording and lowercase confirmation labels. No product fixes were made.
- The new HTTPS test was developed against scored-result prerequisites and the
  native stream's heartbeat window. Expected signed-out session 401 is recorded
  separately from unexpected errors. Repeated fixture creation correctly hit
  production onboarding 429; one fresh process was used for the final smoke, with
  rate-limit configuration unchanged.
- `leaderboardLive.browser.ts` was not run: it deliberately routes SSE directly to
  port 3000 and owns/restarts that API. The HTTPS test provides same-origin proxy
  event evidence; it does not claim the unrun suite's full transport-crash matrix.

## Untested and remaining gates

- Native 200% browser zoom: four Control++ attempts in headless Chrome left viewport,
  DPR and visual scale unchanged. No Xvfb/xdotool facility was installed. CSS or
  pinch scaling is not counted as native browser zoom.
- Physical Android Chrome: no attached Android browser/device or `adb` available.
  Desktop mobile emulation covers layout only.
- Full production Compose build, public TLS/DNS and a new backup/restore exercise
  were outside this local topology. Deployment recovery remains a critical gate.
- The separate security assessment is still in progress. Its unresolved scope
  remains preserved in `PLANS.md`; this product assessment does not replace it.
- Invitation issue/join is exercised, but invitation-administration rotation and
  revocation and roster withdrawal were not independently browser-validated here.
- No backend/migration production code changed. Rust unit/Clippy and the full
  PostgreSQL test ladder were not repeated. Fresh migration/seed, restricted-role
  startup and real API mutations were exercised in the browser topology.

Independent read-only review found no remaining blocker in the report or
test-only changes. [Recorded run summaries](validation-results.json) retain both
initial failures and final results. Local detailed logs are `/tmp/golf-audit-*.log`; screenshots and
sanitized measured findings needed to reproduce the three UI defects are retained
beside this report. Raw capability links, credentials and session payloads are not
included in the report artifacts.
