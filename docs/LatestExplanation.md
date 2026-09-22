# Transient match-list remount investigation

**Completed — READY WITH KNOWN LIMITATIONS.** The remaining cancelled match-list
refreshes are caused by table-loading unmounts. Open/visibility clears projections
and starts authoritative reads; the parent loading branch then removes the list
observers, cancelling those reads. Fresh table data remounts the lists and starts
a replacement generation. Production source remains unchanged at `61d53da`.

The [investigation report](performance/remounts/README.md) retains source tracing,
production-browser HTTP/SSE/DOM timelines and an isolated installed-library probe.
There are 18 distinct browser cases: direct results, delegated player history and
global match-only results at 320, 390 and 1280px, with settled or held initial
lists. They cover visibility with a held current table, visibility superseding
pending lists, disconnect/reconnect, same-account return, account replacement,
selected fresh denials and recovery. They are not repeated identical samples.

For each populated visibility event, three list reads start in the native callback
and abort while table loading removes the list DOM. Releasing the held table starts
three successful replacements with fresh content and new DOM-node identities.
Reconnect after projections have already cleared starts only three list reads.
DOM-node replacement corroborates the lifecycle trace; query-observer ownership is
established by source, not by React or cache instrumentation.

A separate QueryObserver model compares removal with retaining a disabled observer.
Within the existing 20-second freshness window, retaining it avoids the cancellation
and second start for both table-first and list-first completion. If the list
finishes and the table remains pending for 21 seconds, re-enabling the stale query
starts another read: two complete instead of one completing and one aborting.
That counterexample rules out a universal request or latency improvement claim.
The model is not a production implementation or proof of application correctness.

Two pending-load cases deliver Playwright initial request notifications after
the host phase changes; native fetch timestamps still show exactly six new open
starts. Both counts are retained, so delayed notifications are not attributed to
an extra observer or refresh.

The next proposal retains list observers during table-only pending state while
current readiness gates both initial fetching and every private presentation.
Current rounds alone define identities; errors, missing rounds and scope/round
removal still unmount owners. Other callers keep their behavior. No prior-authority
flag, copied descriptors, freshness change, invalidation redesign or hidden stale
DOM is proposed. The benefit must be validated across both completion orders,
prolonged holds and all privacy boundaries before publication of a future repair.

## Validation and limits

- Production build passed, including TypeScript. All 55 fresh-attribution, emitted
  and browser asset hashes match the retained ownership build; frontend source
  status is clean at `61d53da`.
- **79 focused tests across eight files passed**, covering ownership, fresh and
  cancelled denial, transport cancellation, shared management reads, invalidation
  targets, source sharing and queued returns.
- The six observer-model cases and cold-fetch gate pass their count assertions.
- **18 production-browser cases passed**, including 225 obsolete held responses
  cancelled before release, 36 current held tables completed and 18 expected
  denials. Final content is fresh epoch 10, with no unexpected errors or overflow.
- **11 browser-return regression tests passed**, including unfinished frozen
  returns at mobile and desktop widths.
- Independent source/harness/proposal and final artifact reviews passed with no
  blockers. The audit verified all 18 cases, held-response outcomes, native versus
  browser counts, DOM replacement, denial/account clearing and matching hashes.
  Mobile/desktop screenshots, local links, script syntax and diff checks passed.
- Full frontend unit/lint suites are not repeated for this documentation/harness
  change. Backend/PostgreSQL ladders are outside scope: those layers and contracts
  are unchanged, and real database timing is not available from these fixtures.

Expected 401/403/404 responses are retained as denial evidence. In this browser
protocol, visibility clears projections before denial, so it proves non-repopulation
and recovery. Existing event-driven route tests separately prove denial-caused
sibling erasure. The selected matrix is table 403 at 320px, rounds 401 at 390px and
list 404 at 1280px, across all routes/initial modes; it is not an exhaustive endpoint/status
matrix. Actual hidden-final payload filtering remains a required future repair test.

Synthetic data, instrumented native callbacks, desktop viewport emulation, explicit
persisted `pageshow` and finite DOM observation do not establish physical-phone,
BFCache, server authorization, SQL cancellation, PostgreSQL latency or production
speedup. The readiness-gated observer proposal is unimplemented. Full-card payload,
database authorization and the broader security review remain separate.
