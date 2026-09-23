# Handicap corrections recheck session expiry before commit

AUTH-2 is repaired. After the handicap update and audit read, the repository
reuses the existing active-session check immediately before committing. Natural
expiry during a database wait now returns 401 and rolls back both records; the
API emits no invalidation. Valid corrections still write one audit entry and
publish one matching event. Existing lock order and unchanged-value errors remain
unchanged.

Three new regressions observe real PostgreSQL waits on the tournament, membership
and roster update. All three returned 201 on the old code after their sessions
expired; all now return 401 with unchanged persisted data and no event. Three
valid-session controls continue to succeed. The guarantee is a final wall-clock
check after transactional waits, not an atomic expiry check during COMMIT itself.

Validation passed the nine-test correction suite, formatting, 215 backend tests,
Clippy, migration and seed checks. The complete database-enabled run passed 607
tests (including those 215); three existing performance measurements remained
ignored because their `pg_stat_statements` setup was not enabled. Independent
source and documentation review found no issues. The disposable database was
removed. Details are in the [repair report](validation/handicap-session-expiry-2026-09-23/README.md).

This repair is **READY**. Both confirmed authentication findings are now repaired,
but deployment remains **NOT READY** pending the broader security assessment and
existing public-host/device/browser gates. Frontend/browser checks were not rerun
for this repository transaction change. Work remains local, without a push.
The next proposed bounded step assesses recovery capability and credential
revocation behavior; implementation stays separate from that assessment.
