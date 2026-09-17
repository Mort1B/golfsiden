# Performance baseline — 2026-09-17

Investigation only, against application source `7fe6bcb1ffd6751d6621aba19293a801a885e81f`.
No runtime, dependency, API, migration, authorization or scoring change is included.
The baseline is **READY WITH KNOWN LIMITATIONS**: production-build browser evidence
and source-derived database work are available; real API/PostgreSQL timings are not.

This is the preserved pre-optimization snapshot. The subsequent
[route-splitting comparison](route-splitting/README.md) records the current build;
the baseline findings and samples below have not been rewritten.

## Reproduce

From the repository root, with installed frontend dependencies and Google Chrome:

```bash
npm --prefix frontend run build
node docs/performance/bundle.mjs /tmp/golf-baseline-bundle.json
node docs/performance/browser.mjs /tmp/golf-performance
node docs/performance/summarize.mjs /tmp/golf-performance/browser.json /tmp/golf-performance
BASELINE_CASE=long-390 BASELINE_REPEATS=3 BASELINE_STREAM_DELAY_MS=10000 \
  node docs/performance/browser.mjs /tmp/golf-performance-delayed-stream
node docs/performance/summarize.mjs \
  /tmp/golf-performance-delayed-stream/browser.json /tmp/golf-performance-delayed-stream
```

The scripts read the existing production build and start a temporary loopback HTTP
server. They never contact a real API, provision accounts or mutate a database.
The bundle script compares a fresh in-memory build byte-for-byte with `dist`, then
records SHA-256 asset hashes; the browser output records matching asset hashes.
Do not run builds or other heavy jobs alongside timing runs. Output directories
must differ when retaining multiple runs. The delayed-stream control postpones
the fixture's SSE opening beyond the measurement window; it is diagnostic only.

Retained evidence: [bundle.json](bundle.json), [samples.csv](samples.csv),
[summary.json](summary.json), [control-samples.csv](control-samples.csv), and
[control-summary.json](control-summary.json). Scripts retain full per-request
resource timings and screenshots in the output directory. Committed CSV rows
preserve every timing sample and request/byte totals; summaries preserve conditions,
errors and ranges. Do not compare values across different source/build hashes.

## Conditions and limitations

- Linux, Intel i5-1135G7, eight logical CPUs, about 15.3 GiB RAM; Node 22.22.2,
  headless Google Chrome 153.0.8010.36. Versions and hardware are captured in JSON.
- CDP network emulation: 100 ms latency, 200,000 B/s download (1.6 Mbps),
  93,750 B/s upload; CPU slowdown 4×. **All widths**, including desktop, use this
  same profile. This is a throttled laptop, not physical-phone measurement.
- Gzip is used for fixtures and assets. Cold means a new browser context with an
  empty HTTP cache. Warm means full navigation in that context with cached static
  assets and a new JavaScript/TanStack Query cache, not an in-app warm transition.
  The fixture sets immutable asset caching explicitly; the current Caddyfile does
  not set that policy. Warm results are an optimistic cache control, not verified
  production cache behavior. No DNS, TLS, Caddy, zstd or server delay is measured.
- Login is unauthenticated. Other cases use a synthetic administrator and visible
  final results. All matches are net draws with two zero-handicap players, 18
  numeric agreed halves, 36 notes and frozen full-hole metadata. Pairs are manually
  specified by the fixture, unchanged across its rounds; teams are not generated.
- Populated: 24 players/12 matches in one round, short names. Long: 48 players,
  24 matches per round across three rounds, long names. History: that same long
  workload filtered to one player. Stress: 200 players, 100 matches per round
  across three rounds; a capacity probe, not an observed production tournament.
  Repeated names, UUID prefixes and drawn-hole data compress unusually well:
  a 24-match listing is 167,861 decoded bytes but only 4,853 gzip bytes. These
  payloads exercise full decoding/rendering; their wire sizes are not production
  network estimates. Round IDs cause small compressed-size differences.
- Five cold/warm pairs per 390px scenario; three pairs each for the long workload
  at 320×600 and 1280×900. The 390px viewport is 390×844. The primary run contains
  62 navigations; the separate diagnostic control contains six.
- Ready time is navigation start to expected cards/table being present followed
  by two animation frames (login: username field present). It is a harness metric,
  not time-to-interactive. FCP, observed LCP and a long-task-over-50ms sum are also
  retained. LCP and this TBT proxy are **not** finalized field Core Web Vitals or
  Lighthouse metrics. Measurements end 700 ms after first ready; final card/table
  counts and zero outstanding non-SSE requests are asserted. Later events are
  outside that window. Medians/ranges describe these few runs, not population p95.
- No production traffic, real API latency, SQL execution, connection-pool
  contention, private-read enforcement, revocation, hidden-final transitions,
  mixed-format data, varied reports or sustained score-event load is benchmarked.
  PostgreSQL measurement was unavailable: no local PostgreSQL client/server
  binaries were found; Docker socket access was denied and `sudo -n docker ps`
  required a password. No production/shared database was substituted.

## Bundle evidence

| Asset | Actual bytes | Gzip bytes | Offline Brotli size |
| --- | ---: | ---: | ---: |
| JavaScript (one entry chunk) | 782,064 | 222,606 | 184,140 |
| CSS | 84,119 | 14,306 | 12,532 |

Every cold scenario, including login, transfers the same JavaScript body. Browser
resource timing reports 222,906 transfer bytes including its fixed header estimate;
warm JavaScript transfers zero bytes. Brotli values are compression estimates,
not served traffic; production Caddy is configured for zstd/gzip.

The Vite >500 kB warning reproduces. `frontend/src/router.tsx` statically imports
all route pages; `main.tsx` also eagerly imports shared providers and styles.
Largest Rollup **pre-final-minification** contributions are react-dom 561,323 B,
react-router 225,403 B, tournament features 180,632 B, API/decoders 169,506 B,
scoring features 143,578 B, pages 86,606 B, query-core 75,737 B and match features
68,558 B. These values are attribution clues, not additive final gzip savings.
React and shared correctness/queue code cannot simply be removed.

## Browser measurements

Ready time in milliseconds, **median (min–max)**. Cold and warm are paired within
each repetition but no speedup from an implemented optimization is claimed.

| Workload | Width | Pairs | Cold ready | Warm ready |
| --- | ---: | ---: | ---: | ---: |
| Login | 390 | 5 | 1,731 (1,716–1,738) | 255 (238–256) |
| Populated, 12 cards | 390 | 5 | 2,370 (2,281–2,413) | 855 (837–871) |
| Long, 72 cards | 390 | 5 | 2,500 (2,419–2,524) | 1,021 (1,004–1,038) |
| One-player history, 3 displayed cards | 390 | 5 | 2,412 (2,314–2,472) | 968 (954–972) |
| Stress, 300 cards | 390 | 5 | 2,896 (2,866–3,298) | 1,820 (1,339–1,869) |
| Long, 72 cards | 320 | 3 | 2,505 (2,450–2,527) | 1,020 (1,005–1,037) |
| Long, 72 cards | 1280 | 3 | 2,426 (2,402–2,461) | 1,005 (984–1,006) |

Long results and history both fetch the same 72 full cards. Per round, the payload
contains 24×18 holes, 24×36 notes and 24×18 report events even though a summary row
renders only opponents/result/links. History reduces rendered DOM (96 versus 883
elements), but its client-side player filter does not reduce list traffic or
decoding. Stress renders 3,467 elements; its median long-task-over-50ms sum is
218 ms cold/108 ms warm versus 97/0 ms for the 390px long workload. This is a
combined startup/decode/render observation, not isolated attribution to one function.

| Workload | List HTTP calls cold | List HTTP calls warm | Gzip list bodies observed |
| --- | --- | --- | --- |
| Populated: 1 round × 12 | 1–3 | 3 | 2,723–8,169 B |
| Long/history: 3 rounds × 24 | 3–9 | 9 | 14,571–43,713 B |
| Stress: 3 rounds × 100 | 3–9 | 3–9 | 53,530–160,590 B |

These are sums of completed list responses in the window, excluding HTTP headers,
table/round/session responses and the open SSE connection. Nine requests represent
three reads for each round, not nine distinct rounds. The production client is
unmodified; initial SSE `open` clears private projections and invalidates live
queries. Response ordering changes whether initial reads overlap this refresh.
`matchApi.list` does not propagate an AbortSignal to HTTP, so discarded query
results can still consume transfer/server work. Correct projection erasure remains
required; these counts do not establish that any particular refresh is dispensable.

In the separate delayed-stream control, all six long-390 navigations made exactly
three list requests (14,571 compressed body bytes). Median ready times were
2,296 ms cold (2,268–2,312) and 819 ms warm (803–821). This supports stream startup
as the trigger for extra traffic in these conditions. It is not a proposed fix:
delaying/suppressing live recovery would change freshness and privacy guarantees.

## Database work: source-derived, not timed

The ordinary frontend requests one match listing per round, not one HTTP request
per card. The server then loads each card sequentially in one repeatable-read
transaction. For M matches, successful authenticated list requests execute:

| Reader | SELECT statements including HTTP session extraction |
| --- | ---: |
| Admin/scorer with both opponents authorized | `5 + 17M` |
| Viewer or player without a linked player ID | `5 + 12M` |
| Linked player denied on the first opponent each time | `5 + 13M` |
| Linked player requiring the second-opponent check for K matches | `5 + 13M + 4K` |

Add three transaction-control statements (`BEGIN`, isolation `SET`, `COMMIT`).
Gross/net have the same query count. Draft/missing-owner cases can short-circuit
and must not be assumed to follow the authorized formula. Counts are a source
trace for valid successful reads, not database trace output or latency claims.

`backend/src/repositories/match_play/reads.rs::list` has four base SELECTs:
context, session, membership and match IDs. Each authorized card adds aggregate
(1), authority (10), and card assembly (6). HTTP authentication adds one.
`match_play/mod.rs::authority` checks membership/session, then calls
`score_authorization::authorize_mutation` for each opponent. Each call loads the
round, locks/checks the session, checks membership and resolves owner IDs.
`allocation` and `build` load both preserved handicaps, indexes, opponents, holes
and notes. These checks protect real invariants; query reduction needs parity
evidence, not removal of authorization.

At 12/24/100 authorized matches, that is 209/413/1,705 SELECTs per listing.
Worse, `score_authorization.rs::individual_owner_ids` loads/sorts the full P-player
snapshot owner set for each privileged opponent check: **2MP owner rows**, or
**4M²** when P=2M (576/2,304/40,000 rows at those sizes). This work is repeated for
each received list request. Longer transaction/pool occupancy is plausible but
unmeasured. Existing round-match and note indexes do not remove repeated queries.

The repository assignment limit is 500 pairs, while the assignment HTTP body has
a separate 32,768-byte limit. Do not infer that 500 assignments fit one HTTP call.
The stress fixture does not prove setup or backend capacity at any size.

## Prioritized findings and next repair

No high-severity production performance defect is established by this baseline.
Priorities below concern demonstrated work and measured synthetic loading costs.

| Priority | Finding | Evidence and implication |
| --- | --- | --- |
| Medium — first repair | All routes eagerly load one 223 kB gzip JS body | Login and results both pay startup transfer/parse cost. Defer route-only code and measure the entire request waterfall, including added chunks. |
| Medium | Repeated complete-owner-set authorization reads | Source proves quadratic owner-row materialization and many sequential queries. Real PostgreSQL timing/parity is required before selecting an authorization refactor. |
| Medium | Full-card fetch/decoding for one-player history | History displays three cards but fetches all 72; the same list payload is used for summary rows. A compact/player-scoped contract is a separate future change. |
| Medium | Stream-open request amplification | Measured list counts vary with initial response order. Freshness/projection clearing is required; do not suppress it to improve counts. Investigate cancellation/coalescing separately. |
| Low | Long-list rendering/decoding cost at 300 cards | Stress case increases DOM and long-task work; no production size distribution or interaction-latency evidence yet supports virtualization. |

**One bounded next candidate: route-level JavaScript splitting.** Defer route-only
page modules through the existing router, retain shared authentication, account
isolation, scoring guards, offline queues and provider lifetime, and add accessible
loading plus recoverable chunk-load failure behavior. No API, schema, scoring,
authorization, cache-staleness, SSE policy or global dependency rewrite belongs
in that step. Shared modules may remain in the entry chunk; do not force manual
chunks merely to silence Vite's warning.

Validation must compare the same cold/warm workloads and total initial compressed
JavaScript (entry plus all loaded chunks), ready-time ranges, requests and chunk
failure recovery. Exercise direct links and in-app navigation to score, match,
management, public shared results and account pages, including account changes
and pending edits, at mobile/desktop sizes; run the full frontend ladder.
Stop with measured reduced startup bytes, no material ready-time regression and
passing behavior checks, or explicitly reject/rebound the candidate if route
splitting cannot improve the measured result. Do not start SQL repairs or the
security review in that step. No optimization is implemented in this baseline.

## Validation record

Production build and byte-for-byte attribution build passed; existing bundle
warning remains. All 62 primary navigations and six control navigations passed
with expected final content, zero unfinished non-SSE requests, no horizontal
overflow and no console/page/HTTP errors. Mobile/desktop screenshots were inspected.
Final independent read-only review found no actionable issues. Script syntax
checks and `git diff --check` passed. The scripts and documentation are the only
changed files: the full
frontend unit/lint ladder and backend/PostgreSQL ladders were not rerun because
no product code, dependencies or schema changed. The missing database timing
environment is explicitly separate from those non-applicable regression gates.
