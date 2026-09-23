# PERSIST-2: bounded score delivery retries after storage failure

Date: 2026-09-23. Starting revision: `3f6e41a`.

**PERSIST-2 repaired.** Failed storage access no longer schedules an unbounded
microtask drain from old in-memory queue entries. This is a frontend runtime
repair shared by ordinary, four-ball and Stableford scoring. No API, database
schema, migration, dependency or scoring-rule change is included.

## Implementation and invariants

[QueueRuntime](../../../frontend/src/features/scoring/offline/runtime.ts) now
returns an explicit failed-read result and a durable-progress delivery result.
Another immediate pass requires successful durable progress and a successful
queue reload. A failed read/claim, null claim, failed post-claim read, failed
acknowledgement write or failed failure-record write ends that drain. The old
snapshot remains visible alongside the storage error; its eligibility is not
proof that continuing is safe. A concurrent reload cannot manufacture progress
for a failed pass by clearing the visible error.

The existing two-second polling timer and explicit manual/reconnect/page-return/
cross-tab signals can retry. Healthy holes still drain promptly. Network retry
backoff, conflict decisions, account/session fences and current cross-tab updates
remain unchanged. A failed acknowledgement preserves the original immutable
request and lease: an exact replay can wait for the existing 20-second lease to
expire. No retry silently rebases a request or bypasses a lease. Protected card
queries are still removed on authorization failure even if recording it locally
fails.

## Evidence

- Three failing-first cases reached the safety cap of **20 attempts** before a
  zero-delay browser task: failed listing, failed claim and no-progress contention.
  See [baseline output](baseline.log). The same checks now stop at **one attempt**.
- Fourteen new runtime regressions cover those cases, post-claim and post-ack
  reads, acknowledgement/failure-record write faults, concurrent reload, immutable
  replay, manual/timer/page-return/online recovery, session teardown and a sustained
  outage with one initial attempt plus one attempt on the two-second timer.
- The focused offline suite passes **43 tests**, including existing legacy,
  four-ball and Stableford delivery, conflicts, live-refetch cancellation,
  cross-tab successors and account isolation. [Focused output](focused.log).
- Full frontend: **782 tests across 119 files passed**, plus typecheck, lint and
  production build. Results: [tests](tests.log), [typecheck](typecheck.log),
  [lint](lint.log), [production build](build.log).
- Installed Google Chrome against the production frontend, synthetic typed API
  responses, real IndexedDB and loopback SSE: new scenarios at **320/390/1280px**
  observe **one failed storage attempt before the zero-delay task**, retain exact
  request/generation metadata, show the storage error and deliver the exact original
  request once after manual recovery. Existing page-return races and match-note
  session scenarios also pass: **16 total scenarios**; see [Chrome output](chrome.log).
- Chrome checks page/console errors, failed requests and HTTP failures, horizontal
  overflow and the retry control's 44px target and clickability. Reviewed captures:
  [storage error at 320px](storage-error-320.png),
  [storage error at 1280px](storage-error-1280.png),
  [recovered state at 390px](recovered-390.png).
- Independent read-only review found no actionable source defect. It confirmed
  that existing lease expiry remains a prerequisite for retry after a claimed
  operation encounters storage failure.

The first full suite exposed a timing assumption in the previous match-retention
regression: it asserted an enabled input immediately after the save error appeared,
while the replacement queue could still be loading. That test now waits for the
same enabled state; no assertion was removed and no production match code changed.

## Limits and closeout

Browser API responses are synthetic; the queue, browser event loop and IndexedDB
are real. Injected faults are bounded to avoid hanging Chrome on broken code;
physical storage exhaustion is not exercised. Backend/PostgreSQL ladders and
actual-server score delivery were not rerun because this step changes only browser
retry scheduling. The earlier PERSIST-1 PostgreSQL-backed repeat remains queued
with its documented Docker permission blocker; no new Docker attempt or platform
safeguard rejection occurred in this step. Physical Android and native 200% zoom
remain outside this evidence.

Only the task's loopback preview/SSE services are used and stopped at closeout.
No production, external test targets or real credentials are accessed. Completed
work is committed to main and pushed to origin/main under the owner's standing
instruction. PERSIST-3 remains separate and unstarted. Deployment is **NOT READY**
while that finding and outstanding operational/public-host/device gates remain.
