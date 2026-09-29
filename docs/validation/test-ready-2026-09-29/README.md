# TEST-READY-1: practical application readiness

Date: 2026-09-29. Application baseline: `19fd11b`.
The user owns hosting at gg26.no. No hosted requests, migrations, credentials,
server changes or production data were used.

## Scope and outcome

**READY WITH KNOWN LIMITATIONS for continued application testing.**

Validated existing functionality for continued hands-on testing: organizer/player
access, setup, team and individual scoring, results, confirmation/lifecycle and
saved-data persistence. No application, schema, dependency or scoring-rule changes
were needed by the completed checks. The final lifecycle rerun passed and
independent read-only review found no actionable issue in the changes.

The [manual checklist](../../testing_checklist.md) is the handover for continued
testing. Broader security assessment and unverified neighboring callback concerns
remain deferred, as requested.

Follow-up: [SCORE-READ-1](../score-read-retry-2026-09-29/README.md) subsequently
reproduced and repaired the scoring-read serialization-conflict path. The
observations and limitations below preserve this earlier validation run; its
precise original interleaving was not captured.

## Environment and evidence boundaries

- Fresh task-owned rootless Podman PostgreSQL 17 container
  `golf-test-ready-20260929`, loopback port 55441. Existing containers/databases
  were not reused, reset or seeded. `golf_lifecycle` held the practical journeys;
  `golf_creation` provided a separate fresh seed for the lifecycle rerun.
- API built from this checkout, development cookie mode, explicit migrations,
  `RUN_MIGRATIONS=false`; Vite at `127.0.0.1:5173`, API port 3000. Browser tests
  used installed headless Google Chrome and real PostgreSQL-backed API requests.
- Fresh migrations and development seeding succeeded on both databases. The
  integration suite created its own disposable databases in the same task-owned
  container. It finished before the database restart.
- After validation, the task-owned API, Vite process and PostgreSQL container
  were stopped. The container/data and local artifacts were retained; no existing
  services or hosted data were stopped or removed.
- The practical journey uses API onboarding only to create a synthetic account
  and two draft rounds. Invitation issue/join, saved-course selection, manual
  team/flight assignments, tournament start, round opening and score entry/editing
  use the actual UI. The separate tournament-creation suite verifies UI creation.
- Browser suites deliberately inject faults for loading/error/empty states and
  offline recovery. Those cases supplement the real normal flows and are not
  represented as real server faults. Lifecycle bulk-fills remaining holes through
  authenticated API requests; confirmation and administrator corrections use UI.
- Screenshots/layout checks cover 320×600, 390×844 and 1280×900 in the practical
  journey; existing suites also cover 1440px. These are desktop Chrome viewports,
  not a physical-device claim. No hosted HTTPS/proxy acceptance was performed.

## Validation

Commands ran from the repository root unless noted. Browser commands ran from
`frontend/` with their listed explicit opt-in flags. Raw logs and full screenshot
sets are retained locally in `/tmp/golf-test-ready-20260929`.

| Check | Result |
| --- | --- |
| `cargo build -p golf-api --bins` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `cargo test --workspace --all-targets` | 215 passed |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Passed |
| `cargo test --workspace --all-targets --features database-tests -- --test-threads=2`, task-owned PostgreSQL | 615 passed, including the ordinary tests; do not add the two counts |
| `npm --prefix frontend run test` | 806 passed in 120 files |
| Frontend `typecheck`, `lint`, `build` | Passed; lint repeated after browser-test edits |
| `frontend/node_modules/.bin/tsc -p frontend/tsconfig.browser.json --noEmit` | Passed |
| `GOLF_TOURNAMENT_CREATION_BROWSER=1 npx playwright test --config playwright.lifecycle.config.ts tournamentCreation.browser.ts -g 'ordinary signed-in'` | 1 passed: multiple UI-created tournaments, preserved session and lost-response retry |
| `GOLF_STABLEFORD_BROWSER=1 GOLF_FOUR_BALL_BROWSER=1 GOLF_MATCH_BROWSER=1 npx playwright test --config playwright.lifecycle.config.ts stableford.browser.ts fourBall.browser.ts matchPlay.browser.ts` | 15 passed initially; one outdated slot assertion failed |
| `GOLF_MATCH_BROWSER=1 npx playwright test --config playwright.lifecycle.config.ts matchPlay.browser.ts -g 'non-admin failed device write'` | 1 passed after the assertion correction; all 16 format cases have final passing evidence |
| `GOLF_LIFECYCLE_BROWSER=1 npx playwright test --config playwright.lifecycle.config.ts roundLifecycle.browser.ts` | 2 passed on fresh seed: confirmation/correction/reconfirmation/lock, access/final visibility, and loading/error/empty/long-content states |
| `node docs/validation/test-ready-2026-09-29/journey.mjs before` | Clean final run passed; real UI setup/scoring and exact reload/sign-in comparisons |
| Restart task PostgreSQL container, API and Vite without reseeding; then `node docs/validation/test-ready-2026-09-29/journey.mjs after` | Passed in a fresh browser session: exact team/individual scoring responses and gross/net leaderboard responses unchanged |

The practical journey's final before/after runs recorded no unexpected page,
console, HTTP or request failures and no horizontal overflow. It preserves only
synthetic score/result snapshots for comparison; no session cookies or CSRF tokens
are written to its persistence file. It must only run against isolated local data.

Retained summaries: [before restart](before-checks.json) and
[after restart](after-checks.json). Visually reviewed examples:
[team card at 320px](after-score-team_scramble-320.png),
[individual card at 390px](before-score-individual_stroke_play-390.png), and
[results at 1280px](after-gross-1280.png). The full-page phone captures show the
fixed navigation at its viewport position; the scorecard continues by scrolling.

## Diagnosed failures and changes

1. Sandbox restrictions initially prevented Docker access and loopback binds.
   Rootless Podman plus approved local-network execution enabled actual testing.
   Eight ordinary backend tests initially failed to bind their loopback servers;
   the full rerun with local-network access passed. These were environment errors.
2. The lifecycle test treated visible confirmation as permission for an immediate
   hard navigation. A server response/event can reveal confirmation before the
   local confirmation lease is released; navigation then interrupted cleanup and
   reconfirmation reported a lease already in use. The normal UI disables
   **Korriger score** and guards navigation while that work is pending. The test
   now waits for that control to be enabled before its hard navigation, with no
   timing sleep, lease bypass or application change.
3. The match test edited the second opponent but expected the first local-note
   slot. The failure screenshot retained **Lokalt notat 2: 7**. The assertion now
   matches the stable second slot; recovery, discard and access checks pass.
4. During development of the standalone journey, selectors initially matched both
   round editors, and a score assertion read before the second save completed.
   The final script scopes controls to their round and polls the actual stored
   edited value. These harness corrections did not change application behavior.
5. One completed journey recorded a real scorecard GET returning HTTP 500 with
   PostgreSQL `40001`, `could not serialize access due to concurrent update` in
   `ExecLockRows`. Exact saved-card and result comparisons still succeeded. A
   repeat of the same journey and the post-restart comparison passed without the
   error. Read-only review identified a plausible overlap between a repeatable-read
   authority lock and logout's session revocation, but the exact conflicting row
   and overlapping action were not captured. This is an unresolved transient
   read error, not claimed fixed or silently ignored. No lost data or persistent
   core-flow blocker was demonstrated.

## Review and remaining limits

Read-only diagnosis confirmed both browser assertion corrections against the
implementation and failure snapshots. Independent final review found no findings
in the test changes, journey or readiness evidence and accepted the explicitly
recorded limitations. No source repairs were delegated or made.

The transient read error above remains a known limitation. Public hosting,
physical phones, native browser zoom, external course-provider credentials and
the broader security assessment were not validated here. Existing separate
PERSIST-1 same-account/actual-login coverage limits are not all closed by the
match recovery case. None of these claims is replaced with mocked readiness.
