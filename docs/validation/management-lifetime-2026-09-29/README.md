# ADMIN-LIFETIME-1: pairing and tournament-start response ownership

Date: 2026-09-29. Baseline: `805f43a`.

**READY** for the two bounded frontend repairs. The user authorized them together,
requested a tournament-editing plan and asked for an offline-use assessment.
Formats and public sharing remain unchanged.

## Confirmed behavior and repair

Both controls could apply late mutation responses after logout, a renewed session,
account/target change or editor departure. Reproduction inspected actual private
QueryClient writes, invalidations, controls, draft contents and receipts/errors
under StrictMode. This establishes client lifetime defects, not server permission
bypass or other-account UI disclosure.

The pairing editor now owns one account/CSRF/tournament/round lifetime; tournament
start owns one account/CSRF/tournament lifetime. Layout cleanup retires ownership
synchronously. Canonical session checks cover publication before React remounts.
Pairing save/adopt/invalidate, conflict refresh and discard-and-reload continuation
are fenced; so are tournament-start cache insertion, receipt, refresh and busy
completion. A replacement owner gets separate mutation and local draft state.
A collapsed pairing section still preserves its current draft.

Normal current-session save, explicit retry, stale-version handling, pairing
validation and tournament readiness remain intact. Backend authorization, manual
team composition, score ownership/handicaps, round locks, API contracts and schema
are unchanged. Ignoring an old response does not undo a server-accepted write;
current authorized reads recover actual state.

## Failing-first and validation evidence

| Check | Result |
| --- | --- |
| Original source, held-response regressions | 44 failed / 8 passed across 52 tests |
| Repaired focused regressions | 52 passed |
| Full frontend suite | 881 passed across 123 files |
| Frontend/browser TypeScript | Passed |
| ESLint and production build | Passed |
| Production frontend in installed Chrome | 20 passed at 320/390/1280px |
| Independent read-only review | No actionable findings |

The component tests cover both success and error outcomes after departure,
canonical-session replacement before React, an overlapping new-session save,
current-session failure/retry, and the pairing discard-refresh continuation.
Current pairing conflicts preserve the dirty draft until explicit discard.

Chrome uses real production frontend routing, session and query logic with typed
synthetic HTTP responses and a loopback SSE server. Held success/conflict responses
at each width preserve replacement state and permit a new save with the new CSRF
identity. Logout, account switch and navigation are covered for both controls;
current-session failures retain explicit retry. Pairings include an empty eligible
roster and long names; start has a populated ready roster. Pending operations,
receipts/errors, no extra response-driven reads, horizontal overflow, 44px primary
targets and clickability are asserted. Only deliberately injected 409/500 responses
are allowed; no unexpected console/page/network error was recorded.

Evidence: [baseline](baseline.log), [focused](focused.log), [full tests](tests.log),
[typecheck](typecheck.log), [lint](lint.log), [build](build.log), [Chrome](chrome.log).
Visually inspected: [pairing phone](pairing-success-320.png),
[pairing desktop](pairing-stale-1280.png), [start phone](start-success-320.png),
[start desktop](start-stale-1280.png), [pairing retry](pairing-error-390.png),
[start retry](start-error-390.png). Retained logs normalize trailing whitespace only.

## Planning and offline handover

`TOURNAMENT-EDIT-1` in [PLANS.md](../../PLANS.md) specifies basic draft-tournament
name/description/date editing, exact-admin/version guards and validation. This is
planned work, not implemented editing functionality.

The [offline course assessment](../offline-course-assessment-2026-09-29/README.md)
finds working durable edits on an already-open card, but no supported offline
return through the main Score link or app reopening after closure/eviction.
`OFFLINE-RETURN-1` is queued for the smaller explicit in-app return path; reliable
closed-app reopening needs a separate prepared-workspace design. No offline
runtime changes were made in this step.

## Limits and closeout

Backend/PostgreSQL ladders were not rerun because only frontend lifetime handling
changed. Browser HTTP is synthetic and does not establish new server persistence
or authorization evidence. Existing offline tests ran as part of the full suite;
no new offline browser/server or physical-phone probe ran. Hosting at gg26.no,
existing databases, format rules and public sharing were untouched.

The task-owned preview is stopped at closeout; SSE fixtures close after each test.
Deploy rebuilt frontend assets normally. No migration or API/configuration change
is required. No broader callback audit or queued implementation is included.
