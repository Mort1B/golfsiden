# Refresh authority after overlapping page returns

**M3 is complete — READY.** A page return that overlaps an unfinished refresh now
queues another session and authority refresh. Previously, the second return
shared the older promise without scheduling fresh reads. If a round locked after
those earlier authority responses, editable controls could remain after recovery.
The completed refresh now reflects the newer lock and displays the read-only card.

The shared return boundary keeps one active refresh and one queued flag per
query client and user. Same-turn signals coalesce; a return during authentication
or private reads schedules a subsequent pass without cancelling the active pass.
A return during that subsequent pass can queue another. Only external return
signals schedule additional work, so this does not introduce polling or an
unconditional retry loop. The shared promise covers the queued work, and cleanup
occurs inside the drain to avoid losing a return at completion.

Each pass checks the cached account before refreshing authentication, then requires
a successful session for that same account before private reads. Expiry, account
changes and failed authentication stop private refreshes. A later valid return
can retry. An old account's queued pass cannot restart or cancel the replacement
account's authentication request.

Ordinary score-event invalidation, private projection clearing, local scoring
entry, durable edits, pending verification and delivery restrictions retain their
existing behavior. No backend, schema or score-mutation contract changed.

## Validation and review

- Eight focused unit tests cover coalescing, returns during authentication and
  subsequent passes, expiry, account changes, failed reads and later recovery.
  Before the repair, seven failed and one passed (`/tmp/m3-red.log`). The focused
  suite then passed all 23 tests (`/tmp/m3-focused.log`).
- The unchanged controlled browser reproducer passed three repetitions after the
  repair (`/tmp/m3-reproducer-green.log`); the prior investigation recorded three
  failures. It still holds an older scoring response across a frozen-page return
  after completed authority reads, with no added pre-freeze freshness barrier.
- The original intermittent frozen-page case passed five unchanged repetitions
  (`/tmp/m3-original-repeated.log`).
- All 11 return-browser scenarios passed (`/tmp/m3-browser-full.log`). The overlap
  regression is now automatic at 320, 390 and 1280 pixels, with fresh session reads,
  read-only state, no edit controls and preserved hole selection asserted. The
  matrix also covers native stream restart, delayed/failed recovery, durable edits,
  session expiry, restricted/revoked reads and history/reload selection.
- Mobile and desktop screenshots were inspected. Long names wrap, horizontal
  overflow checks pass, and touch targets remain reachable. Captures are
  `/tmp/m3-overlapping-return-320.png`, `/tmp/m3-overlapping-return-390.png` and
  `/tmp/m3-overlapping-return-1280.png`. Browser console, page-error and unexpected
  HTTP checks passed.
- The complete frontend ladder passed: 621 tests across 108 files, type checking,
  lint and production build (`/tmp/m3-frontend-full.log`, `/tmp/m3-typecheck.log`,
  `/tmp/m3-lint.log`, `/tmp/m3-build.log`). The existing bundle-size warning remains
  a performance investigation candidate.
- Read-only review found no source or test issues. `git diff --check` passed.
  Architecture, current behavior and the plan were updated with this iteration.

Browser validation uses a mocked API with real Chrome lifecycle transitions and
a native local SSE server. It establishes client refresh ordering, not real
PostgreSQL lock enforcement or server-mutation authorization. Backend/PostgreSQL
checks were not run because this step changes no backend or schema code.

The next candidate is a measured performance baseline. No optimization or wider
security review was started in this step.
