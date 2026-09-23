# Result-link mutations now reject late session expiry

The [SHARE-1 repair](validation/result-share-session-expiry-2026-09-23/README.md)
rechecks session validity after grant/audit writes immediately before commit.
Replacement also checks after the old grant's audit, before inserting the new
link. Detected expiry now returns 401 and rolls back every grant/audit change
without an invalidation event. Failed replacement/revoke preserves the original
grant unchanged; an otherwise valid capability remains usable.

Eight deterministic database cases cover expired and valid sessions for issue,
revoke and both replacement audit writes. Before the repair, expired issue/new
replacement returned 201, revoke returned 204, and expiry between replacement
writes returned 500. All four expiry cases now return 401; controls preserve
normal responses, audit counts and exactly one event. Tests observe the exact
wait while the session is active and then wait for database wall-clock expiry.
The synthetic maintenance/advisory waits do not demonstrate an anonymous ability
to induce such a delay in production.

The change reuses existing held session/user locks and the active-session
predicate; exact membership, expected-grant intent and public projections remain
unchanged. No migration, dependency or frontend source changed. Expiry during
COMMIT itself is outside the promised boundary. Independent read-only code,
regression and documentation review found no issues.

Validation passed formatting, 215 backend tests, Clippy, migration/seed, 35
frontend tests and a frontend production build. The complete PostgreSQL-enabled
rerun passed 615 tests, with three existing benchmark tests ignored. Its first
attempt hit an unchanged round-configuration race test (409 versus 200); that
test passed alone and in the full rerun. The report preserves that failure and
the timing-test concern instead of silently treating the first run as green.

Both sharing scenarios passed in installed Chrome 153 at 320, 390 and 1280px.
Fresh representative screenshots were inspected. Disposable services and
synthetic credentials were removed. A sandboxed database connection initially
needed local permission; no cybersecurity safeguard blocked the work.

SHARE-1 is repaired. Overall deployment remains **NOT READY** pending broader
persistence, operational and public-host/device gates. The next proposed bounded
step is a read-only browser/offline persistence assessment. Work remains local
without a push; no queued work was started.
