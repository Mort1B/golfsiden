# Match history reads only the selected player's cards

**Completed — READY WITH KNOWN LIMITATIONS.** Canonical player-selected match
results/history now uses a filtered full-card round listing. In the synthetic
24-match, three-round scenario, history transfers and validates three cards instead
of 72: **2,643 versus 21,491 gzip bytes, an 87.7% reduction**. Warmed strict decoder
median at 4x CPU slowdown is **0.31ms versus 7.46ms**. The
[report and retained evidence](performance/history-filter/README.md) distinguish
these browser measurements from real PostgreSQL/API correctness checks.

The backend accepts an optional typed `player_id`, checks the same membership and
selects matching IDs before constructing cards. Existing visibility, historical
handicap and writable authority remain authoritative. Filtered responses echo the
round/player; the frontend checks that identity, opponent membership, at most one
card, writable subsets and all existing event/score coherence rules. Hidden final
metadata remains null until release. No schema change or compact summary is used.

Each account/round/player has a separate cache key under the private read-list
prefix. Management and all-results views keep the full listing. This avoids
publishing a partial pairing list through the management cache, but opening a new
player adds one read per round even if all results are fresh. Browser navigation
measured three added reads for three rounds, both all→player and player→another
uncached player; revisiting either filter or all results added none. The latter
direct player transition was driven through the client URL because the view has
no direct other-player link. The full tournament match table remains unchanged.

Readiness gates, cancellation/late-denial guards, private erasure, return/SSE and
mutation invalidation, and the 20-second freshness window are preserved. Invalid
or noncanonical URL text keeps its previous exact-match behavior; valid absent
players return empty cards. Detail, scoring, recovery and assignment PUT remain
unchanged.

## Validation

- PostgreSQL feature suite: **581 tests passed**. Migrate and seed passed on the
  task-owned disposable PostgreSQL 17 database. Listing parity covers roles, states,
  both opponent positions, missing players, hidden/released/corrected results,
  frozen handicaps, writable subsets and removed membership.
- Backend ordinary suite: **208 tests passed**; formatting and all-feature Clippy
  passed. The API regression fails against the previous unfiltered handler
  (two cards instead of one), then passes after candidate restoration.
- Full frontend suite: **735 tests passed** before ten additional filtered transport
  cases. The final transport/cache run passed **38 tests**, including all 30
  transport cases. TypeScript, browser TypeScript, lint and production build pass.
- **42 route-browser cases** and **11 real-API match cases** passed. The three
  filtered-history cases at 320/390/1280px also passed with persisted `pageshow`
  refresh checks. Hidden/released wire payloads, SSE updates, long names, layout,
  overflow and interactions were checked; screenshots inspected.
- **54 native measurement cases** passed with 144 list and 54 table refreshes.
  Filter queries, wire cardinality, completion and body bytes are asserted.
  Three navigation cases and four real-decoder paired probes passed.
- Read-only production and artifact review found no remaining blocker. All 55
  measured asset hashes and production source hashes match current files;
  source-file limits, documentation and diff checks passed.

The initial sandboxed ordinary backend run could not bind its local HTTP mocks;
its complete loopback-enabled rerun passed. An earlier database run ended without
a final result and is not counted. All owned local services are cleaned up after
validation. Nothing was tested against production.

Synthetic transfer/probe timings do not establish production latency or database
speedup. Warmed decoding and viewport emulation do not cover cold JIT, physical
phones or actual BFCache restoration. Database authorization measurements and the
broader security review remain separately queued.
