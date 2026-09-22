# One live owner per match-results route — 2026-09-22

**READY WITH KNOWN LIMITATIONS.** The route-ownership repair removes duplicate
invalidation on delegated match-only results. The direct wrapper now owns its
live subscription; player history and global results retain their existing
parent subscriptions. Shared `MatchResults` owns queries and rendering.

The production change relocates one hook in
[`MatchResultsPage.tsx`](../../../frontend/src/pages/MatchResultsPage.tsx).
Event fan-out, query keys, freshness, synchronous projection erasure, required
authority refreshes, cancellation/denial guards, shared management reads and
queued browser-return ordering remain unchanged. No API, backend, schema,
dependency, style or sporting-rule change is included.

## Comparison protocol

The [baseline investigation](../subscribers/README.md) measured application
`72f8aa9`. This comparison reuses its browser, server, observer and summarizer
**without changes**, serving a fresh production build of the modified worktree
on parent `36248ba`. The summary explicitly records the candidate's frontend
source status; it does not label the parent commit as the repaired build.
Fresh bundle attribution matches the emitted assets and browser hashes.

Both runs contain 24 navigations: direct full results, direct filtered history,
delegated player history and global match-only results, repeated twice at
320×600, 390×844 and 1280×900. Each uses a fresh context, synthetic native HTTP/SSE,
three rounds with 24 matches each, 100 ms network latency and 4× CPU slowdown.
Phases are delayed initial stream opening, a settled match event, held match
responses followed by disconnect/native reconnect, same-account return, then
held old-account responses followed by changed-account return.

Request start/completion/abort timestamps, native callback dispatch IDs, source
creation/closure, server held-response closure and transient DOM epochs are
retained. The observer records native calls without modifying the query cache
or application source. Browser starts and server arrivals are different events;
request starts determine phase attribution.

## Results

All 24 candidate cases pass the unchanged assertions. Every route ends with
fresh epoch-7 results, no old content after checked open/disconnect/account
clearing, no unexpected console/network errors, no overflow and no pending
non-SSE requests. Direct and delegated views perform the same settled match
refresh: three list reads and one table read, all completed.

Per navigation, `S/C/A` means browser starts/completions/aborts:

| Phase and route | Baseline lists; table S/C/A | Candidate lists; table S/C/A |
| --- | --- | --- |
| Settled match, direct routes | 3/3/0; 1/1/0 | 3/3/0; 1/1/0 |
| Settled match, delegated routes | 6/3/3; 2/1/1 | 3/3/0; 1/1/0 |
| Delayed initial open, delegated routes | 9/3/6; 2/1/1 | 6/3/3; 1/1/0 |
| Native reconnect, delegated routes | 3/3/0; 2/1/1 | 3/3/0; 1/1/0 |
| Same-account return, all routes | 3/3/0; 1/1/0 | 3/3/0; 1/1/0 |

For settled match events on delegated routes, request starts fall from eight to
four; the four superseded starts disappear. All retained starts occur inside
one native callback and content stays mounted during the refresh. This is a
controlled request-work reduction, not a production latency or database-speedup
claim. Completed useful reads remain the same.

Delayed opening still starts six list reads on all routes: three during the
required refresh, then three after table-loading unmounts/remounts the list
observers. Three abort. This independent remount cost remains intentionally
unresolved. Initial/account-transition totals may vary with effect timing and
parent composition; per-phase evidence retains them without assigning all such
work to subscription count.

Same-account return still performs one session read. Account change still closes
the old source, clears its private projections and creates a replacement source.
Native reconnect keeps the current source instance. All **192 server-observed
held responses** abort before body release without finishing. Full traces and counts
are retained in [samples.csv](samples.csv), [summary.json](summary.json) and
[traces.jsonl](traces.jsonl).

Across the complete 24-navigation workload, combined list/table starts fall from
1,206 to 972, while completions remain 594; aborts fall from 612 to 378. This
includes account-transition and remount timing, so the settled-event comparison
above is the stronger attribution of the repaired duplication.

## Validation

- 28 new route tests use the real hook, shared transport and invalidation with a
  controlled EventSource. They cover all four entry points, early and settled
  opening, populated match refresh with held responses, visibility clearing,
  child loading/remount/error/empty recovery, fresh 401/403/404 denial, tournament
  switching, old-source suppression, logout and replacement-account ownership.
- Against the previous source, the two delegated settled-opening tests fail while
  the two direct controls pass. With the repair, all 28 pass. This confirms that
  the tests detect duplicate invalidation rather than just shared connections.
- Full frontend validation passed: **677 tests / 112 files**, typecheck, lint and
  production build; browser TypeScript also passed. All **33 production-browser
  route/cancellation/return tests** and **24 comparison navigations** passed.
  Independent source/test and final artifact reviews passed with no blockers,
  verifying all 24 cases, 192 held cancellations, 55 asset hashes and baseline
  comparison totals. Script syntax, local links and diff checks also passed.
- Existing lower-layer tests retain cancelled-success/denial ordering, fresh
  denial scope, account isolation, shared management read ownership and queued
  returns. The production route suite also covers unfinished frozen returns.
- Backend/PostgreSQL ladders do not apply: no backend, persistence or HTTP payload
  contract changed. No real database measurement is claimed.

## Limits and reproduction

The fixture uses synthetic compressible data, desktop Chrome viewports, explicit
persisted `pageshow` and a shortened 200 ms native SSE retry. It does not establish
physical-phone behavior, actual BFCache restoration, server authorization,
production delivery, sustained live load or SQL cancellation. Callback/DOM
observation adds overhead; a finite mutation-observation window is not physical
frame capture. Early stream opening is separately covered by route tests and
the existing production browser suite; the comparative replay deliberately
settles both parent and child before opening.

```sh
npm --prefix frontend run build
node docs/performance/bundle.mjs /tmp/owner-bundle.json
node docs/performance/subscribers/browser.mjs /tmp/match-owner
node docs/performance/subscribers/summarize.mjs /tmp/match-owner/trace.json /tmp/match-owner
```

Do not run heavy checks during measurement. See the unchanged baseline harness
for optional route/width/repetition selectors. Full raw trace and inspected
screenshots for this run are in `/tmp/owner-final/` and may be cleaned. Retained
JSONL omits static request rows; asset hashes and build attribution remain in
[summary.json](summary.json) and [bundle.json](bundle.json).
