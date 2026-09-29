# Draft tournament details — 2026-09-29

**READY WITH KNOWN LIMITATIONS** for the bounded TOURNAMENT-EDIT-1 flow.
The exact tournament administrator can change a draft's name, description and
containing date range from Settings. Started tournaments are read-only. Formats,
round dates/count, teams, score data, snapshots and sharing are unchanged.

## Environment and checks

Fresh task-owned rootless Podman PostgreSQL 17 on loopback port 55443, synthetic
seed/created accounts and tournaments, local API on 3000, Vite on 5173 and Google
Chrome through Playwright. No production or gg26.no access. Clean migration through
33 and seed twice passed. The schema-32 upgrade test preserves existing data.

- `cargo fmt --all -- --check`: passed.
- `cargo test --workspace --all-targets`: 215 passed. The first sandboxed attempt
  could not bind the existing course-provider mock servers; permitted local-host
  execution passed without source changes.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
  Existing vendored SQLx warnings remain dependency warnings.
- Full PostgreSQL-enabled workspace/all-targets run exercised 629 checks.
  After migration 33, two legacy fixtures inserted synthetic rounds beyond their
  tournament dates. Their dates were corrected without changing the scenarios.
  The final full run passed 628 checks; its sole remaining fixture failure in
  `tournament_start` was fixed and that entire target reran successfully (10/10).
  Across the full run and focused rerun, all 629 checks pass. No production code
  changed after that run.
- `npm run test`: 897 passed across 124 files.
- Frontend typecheck, lint and production build: passed.
- `GOLF_TOURNAMENT_DETAILS_BROWSER=1 npx playwright test --config
  playwright.lifecycle.config.ts tournamentDetails.browser.ts`: passed, one
  real database-backed end-to-end scenario, 22 seconds.

The eight new PostgreSQL tests cover exact-admin authority (including rejecting
an unrelated global administrator), strict input, no-op version/event behavior,
round/roster/configuration preservation, stale concurrent edits, start races,
direct SQL guards, schema-32 upgrade, membership removal during a round lock wait,
expiry after an update-trigger wait with rollback/no event, and a fixed-snapshot
containment race. Review found the last race in the initial migration; changed
SQL detail writes now explicitly require READ COMMITTED and the regression passes.

Frontend tests cover failed saves and explicit retry, stale drafts and discard,
accepted writes followed by failed refresh, late responses after logout/account/
CSRF/unmount, readonly statuses, loading/read errors, empty rounds, date/UTF-8
validation and API method/body/identity decoding. A save receipt follows query
reconciliation rather than merely the mutation response.

## Browser evidence

The browser creates a real two-round tournament as an ordinary account with exact
admin membership. At widths 320, 390 and 1280, it checks loading, failed read,
empty round list, populated form, blank name, long content, stale conflict and
network failure. Loading, read-error and empty-list responses are controlled
interceptions; all mutation, conflict, persistence and start checks use the actual
API and PostgreSQL. No horizontal overflow; visible buttons at least 44px tall.

Real saves survive reload, update the tournament heading/list, retain all round
objects unchanged, reject a competing old version, preserve the user's draft,
recover through explicit refresh/retry and become readonly after real start.
Console, page errors and network responses are checked, allowing only the precise
injected 503/409/ERR_FAILED detail/round endpoints. A preliminary run's five-second
receipt assertion expired while the concurrent database suite was busy; the final
run completed after that workload ended without adding sleeps or weakening the
assertion. Other preliminary failures were test routing/selectors, resolved before
this acceptance run.

Inspected screenshots:

- [320px long form](mobile.png)
- [1280px conflict](desktop-conflict.png)
- [320px failed read](mobile-read-error.png)

## Limits

This step does not edit started tournaments or restructure rounds. Offline return
remains the next candidate; cold offline launch remains unsupported. No hosted
rollout, Docker Engine deployment, native 200% zoom, physical Android/iOS test or
new recovery drill was performed. Deploy schema 33 plus matching API/frontend;
see the deployment guide. Local execution logs and additional screenshots were
recorded under `/tmp/golf-tournament-edit-20260929/`.
