# Match-list startup investigation — 2026-09-22

**READY WITH KNOWN LIMITATIONS.** The investigation supports one bounded transport
cancellation repair; it does not establish a production speedup.

Investigation only, against application commit
`d047830f51f051113701d9fb6e45e4ed9eb6a507`. No production source, runtime fetch
policy, dependency, API or schema change is included. The next implementation
candidate is to forward existing query cancellation into list/table HTTP reads.

## What the source establishes

The protected result loader already consumes a TanStack query signal, checks it
before and after loading, and prevents a superseded denial from erasing newer
results. However, `matchApi.list` and `matchApi.table` do not accept or forward a
signal. Their protected consumers discard the query-function context. Cancelling
a query generation therefore stops publication of its result without stopping
its underlying HTTP read or the decoding performed inside the adapter.

Relevant source:

- [Protected query wrapper](../../../frontend/src/features/leaderboards/usePrivateResultQuery.ts)
  and [private-result cancellation/denial handling](../../../frontend/src/api/privateResults.ts).
- [List/table HTTP adapters](../../../frontend/src/api/matchPlay.ts),
  [round-list consumer](../../../frontend/src/features/matchPlay/MatchRound.tsx) and
  [table/parent composition](../../../frontend/src/pages/MatchResultsPage.tsx).
- [Projection erasure and live invalidation](../../../frontend/src/api/liveInvalidation.ts).

On stream `open`, the app cancels and erases private projections, then refreshes
active queries. Clearing the table makes its parent render loading, unmounting
the list children. A replacement list read started by invalidation can therefore
be cancelled when its last observer disappears; fresh table data remounts the
children and starts another read. The installed query library cancels an
unobserved query whose signal has been consumed. This is the source explanation
for the observed initial → refresh → remount sequence, not three independent
requests for different cards.

The initial/reconnected stream refresh remains necessary authority verification.
Removing it, keeping erased private data visible, changing freshness, or
suppressing queued browser-return checks is not justified by these measurements.

## Browser protocol and evidence

The harness serves the unchanged production build with synthetic HTTP fixtures
and native EventSource. Each navigation displays 24 matches per round across
three rounds (72 cards, 48 table rows). It records browser request starts,
completion/failure, fetch signal presence, SSE events, DOM content epochs,
resource timing body/transfer bytes, and separate server arrival/release/finish
metadata. No query cache is patched and no production source is instrumented.
Only synthetic IDs and request metadata are retained; no credentials or bodies
are logged.

Four cases, each repeated twice cold/warm at 320×600, 390×844 and 1280×900:

| Case | Controlled transition |
| --- | --- |
| `natural` | Immediate stream headers race ordinary initial HTTP reads. |
| `late-open` | Initial epoch-0 content settles, then stream headers open after 400 ms; fresh responses use epoch 1. |
| `overlap` | Hold the first three list responses as epoch 0, open the stream at epoch 1, release the old responses 400 ms after the server opens. |
| `reconnect` | After settled epoch 1, emit a match event and hold its three list reads plus table; end the stream, observe empty protected content, allow native reconnect at epoch 2, then release old responses after 400 ms. |

A new browser context provides a cold cache; the warm phase uses a new page in
that context with cached immutable assets and a fresh JS/query cache. CDP uses
100 ms latency, 200,000 B/s download, 93,750 B/s upload and 4× CPU slowdown at every
width. These are viewport-sized Chrome runs, not physical phone measurements.
The fixture advertises a 200 ms EventSource retry for bounded reproduction;
production retry timing is not measured. Final content is checked after 700 ms
settling and zero outstanding non-SSE requests. DOM assertions reject old epochs
after stream open/disconnect and after complete fresh content, including transient
repaints seen by MutationObserver.

The first overlap pilot waited for replacement content before releasing old
responses. It could not finish: newer browser requests had started but had not
arrived at the server while older same-URL reads were held. The retained protocol
uses a bounded hold. Request timestamps document this scheduling observation;
they do not identify an internal Chrome queue/connection mechanism. The discarded
pilot is not counted as a completed sample or evidence of a product failure.

## Results

All **48 navigations passed**: 72 fresh cards and 48 fresh table rows, no old-epoch
repaint after the checked transitions, no pending non-SSE requests, overflow,
console/page/HTTP errors or unexpected API calls. Across the run, all **468 list
reads and 120 table reads completed**. None of those 588 fetch calls had a signal,
and none aborted. These totals include useful reads as well as obsolete work.

Each cell below describes a whole three-round navigation; 12 samples per case
span all widths, cold/warm and repetitions. Starts and browser completions agree.

| Case | List starts/completions | Table starts/completions | List gzip body total | Table gzip body total |
| --- | ---: | ---: | ---: | ---: |
| Natural | 3 or 9 | 2 | 14,607 or 43,821 B | 982 B |
| Late open | 9 | 2 | 43,822 B | 982 B |
| Overlap | 9 | 2 | 43,822 B | 982 B |
| Reconnect, including startup + match event | 9 or 15 | 4 | 43,822 or 73,036 B | 1,964 B |

In the 12 overlap cases, **36 held old list responses** continued to completion
after opening the stream invalidated their query generations. In 12 reconnect
cases, **36 held lists plus 12 held tables** likewise finished after disconnect.
Their obsolete epochs did not reappear. Those controlled responses provide direct
evidence of the transport gap without treating every refresh as unnecessary.
There is no post-repair result in this investigation.

For one representative 390px cold `late-open` sample (repeat 1), timestamps are
milliseconds from that sample's host-side origin:

| Event | Time |
| --- | ---: |
| Initial three list fetches start | 1,659–1,666 |
| Initial epoch-0 content complete | 1,945 |
| Browser receives SSE open | 2,356 |
| Three replacement list fetches start | 2,363–2,367 |
| Parent loading leaves zero cards/table rows | 2,384 |
| Table resolves; three list fetches start on remount | 2,487–2,494 |
| Fresh table rows visible | 2,496 |
| All epoch-1 cards visible | 2,734 |

This ordering, combined with the source cancellation path, explains the three
list generations. The DOM trace observes output, not TanStack internal query IDs;
“on remount” is the source-supported interpretation of the sequence.

The corresponding overlap case receives open at 1,673 ms and clears content at
1,696 ms. Its three old bodies are released at 2,046 ms and all finish; no epoch 0
reappears, and all epoch-1 cards appear at 2,402 ms. In the representative reconnect
case, error arrives at 2,890 ms, content is empty at 2,901 ms, reconnect opens at
3,195 ms and old bodies release at 3,512 ms. Only epoch 2 repopulates the page,
complete at 3,899 ms. These illustrate ordering, not a latency improvement.

## Validation

- Production build passed. A fresh in-memory attribution build matched emitted
  bytes; browser hashes match both that build and the published route-splitting
  bundle. The application source is clean at `d047830`; recorded source status is
  empty. Chrome is 153.0.8010.36 and Node is 22.22.2.
- 28 focused tests across five files passed: private-result denial/cancellation,
  live invalidation, queued return refresh, tournament subscriptions and match
  read/scoring separation.
- All 11 existing production browser return-loading/ordering tests passed,
  including 320/390/1280px unfinished-return recovery with changed authority.
- All 48 investigation samples passed the stronger transient-epoch assertions;
  representative mobile/desktop screenshots were inspected.
- Independent read-only review checked source boundaries and the harness; its
  recommendations added transient repaint assertions and browser resource timing.
  Final evidence review is recorded in the latest explanation.
- Script syntax and diff checks passed. Full frontend unit/lint ladders were not
  repeated for this documentation/harness-only change; no production frontend
  source or dependency changed. The build ran TypeScript compilation. Backend and
  PostgreSQL ladders are outside the affected scope and were not run.


## Reproduce

From the repository root, with installed dependencies and Google Chrome:

```bash
npm --prefix frontend run build
node docs/performance/bundle.mjs /tmp/startup-bundle.json
node docs/performance/startup/browser.mjs /tmp/match-startup
node docs/performance/startup/summarize.mjs /tmp/match-startup/trace.json /tmp/match-startup
```

Optional `STARTUP_WIDTH`, `STARTUP_MODE` and `STARTUP_REPEATS` environment values
select a smaller run. Use different output directories to retain another run.
The browser run reads `frontend/dist`; the attribution command verifies it against
a fresh in-memory build. Do not run heavy checks alongside measurements.

Retained [samples.csv](samples.csv) contains every navigation's counts and byte
sums; [summary.json](summary.json) records source/build provenance and conditions;
[traces.jsonl](traces.jsonl) retains every sample's API request and content timeline
(one JSON object per line). Static asset request rows are omitted from that file;
asset hashes remain in the summary. Full raw traces and screenshots for this run
are in `/tmp/match-startup-final/` and may not survive environment cleanup.

## Safe next implementation boundary

Add optional `AbortSignal` arguments to `matchApi.list`/`table`, pass them to the
existing HTTP decoder, and forward the query context signal from `MatchRound`
and the `MatchResults` table query. Preserve the private loader's pre/post-load
checks, late-denial suppression, query keys, retry rules, projection erasure,
account isolation, writable intent and all required refreshes. No API payload,
backend or schema change is needed.

The management `MatchSetup` consumer shares the list key but uses ordinary
`useQuery` without consuming its signal. Keep its existing lifecycle in this
bounded repair; optional parameters preserve compatibility. Test navigation
between protected results and management so shared-key ownership remains sound.

Acceptance must show **superseded delayed requests abort instead of completing**,
with fresh final content and unchanged privacy/return-order regressions. Record
starts, aborts, completions and body/transfer bytes separately. Fewer request
starts are not required, and aborting browser fetch does not establish cancellation
of already-started SQL. Completed responses cannot be retrospectively aborted.
Transport signal tests and delayed success/401/403/404 ordering tests are required,
along with the frontend ladder and replayed mobile/desktop browser cases.

## Separate findings and limits

- A shared EventSource is not shared invalidation: every subscriber invokes its
  own handler. Match-only player history and global leaderboard each mount a
  parent live hook plus a child `MatchResults` live hook. This is source-traced in
  `PlayerHistoryPage`, `LeaderboardPage`, `useTournamentLive` and `tournamentLive`;
  these delegated routes were not measured here. Only return/resume currently
  coalesces. Changing fan-out is a separate candidate, not part of the proposed
  transport repair.
- Eliminating transient refetches that parent loading immediately unmounts needs
  a separate ownership/freshness design. The current evidence supports transport
  cancellation without that change.
- Synthetic epoch markers prove only frontend handling of supplied data. They
  do not exercise server membership enforcement, hidden-final policy or revocation.
  Existing unit tests cover cancellation/denial ordering; delayed HTTP denials are
  required in the implementation step. The finite DOM observation cannot establish
  behavior outside the measured window.
- Fetch/EventSource wrappers and DOM observation can affect scheduling. These are
  causal request traces, not a new latency benchmark or field performance claim.
  Payloads compress unusually well; epoch markers also differ from the earlier
  baseline fixture. Immutable caching is optimistic compared with current Caddy
  policy. No production delivery, physical device or sustained live load is tested.
- Browser `requestfinished` establishes response completion. Resource timing
  `encodedBytes` is the observed compressed body size; `transferBytes` includes
  the browser's header estimate. Server `bodyBytes` is planned payload size and
  server `finish` is socket handoff, not proof of browser receipt. SSE server-close
  records at teardown are not application-request cancellation evidence.
- Backend authorization reuse, full-card history payloads and real PostgreSQL
  timing remain outside scope. No SQL speedup, backend cancellation or production
  impact is established, and the previous database measurement blocker was not
  retried in this frontend-only investigation.
