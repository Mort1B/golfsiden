# Pairing and tournament-start responses stay with their session

**READY** for both requested frontend repairs. Late pairing saves or tournament
starts could previously update private cache/local state after logout, account or
session replacement, target change or navigation. Each editor now has an
account/CSRF/target-owned lifetime, with synchronous cleanup and canonical-session
checks before dispatch and response effects. Pairing discard-and-reload also
checks ownership after its awaited refresh. New sessions retain their own drafts,
receipts and busy state.

For example, renewing a session during a pairing save no longer lets the old
response overwrite the new flight draft. Current-session saves, conflicts,
explicit retries and tournament-start readiness remain intact. Manual teams,
formats, public sharing, scoring/handicap rules and server permissions are unchanged.
Ignoring a late response does not undo an accepted server update.

The original implementation failed 44 of 52 held-response regressions; all 52
pass after repair. The full frontend suite passed 881 tests across 123 files,
with frontend/browser TypeScript, ESLint and production build passing. Twenty
installed Chrome scenarios passed at 320/390/1280px using the production frontend
and synthetic HTTP/SSE fixtures. Screenshots, cache/network effects and errors
were checked. Independent read-only review found no actionable issue. The
[repair report](validation/management-lifetime-2026-09-29/README.md) retains evidence.

[PLANS.md](PLANS.md) now defines `TOURNAMENT-EDIT-1` for basic draft-tournament
name/description/date changes with administrator/version checks. It is planned,
not implemented. The [offline assessment](validation/offline-course-assessment-2026-09-29/README.md)
finds a practical connectivity gap: an already-open card supports durable local
edits and reconnect delivery, but main-Score return and reopening the app without
coverage are not supported. `OFFLINE-RETURN-1` is queued for explicit return within
the running app; reliable reopening after browser closure/eviction needs a separate
prepared app shell and local scorecard design. No offline runtime changes were made.

Deploy rebuilt frontend assets normally. No API, migration or configuration change
is needed. gg26.no and existing databases were untouched. The
[testing checklist](testing_checklist.md) now includes the current coverage-gap
procedure and its limits. Backend/database ladders and new offline/physical-phone
browser probes were not run for these frontend-only repairs.
