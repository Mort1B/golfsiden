# VISIBILITY-1: final-round visibility response ownership

Date: 2026-09-29. Baseline: `4c117d7`.

**READY** for this bounded frontend repair.

## Confirmed problem and repair

A held visibility save could complete after logout, account/session replacement,
target change or editor departure and still insert its response into the private
cache, show an old receipt/error or initiate a stale-response refresh. This was
reproduced with the real control, QueryClient and session transition code in
StrictMode. No server authorization bypass or other-account UI disclosure is
claimed.

The control now creates a separate editor for each account, CSRF identity,
tournament and final round. Synchronous layout cleanup ends that editor's
ownership. Dispatch and each response-driven write, receipt and invalidation
check both mounted ownership and the canonical session query. An old response
cannot release a replacement editor's busy state. Logout renders no control.

Current-session saves still update authoritative visibility, clear/refetch
visibility projections and show confirmation. Conflict responses still refresh
server state; failed saves retain explicit retry. Locked finals remain editable
for visibility. Backend permissions, visibility/scoring rules, API contracts and
query-key formats are unchanged. Ignoring a late response does not undo an
accepted server write: current authorized reads recover actual state.

## Reproduction and regression evidence

The final regression suite fails **19 of 23 cases** on the original source and
passes on the repaired source. It covers success, stale conflict and failure after
logout, account change, renewed CSRF, unmount, tournament change and round change;
canonical session publication before React replacement; an overlapping new-session
save; normal failure/retry and stale-version refresh on a locked final.

Tests inspect real cache entries, response cache writes, invalidation calls,
refetch calls, receipts/errors, busy controls, request CSRF and expected versions.
An initial canonical-logout test conflated observer recreation after cache removal
with a mutation write. It was separated into canonical publication with retained
cache and the real session-transition matrix. The final failing-first result
above uses that corrected harness; no production cache-clearing behavior changed.

## Validation

| Check | Result |
| --- | --- |
| Focused component regressions | 23 passed; original source 19 failed / 4 passed |
| Full frontend suite | 829 passed across 121 files |
| Frontend and browser TypeScript | Passed |
| ESLint | Passed |
| Production build | Passed |
| Real Chrome at 320/390/1280px | 10 passed |
| Independent read-only review | No actionable findings |

The retained browser tests use real production frontend routing/session/query
behavior with synthetic typed API responses and a loopback SSE service. They
hold mutation responses across renewal/departure and exercise release/hide,
loading, pending and explicit error recovery with long labels and an empty roster.
They are client lifecycle checks, not new PostgreSQL/server acceptance evidence.

Evidence: [original-source failures](baseline-final.log), [focused pass](focused.log),
[full suite](tests.log), [typecheck](typecheck.log), [lint](lint.log),
[build](build.log), [Chrome](chrome.log). The final browser run passed first time.
It checked console/page errors, failed requests, expected HTTP failures only,
horizontal overflow, control clickability and a 44px switch-label target.
Visually reviewed captures: [320px](renewed-success-320.png),
[390px late conflict](renewed-stale-390.png), [1280px](renewed-success-1280.png),
and [320px explicit retry](error-320.png).

Independent review checked mounted/canonical ownership and confirmed that the
visibility invalidation helper sets up its work synchronously; there is no
unfenced continuation after its returned promise resolves. The local preview
service is stopped at closeout. The browser's SSE fixtures close after each test.

## Limits

Only final-round visibility was repaired. Pairing and tournament-start callback
concerns remain separately queued and unverified. No global cache/query ownership
redesign or server/database/dependency change is included. Backend/PostgreSQL
ladders are not rerun for this frontend-only change. gg26.no and existing databases
were not accessed; physical-device and native zoom acceptance remain unverified.
