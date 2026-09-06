# Latest iteration: Supplied men's red-tee course presets

Hacienda del Alamo Golf Club (72.5/125), Saurines Golf Course (66.2/116), and
Mar Menor Golf Course (66.9/118) are now built-in selectable layouts. Each uses
the user's men's Red Tees rating, exact supplied 18-hole pars and stroke indexes,
and totals par 72. Distances and location remain null. No provider provenance or
external verification is claimed.

## Behavior and boundaries

Schema 21 creates finalized manual course/tee/hole revisions and an explicit
preset registry. Existing private course revisions never become presets merely
because they exist. The new exact-tournament-admin GET authorizes and assembles
facts within one repeatable-read transaction, retaining membership locks and
private/no-store responses.

In administration, select Baner, expand a draft round, and choose Velg lagret
bane. Inspect the men's tee, rating, slope, total par, and expandable ordered hole
table. Bruk lagret bane på runden explicitly saves an independent 18-hole revision
through the existing manual configuration endpoint. It retains CSRF, exact
authority, optimistic version, draft-state, duplicate-submit and atomic rollback
protections. Other rounds and historical results are unchanged. Provider catalog
and manual entry remain separate alternatives.

The shared save hook now discards late responses after workspace unmount,
preventing private cache reinsertion after an account change. A regression test
clears the cache before a pending successful response resolves.

## Validation and review

- Rust formatting, workspace/all-target tests (114), and all-feature Clippy with
  warnings denied passed.
- Full PostgreSQL 17 workspace/all-target database ladder passed: 336 tests,
  including five new preset tests. Fresh migration, migration no-op and repeated
  development seed passed on a disposable database.
- Schema-20 upgrade tests preserve existing rounds, opened handicap snapshots
  and round-specific team ownership. Exact arrays, immutable revisions,
  registry-only reads, authorization, independent saves and stale/no-orphan
  behavior are covered.
- Existing configuration, lifecycle and archive fixtures now scope their counts,
  mutations and scoring holes to their own rows instead of assuming there are
  no built-in finalized courses. Production guards were not weakened.
- Frontend tests (338), strict typecheck, ESLint and production build passed.
- Two real Chrome scenarios passed at 320, 390 and 1280px: all three selections,
  exact 18-hole tables, successful independent saves, persisted round summaries,
  focus restoration, loading, empty, unavailable/retry and long-content states.
  Happy-path console/page errors and failing HTTP responses were empty. Mobile
  populated and desktop long-content screenshots were visually inspected.
- Independent read-only review reported no actionable implementation findings.

## Release and deployment limits

**READY WITH KNOWN LIMITATIONS** for code publication. The existing production
bundle remains above Vite's 500 kB warning threshold; the build succeeds.

Only the disposable validation database was migrated. The workspace-configured
database at localhost:5432 refused connections; no retained or production
database was changed. Availability in the deployed application requires the
normal backup, schema-21 owner migration, runtime permissions refresh, and
matching API/frontend release described in deployment_guide.md. Do not run the
development seed on a retained database. Production rollout and a separate
production least-privilege deployment rehearsal were not performed.

General-purpose editable course libraries and additional courses are outside
this step. Supplied values remain user-authoritative, not externally verified.
