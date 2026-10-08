# Fantasy release acceptance

FANTASY-6 completes local acceptance of the private Fantasy game. Three new
persisted Chrome scenarios cover nine two-person teams across three rounds,
Stableford net-versus-physical aces, and four-ball followed by early-finished
match play. The existing selection, non-finish correction, privacy and recovery
scenarios also passed. This step changes tests and documentation only; production
behavior and schema 37 remain unchanged.

The nine-team tournament changes partnerships each round. One manager changes
picks and captain, another carries the original lineup through two successive
rounds, and a third misses selection. Their totals are 77, 123 and zero. All
eighteen golfers appear independently of selection, including an unselected
golfer with −15. Placement is awarded once per team before attribution to both
partners; captain doubling applies only to the manager's contribution.

The smaller fixtures verify gross 3/net 1 on a par three earns eagle +3 while a
physical ace earns +10. Quad bogey and pickup each earn −5. Four-ball credits both
partners. Match play awards only +3/+1/−1, even when an ace was recorded before an
early concession; a losing captain contributes −2.

Acceptance repaired test setup rather than production logic: a 100 ms database
fixture deadline was vulnerable to full-suite contention, the new match fixture
assumed UUID-sorted results retained request order, and an existing browser
response recorder hid body-read errors. The deadline helper now uses database
time with bounded setup retries, matches are identified by opponent IDs, and
response diagnostics preserve failures while separately counting explicitly
canceled result reads. Dedicated deadline-race assertions remain unchanged.

Validation completed:

- Backend: 249 default tests; formatting and strict Clippy passed.
- PostgreSQL: 712 tests passed, including populated schema-34 upgrade, repeat seed,
  authorization, scoring and races. Three unrelated match-list performance probes
  remain explicitly ignored because they require exclusive `pg_stat_statements`.
- A fresh disposable database migrated to schema 37 and was seeded twice. Runtime
  initialization/grant scripts passed, and the API used the actual restricted
  login with migration-history writes and schema creation denied. Health and
  readiness passed.
- Frontend: 133 test files / 977 tests passed; typecheck, lint, build and standalone
  browser-test types passed.
- Real Chrome: all six scenarios passed in the final combined run. Three decoded
  `409 fantasy_conflict` result reads recovered; zero canceled 409 bodies were
  observed. The three new scenarios had zero response conflicts or unexpected
  console/request errors. Controlled 503/403 responses exercised retry and denial.
- Rendered states at 320, 390 and 1280 pixels passed overflow and control-height
  assertions. Representative mobile manager and desktop Stableford screenshots
  were visually inspected. Entire flows are not independently rerun at each width.

Read-only review checked expected arithmetic, fixture repairs and evidence scope.
The [durable acceptance report](validation/fantasy-release-2026-10-08/README.md)
contains reproduction commands and the format/field/lifecycle matrix. Twenty-team
coverage is PostgreSQL-only; invalid carry is covered below the browser layer.
These tests do not establish a load-capacity limit or resolve the separate SQLx
transaction-cancellation investigation.

The approved Fantasy implementation plan is complete. No production deployment,
public DNS/TLS acceptance, new restore rehearsal, physical Android testing or
native 200% zoom check ran. Hosted acceptance remains with the operator using the
[deployment guide](deployment_guide.md) and [testing checklist](testing_checklist.md).
