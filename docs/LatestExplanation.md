# Reproduce the overlapping page-return freshness defect

The offline/frozen-page investigation reproduced the intermittent failure and
identified a production refresh-ordering gap, now queued as **M3 — Medium**.
The approved investigation stops before production lifecycle changes. No runtime
source or original browser assertion was changed.

The original test assumes that an enabled score button proves reconnect recovery
has finished. It does not: cached scoring controls deliberately remain available
for local entry while private reads refresh. However, simply waiting longer in
that test would exclude a valid overlap and hide the discovered production case.

`refreshOnReturn` in `frontend/src/api/liveInvalidation.ts` keeps one pending
promise per user/client, covering session validation and all private-query reads.
A later return reuses it without scheduling a subsequent pass. The reproducer
establishes this ordering:

1. Reconnect reads return an open round, open completion state and writable owner.
   Their response bodies are checked and their transfers have completed.
2. A scoring response captured before the lock remains held. Cached input is
   enabled and the live connection has recovered.
3. Chrome freezes the page; the fixture changes the round/access state to locked.
4. On activation, persisted `pageshow` shares the earlier pending refresh.
   Authentication-read count remains unchanged, and no new authority reads occur.
5. The older scoring response is released. The expected read-only notice does
   not appear; the prior editable view remains.

This proves a client freshness defect. The fixture does not attempt a score
mutation, so this investigation establishes neither unauthorized server writes
nor lost durable drafts. A healthy live stream need not reopen on return and
therefore cannot be relied on to trigger another refresh.

## Retained regression and evidence

`frontend/e2e/returnOrdering.browser.ts` is an opt-in regression test with real
Chrome freeze/activation, the existing mocked workspace and a native local SSE
server. It retains the expected read-only notice, disappearance of edit controls
and selected-hole assertions. It captures only endpoint labels and fixture-state
markers, attaches the request ordering, releases held responses and detaches CDP
in cleanup. Response-time fixture state is distinguished from captured authority.

Run against the local Vite frontend:

```bash
cd frontend
GOLF_RETURN_ORDERING_REPRO=1 npm run test:browser:lifecycle -- returnOrdering.browser.ts
```

The test currently fails and must turn green in M3; it is not marked as an
expected success. Without that explicit flag, only this new reproducer is skipped.
The original intermittent case remains enabled and unchanged.

- Five unchanged baseline repeats: **four passed, one failed** at the read-only
  assertion (`/tmp/return-baseline.log`).
- Initial controlled trace reproduced the same missing refresh
  (`/tmp/return-ordering-proof.log`). After adding explicit completed-authority
  checks and cleanup, **all three repetitions failed at the read-only assertion**
  (`/tmp/return-ordering-final.log`). This is unresolved regression evidence.
- Complete existing return suite: **eight passed**, with the opt-in reproducer
  skipped (`/tmp/return-suite.log`). These passing timings do not resolve M3.
  Existing checks exercise 320/390/1280-pixel layouts, native stream restart,
  delayed/failed recovery, retained device edits, session expiry, restricted or
  revoked reads, history/reload selection and the original frozen-page case.
- Full frontend ladder: **613 tests across 107 files passed**, strict type checking,
  lint and production build passed. Logs: `/tmp/return-frontend-full.log`,
  `/tmp/return-typecheck.log`, `/tmp/return-lint.log`, `/tmp/return-build.log`.
  The existing bundle-size warning remains queued separately.
- Read-only review confirmed the ordering, Medium severity, diagnostic scope and
  retained assertions. `git diff --check` passed. Backend/PostgreSQL checks were
  not rerun because no backend/schema/runtime code changed; no production service
  was touched. The synthetic lock and held response isolate the client ordering,
  rather than claiming a real database lock or server-mutation test.

**Investigation complete; M3 production repair NOT READY.** The next approved
candidate must preserve identity checks and bounded concurrent coalescing while
ensuring the second return receives fresh authority after pending work. The plan
records its scope, invariants and validation; performance and security work remain
later in the queue.
