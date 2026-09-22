# Cancelled transaction starts leave clean pooled connections

**Completed — READY.** The PostgreSQL driver now queues rollback when a request
is cancelled while its transaction is starting. This prevents the next request
from inheriting an open transaction. The repair is a small backport of upstream
SQLx PR #4394 into the pinned 0.8.6 driver, used by both Cargo and the production
container. Repository SQL, authorization, isolation, APIs and browser refresh
behavior remain unchanged. The [report](performance/transaction-cancellation/README.md)
retains the controlled failures, validation and dependency provenance.

The reproducer holds the server acknowledgement of an ordinary `BEGIN`, cancels
its future, then reuses the same PostgreSQL connection. The original driver fails
with SQLSTATE 25001 on the next repeatable-read request. A second regression shows
that cancelling savepoint creation incorrectly loses earlier outer-transaction
writes. The backport fixes both by recording transaction depth before the await,
so the existing drop guard can roll back the correct transaction/savepoint.

A real HTTP test closes a socket while the Axum handler awaits BEGIN, observes the
handler being dropped, and then releases the held reply. The repaired connection
rolls back and serves subsequent admin/viewer requests correctly; removed membership
and expired sessions still fail closed. The original dependency fails this test.
A 100-transaction protocol audit retains one connection and exactly 100 BEGIN,
100 COMMIT and zero ROLLBACK commands on both versions: successful requests gain
no extra query or connection. No latency improvement is claimed.

Validation passes: formatting; 208 ordinary tests; 593 PostgreSQL-feature tests
(including those 208 plus 385 database integration tests); all-feature Clippy;
migrate and seed; production container build; independent read-only review.
All 37 Chrome cases pass at mobile/desktop widths, including 11 real-API match
and history flows against the repaired container with one pooled connection.
The 26 mocked-API cases cover cancellation, hidden results, loading/error/empty
recovery, offline/frozen return and overlapping authority refresh. Screenshots
and console/network assertions were checked. Frontend files are unchanged, so
its separate unit/type/lint/build ladder was not repeated. Three unrelated
release-measurement tests remain intentionally ignored.

The original desktop event's exact abort timing was not retained; the controlled
HTTP test establishes the causal mechanism without claiming to reconstruct that
historical sequence. Physical devices, Safari and a full Caddy deployment were
not tested. The vendored driver retains upstream source organization and known
dependency warnings; remove the override only after validating a compatible
upstream release containing the fix. There is no schema migration. The broader
security review remains a separate next step.
