# Pre-deployment logic and optimization fixes

All three assessed issues are addressed in separate bounded changes.

- R1 repairs the browser observer: expected retryable conflicts and explicitly
  canceled reads are distinguished from unexpected failures. Recovery must
  actually restore both result boards.
- R2 reuses a target only when the current refresh observed a successful network
  read and the same query still passes session and freshness checks. Otherwise
  the explicit read remains. Real-SSE Chrome probes reduced round request starts
  from 24 to 21 for three saves and 60 to 55 for five, with unchanged write counts.
- R3 projects only the requested round. Authorization, full fact loading,
  all-round locks, lazy closure and session rechecks remain shared. Overall and
  breakdown endpoints still calculate the whole tournament.

For example, asking for round five previously projected all five rounds and
calculated overall standings before discarding them. The scoped path uses the
same loaded snapshot to project and hash round five alone. In the release fixture,
median projection time fell from 1.941 to 0.368 ms. This is a CPU projection
measurement; database and network costs are excluded. Broader batch invalidation
and database-loading optimizations remain outside this work.

The new parity matrix checks 720 exact serialized round responses and their
internal finalization flags. Transaction regressions protect cross-round lazy
closure, one post-commit event, rejected-target rollback and revoked sessions.
Independent review found no actionable issues. The
[evidence report](validation/predeployment-fixes-2026-10-14/README.md) records
commands, measurements, validation results and fixture limits.

Validation passed: 1,113 frontend tests in R2, 258 backend tests without the
database feature, 736 tests in the full PostgreSQL ladder, migration/seed checks
and nine retained Chrome scenarios. Typecheck, lint, builds, formatting and
Clippy passed. One legacy-upgrade browser case and three existing database
profiling probes were skipped for the fixture requirements recorded in the report.
The new projection benchmark ran separately.

Physical iPhone Chrome/Safari and hosted acceptance remain operator-owned.
No deployment was performed.
