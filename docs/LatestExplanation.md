# Browser/offline assessment confirms three boundary defects

The [persistence assessment](validation/browser-persistence-2026-09-23/README.md)
confirmed three P2 issues without changing application code:

- PERSIST-1: same-account session replacement remounts match input and silently
  loses unsaved/failed-save notes and their navigation guard. Real Chrome with
  local authentication reproduced both cases; unchanged-session return preserves
  input, and a durable queued note survives replacement and reaches the server.
- PERSIST-2: a storage failure leaves eligible queued work in memory and causes
  repeated microtask retries. A bounded synthetic Chrome probe observed 52
  storage accesses before a zero-delay timer, then restored storage. It did not
  leave Chrome hung or demonstrate permanent queue loss.
- PERSIST-3: a delayed successful Stableford-settings response recreates the old
  account's memory cache after logout/account switching. The component probe uses
  actual UI/session code with a held mocked API response. Other similar callback
  paths remain source-supported concerns; no other-account UI disclosure or
  server authorization bypass was demonstrated.

Independent read-only reviewers checked each source path, diagnostic probe and
report. The report separates account-scoped durable intent, private query state,
capability state and explicitly untested schedules. Existing account-indexed
storage, immutable conditional requests, session fences and server authorization
passed their scoped validation. No service worker or offline app shell exists.

The full frontend suite passed 751 tests, typecheck, lint and production build.
The four backend score/match suites passed 99 PostgreSQL tests. Twenty-nine distinct existing Chrome scenarios passed across runs, plus four
browser diagnostic cases and three component cases. The initial recovery flow
failed because the temporary harness lacked a recovery origin; both recovery
scenarios passed after that local harness correction. Disposable services and
synthetic credential files were removed. Detailed outcomes are retained with the report.
These checks establish bounded evidence, not whole-application security sign-off.

No implementation, migration, dependency or production configuration changed.
The next proposed repair is PERSIST-1; the other two repairs and operational
assessment remain separate queued work. Deployment is **NOT READY** while these
findings and public-host/device gates remain open. Work stays local without a push.
