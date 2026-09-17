# Make signed-minimum handicap allocation safe

L1 repairs the generic hole allocator at `i32::MIN` without changing current
stored handicap inputs or their scoring results. The negative branch previously
cast unsigned magnitude 2,147,483,648 back to i32, wrapping it negative. One-hole
allocation could panic in debug builds, and multi-hole allocations could return
the wrong sign and total.

The allocator now widens the negative handicap to i64 before taking its magnitude,
keeps division/remainder arithmetic wide, and checks the final signed result back
to i32 with the existing `ArithmeticOverflow` error mapping. For valid arguments,
every per-hole result fits: magnitude is at most 2^31 and the signed allocation
lies between i32::MIN and zero. The positive branch is unchanged. Hole count and
stroke index validation still precede arithmetic.

For example, i32::MIN over 18 holes gives −119,304,647 on indexes 1–16 and
−119,304,648 on indexes 17–18, totaling −2,147,483,648. On one hole it gives
exactly i32::MIN. Ordinary plus-handicap strokes still go to high indexes;
positive extra strokes still go to low indexes. No rounding policy changed.

## Regression evidence

Four new allocator tests cover:

- Exact signed-minimum allocations over one, nine and eighteen holes, signed
  maximum boundaries and adjacent values.
- Sum conservation using an i64 sum, correct sign, nonincreasing allocation order
  and at most one stroke difference between holes for representative extremes
  and ordinary handicaps.
- Every one of the 65,536 possible stored i16 handicaps across one, nine and
  eighteen holes, comparing all 1,835,008 hole allocations to the previous valid
  allocation policy and checking their totals.
- Invalid nonpositive hole counts and out-of-range indexes, plus first/last
  indexes of a valid i32::MAX-sized course without materializing a huge vector.

Before repair, three new tests failed with overflow; the exhaustive current-
snapshot compatibility test passed. After repair, all **13 scoring-domain tests**
passed in both debug and optimized release builds, including the same exact
boundary assertions and compatibility checks. Independent read-only review found
no source or regression-test issues.

Full affected validation passed:

- `cargo fmt --all -- --check` and strict all-targets/all-features Clippy.
- Workspace/all-targets backend tests: **208 passed**.
- PostgreSQL-enabled workspace/all-targets tests: **572 passed**, including those
  208 non-database cases and the legacy, four-ball, Stableford and match format
  regressions. Two test threads per target used fresh SQLx test databases.
- Existing migration and seed binaries against fresh disposable
  `golf_l1_validation` on PostgreSQL 17.11. Direct reads confirmed **32 successful
  migrations**, eight seeded players and five rounds.
- `git diff --check`; the changed production module has 127 substantive lines
  excluding tests, comments and blank lines.

Source and documentation reviews found no remaining issues.
Evidence logs: `/tmp/l1-red.log`, `/tmp/l1-green.log`, `/tmp/l1-release.log`,
`/tmp/l1-fmt.log`, `/tmp/l1-backend.log`, `/tmp/l1-clippy.log`,
`/tmp/l1-database.log`, `/tmp/l1-migrate.log` and `/tmp/l1-seed.log`.
Logs are disposable; the committed tests preserve the regression proof.

## Scope and limits

The change is confined to the generic allocator and direct tests. Historical
snapshots, team ownership, round locks, audits, persistence schemas and API/UI
contracts are unchanged. Arbitrary-i32 net-score subtraction is a separate
arithmetic boundary and was not broadened by this repair; production snapshots
remain i16. No claim is made that all scoring arithmetic accepts arbitrary i32
inputs.

Frontend tests, browser layout checks and production deployment were not rerun:
there is no frontend or user-facing contract change, and the exhaustive snapshot
comparison preserves current allocations. Backend/PostgreSQL format regressions
exercise affected score projections and lifecycle behavior.

L2 (format-specific course errors), L3 (match-only tournament selection), the
intermittent frozen-page validation follow-up and later performance/security work
remain queued in [PLANS.md](PLANS.md).

**READY for L1.** The bounded allocator repair, regression proof, debug/release
parity, affected validation ladders and documentation are complete.
