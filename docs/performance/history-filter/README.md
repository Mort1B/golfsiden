# Player-filtered full-card history

The optional typed `player_id` round-list query now selects matching IDs before
full-card construction. It preserves the existing membership transaction,
visibility projection, frozen handicaps and independent writable authority.
Filtered responses echo the round/player. The frontend validates those identities,
at most one card, opponent membership, writable subsets and the existing full-card
evidence before inserting into its separate account/round/player cache.
Unfiltered GET, assignment PUT, table, management, detail, scoring and recovery
retain their existing contracts. No schema or freshness changes are included.

## Evidence design

This candidate uses the same synthetic fixtures and native gzip HTTP/SSE procedure
as the [preceding investigation](../history-payload/README.md). Historical artifacts
remain unchanged. `fixtures.mjs` reexports that fixed fixture; `server.mjs` serves
the implemented filtered response shape, including player identity. The browser
asserts query selection, wire card cardinality, required list/table reads, completed
requests and exact ResourceTiming raw/gzip body equality. All-results and selected
views run against the same current production build, avoiding cross-build timing
attribution. The isolated probe uses the real `decodePlayerListing` and validates
exact parity after removing only the echoed identity.

Four collection scenarios cover 12 matches/one round, 24/three, 24/three with a
restricted final and 100/three stress. Each route runs three repetitions; populated
cases use 320/390/1280px and others 390px, totaling 54 cases. Native Chrome uses
100ms latency, 200,000 bytes/second download, 93,750 upload and 4x CPU slowdown.
No heavy checks run during these measurements. Warmed decoder probes alternate
seven batches of 20 iterations after warmup. JSON parsing is separate. Browser
response-to-DOM tails include remaining work and are not pure rendering time.

The measured source is the working implementation based on `2c711ee`, not that
commit's unchanged source. `source.json` records production source hashes and the
production diff hash; evidence records the working-tree status and every emitted
asset hash. `bundle.json` independently attributes the build. No credentials or
real player data occur in these fixtures or artifacts.

## Results

**READY WITH KNOWN LIMITATIONS.** All 54 cases pass with 144 completed measured
list reads and 54 table reads. Required refresh counts are unchanged. Native body
sizes equal server gzip/raw sizes; no unexpected browser/server errors or overflow.
Each selected route transfers one card per round, with identical permitted card
content and writable IDs. Inventory below uses epoch-0 fixtures; measured refresh
sizes (epoch-2 names) are separately retained in `samples.csv` and raw evidence.

| Scenario | Full → selected cards | Full gzip bytes | Filtered gzip bytes | Reduction | Decode median, full → filtered |
| --- | ---: | ---: | ---: | ---: | ---: |
| 12 matches × 1 round | 12 → 1 | 3,972 | 882 | 77.8% | 1.68 → 0.15ms |
| 24 × 3 | 72 → 3 | 21,491 | 2,643 | 87.7% | 7.46 → 0.31ms |
| 24 × 3, restricted final | 72 → 3 | 18,384 | 2,436 | 86.7% | 6.37 → 0.28ms |
| 100 × 3 stress | 300 → 3 | 81,445 | 2,643 | 96.8% | 33.51 → 0.33ms |

For 24 × 3 the filtered raw body totals 21,420 bytes versus 504,591. The echoed
player identity is included in both raw and gzip sizes. These are measured
candidate transfer/decoder savings, not merely the earlier offline subset estimate.
The table still transfers in full. [summary.json](summary.json) keeps medians and
ranges, [evidence.json](evidence.json) keeps requests/timings/hashes, and
[samples.csv](samples.csv) keeps individual route measurements.

## Navigation tradeoff

`navigation.json` records separate native browser checks at 320/390/1280px. After
the required initial stream-open refresh settles, all→uncached player makes three
list requests for three rounds. Player→another uncached player also makes three.
Returning to the fresh all-results or either previously visited player makes zero.
All views retain their respective card counts, without console/network errors or
overflow. Direct player→player is a controlled client URL transition because the
selected view does not offer a direct link to another player; other transitions
use actual links. The existing cache previously reused full lists for these new
filters, so the added request is a deliberate cost, not a universal speedup.

## Correctness validation

- Full PostgreSQL feature suite: 581 tests pass, including listing parity across
  roles, draft/open/completed/locked rounds, both opponent positions, absent players,
  hidden/released/corrected results, frozen handicaps and writable intersections.
  API tests verify unchanged unfiltered shape, authenticated private/no-store
  behavior, malformed/duplicate filters and removed membership. Migrate and seed
  pass against the disposable PostgreSQL 17 instance; no schema changes.
- The new API regression fails against the prior handler (two cards rather than
  one), then passes with the candidate restored.
- Backend ordinary suite: 208 tests pass; formatting and all-feature Clippy pass.
- Full frontend suite: 735 tests pass before adding ten filtered transport cases;
  the final focused transport/cache run passes 38 tests, including all 30 transport
  cases. TypeScript, browser TypeScript, lint and production build pass.
- 42 route-browser cases pass. Eight existing real-API match cases pass; three
  new history cases verify hidden/released final payloads at 320/390/1280px.
  Those three also pass with persisted `pageshow` refresh assertions. Browser
  checks cover long names, overflow and control interaction; screenshots inspected.
- Decoder/cache tests cover player and account isolation, empty/malformed URL
  filters, readiness, denial and late-response guards, independent management data,
  SSE/mutation invalidation and fresh cache reuse. Read-only review found no
  production blocker; its return/wire-assertion evidence gaps were addressed.

The first ordinary backend run could not bind local mock servers inside the
sandbox; the complete rerun with loopback access passed. An earlier database run
ended without a final result; only the complete successful rerun is counted.

## Reproduction and limits

```sh
npm --prefix frontend run build
node docs/performance/bundle.mjs docs/performance/history-filter/bundle.json
node docs/performance/history-filter/source.mjs
node docs/performance/history-filter/navigation.mjs docs/performance/history-filter/navigation.json
node docs/performance/history-filter/browser.mjs /tmp/history-filter-final
node docs/performance/history-filter/summarize.mjs /tmp/history-filter-final/evidence.json docs/performance/history-filter
```

The browser harness serves synthetic HTTP, not PostgreSQL. Database correctness is
validated separately by SQLx and actual-API browser tests. No database timing,
production latency or SQL authorization speedup is claimed. Compression depends
on content, warmed probes do not model cold JIT or memory pressure, and desktop
viewport emulation does not validate physical phones or real BFCache restoration.
The match table remains a full tournament read. Database authorization measurements
and the wider security review remain separately queued.
