# Full-card history payload investigation

This investigation changes no production queries, API contracts, payloads or
cache behavior. It measures the current source after the observer-retention repair
at `d1e3d32`, then evaluates an offline player subset of the same full-card format.
The subset is a measurement input, not an implemented or served endpoint.

**Verdict: READY WITH KNOWN LIMITATIONS.** The evidence supports a separately
approved player-filtered full-card read; no production repair is implemented here.

## Current path and consumer boundary

`PlayerHistoryPage` loads rounds and delegates match-only history to `MatchResults`;
mixed-format history links to `match-results?player=…`. `MatchResults` loads the
whole match table and creates a `MatchRound` for each match round. `MatchRound`
requests the ordinary unfiltered listing and filters by opponent identity only
**after** `decodeListing` has decoded every card. Table rows are also filtered in
the browser. A player can belong to at most one match per round.

| Boundary | Source | Relevant behavior |
| --- | --- | --- |
| History routing | `frontend/src/pages/PlayerHistoryPage.tsx:28` | Rounds determine match-only delegation; mixed histories link to the player-filtered result route |
| Parent composition | `frontend/src/pages/MatchResultsPage.tsx:18` | Complete table and current match-round children; readiness hides private content |
| Read/list presentation | `frontend/src/features/matchPlay/MatchRound.tsx:7` | List fetch precedes client player filtering |
| API/cache | `frontend/src/api/matchPlay.ts:13` | Account+round `read-list`, no player parameter; assignment PUT uses the same listing format |
| Parse/decode | `frontend/src/api/http.ts:30`, `frontend/src/api/matchPlay/decoders.ts:5` | JSON parse followed by strict decoding of every card |
| Coherence | `frontend/src/api/matchPlay/cardDecoder.ts:33`, `coherence.ts:4` | Recompute/validate event sequence, allocation, lead, finish and points from permitted evidence |
| Private HTTP | `backend/src/api/match_play.rs:21` | Authenticated session, typed round UUID, private/no-store response |
| Repository listing | `backend/src/repositories/match_play/reads.rs:162` | Repeatable-read membership, ordered match IDs, independent writable checks and full projected cards |
| Restricted projection | `backend/src/repositories/match_play/reads.rs:104` | Permitted events, holes and notes; nullable hidden completion metadata |

The other production list consumer, `MatchSetup`, shares the unfiltered key and
uses opponent IDs for administrator-managed pairs. `MatchRound` also serves
ordinary match lists, match-read workspaces, round details, score selection and
mixed leaderboards. Detail/scoring/offline recovery use separate full-card reads.
These consumers must not receive a partial round through the existing exact key.

The list UI uses identities, opponent names, mode, lead/resolved count, finish,
confirmation and writable IDs. It does not directly display all holes, notes and
events, but its **decoder needs them**. Removing those arrays would remove current
coherence checks; a compact-summary DTO would need a distinct trust/validation
contract and is not proposed as an incidental optimization here.

## Measurement design

The checked-in harness serves unchanged minified production assets with native
synthetic gzip HTTP and SSE. Each context settles initial loading and the required
stream-open authority pass before measuring one independent `match` invalidation.
That refresh must complete exactly one list per round and one table. Every measured
list/table ResourceTiming encoded/decoded body size must equal its server gzip/raw
size. Transfer overhead remains a separate browser field.

There are four collection scenarios: 12 matches/one round; 24 matches/three rounds;
24/three with a restricted final; and a 100/three stress case. All have long names,
fully populated numeric notes and completed draws with varied hole winners and
scores. The final restriction preserves the first nine reports and hides later
results/confirmation/points. Its table excludes final awards. A synthetic viewer
receives no writable IDs in that scenario. “Typical” describes collection size,
not a measured distribution of real tournaments. Stress is not a product capacity
claim; assignment also has an HTTP body limit.

For each scenario, an isolated browser probe bundles the **current real decoder**
in memory, outside the production application. It compares all full listings with
a selected-player subset that intersects writable IDs. Exact decoded subset parity
is asserted. One warmup is discarded; seven batches alternate order, each with
20 iterations. JSON parsing, strict decoding and client filtering are timed
separately for the entire round collection. No network or React runs in that probe.
Payload inventory uses epoch-0 inputs, while production refresh uses epoch-2 names;
the raw evidence retains both without pretending their gzip sizes are identical.

Production cases compare all-results, direct player-filtered results and delegated
player history. Three repetitions rotate route order. The populated scenario runs
at 320/390/1280px, the others at 390px: 54 total cases. Native Chrome uses 100ms
network latency, 200,000 bytes/second download, 93,750 bytes/second upload and 4x CPU
slowdown. No heavy checks run concurrently. The two list/JSON-to-DOM measurements
are **tails after the last list response/JSON completion**, not total decoder or
pure render time; work on earlier cards may already have happened. Event-to-DOM
starts at a browser mark just before the host sends the SSE event, so it also
includes trigger handoff, network, decoding, query notification and rendering.
The list-response tail can include any remaining table wait. DOM cardinality
and the separate decoder probe help distinguish sources without subtracting noisy
measurements and calling the residual “render cost.” Mutation observation and two
animation frames do not prove a physical paint time. Native fetch/JSON wrappers
and DOM observation can themselves affect scheduling.

## Results

All **54 production-browser cases** passed, with 144 measured list responses and
54 table responses, all completed. Each route still makes one required list read
per round and one table read. All routes transfer the same full listings for the
same scenario, even though selected-player routes render one card per round.
No unexpected console/page/network/server errors or horizontal overflow occurred.
All 55 production asset hashes match the previous observer-retention build.

These totals cover all round listings per refresh; sizes are bytes of synthetic
epoch-0 JSON and independently gzip-compressed response bodies. The potential
subset uses the same cards and strict decoder, with no summary redesign.

| Scenario | Cards fetched → displayed by history | Full raw bytes | Full gzip bytes | Subset gzip bytes | Potential gzip reduction | Decode median, full → subset |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 12 matches × 1 round | 12 → 1 | 84,137 | 3,972 | 879 | 77.9% | 1.61 → 0.12 ms |
| 24 matches × 3 rounds | 72 → 3 | 504,591 | 21,491 | 2,634 | 87.7% | 7.57 → 0.30 ms |
| 24 × 3, restricted final | 72 → 3 | 426,714 | 18,384 | 2,425 | 86.8% | 6.42 → 0.25 ms |
| 100 × 3 stress | 300 → 3 | 2,102,034 | 81,445 | 2,634 | 96.8% | 33.36 → 0.30 ms |

For 24 × 3, the subset is 21,267 raw bytes instead of 504,591 (95.8% less).
Full JSON parsing adds a median 2.95ms in the isolated probe, separate from its
7.57ms strict decoding. Holes, notes and events account for about 88.4% of raw
listing bytes, but compression already removes much of that repetition. This is
why the report uses gzip bytes and actual decoder measurements, not raw bytes
alone. Median/range and individual batch timings remain in the artifacts.

At 390px the **unchanged** all-results route contains 832 descendants in its match
page for 24 × 3, versus 45 for either player-filtered route. Median event-to-complete
DOM is 287.2ms for all results, 263.7ms for direct filtered results and 262.8ms for
history. The tail after the last list response is 62.8, 39.4 and 39.5ms respectively.
These compare current routes with **identical full payloads**, not a new endpoint;
they show that filtering already limits rendered output while payload/decoder
work remains. They do not isolate React rendering or prove how much a new endpoint
would improve end-to-end latency. Stress history still has only 45 descendants but
its response-to-DOM tail is 144.7ms with 300 decoded cards.

The body/decoder reduction is material enough to justify the narrow follow-up.
Do not bypass coherence validation, cancel required authority reads or change
freshness to obtain it. Total match-table transfer remains outside this proposal.

Artifacts: [summary.json](summary.json) contains grouped medians/ranges and hashes;
[evidence.json](evidence.json) retains every probe batch, server request and browser
resource entry; [samples.csv](samples.csv) exposes route measurements;
[bundle.json](bundle.json) verifies fresh build attribution against emitted assets.

## Supported follow-up boundary

A player-filtered **full-card** listing could avoid transferring and decoding
other players' cards while preserving every current card check. Keep unfiltered
GET, assignment PUT, table, detail, scoring and management behavior unchanged.
Use a typed player parameter, ordered selection before card construction, existing
membership/projection/writable checks, and a separate account+round+player key
under the existing private read-list prefix. The response must validate its target
player and round; each card must contain the requested player and writable IDs must
be the matching intersection. Never write that response to the unfiltered key.

This is a proposal, not a completed repair. Switching from all results to a player
currently can reuse the fresh unfiltered cache; a separate filtered key may require
another request. Measure that navigation tradeoff before claiming a universal
benefit. Malformed/noncanonical URL player text currently yields no matching cards;
preserve that behavior deliberately rather than silently normalizing it into a
different player's results. A valid absent/nonparticipant player yields an empty
list. The complete table remains unchanged in this bounded candidate.

Implementation requires disposable PostgreSQL listing parity tests, including
both opponent slots, roles, draft/open/completed/locked rounds, membership loss,
missing players, hidden final/release/correction, frozen handicaps, writable subsets
and authenticated private/no-store HTTP behavior. Existing detail/table privacy
tests do not replace direct listing tests. Browser/cache tests must prove account
and player isolation, mixed/unfiltered/setup compatibility, readiness, all denial
boundaries, held late responses, return and SSE/mutation invalidation. Scoring
permissions cannot be inferred from player identity or visibility. A permitted
finish before hole nine must remain visible even while confirmation is hidden.

## Reproduce

```sh
npm --prefix frontend run build
node docs/performance/bundle.mjs docs/performance/history-payload/bundle.json
node docs/performance/history-payload/browser.mjs /tmp/history-final
node docs/performance/history-payload/summarize.mjs /tmp/history-final/evidence.json docs/performance/history-payload
```

`HISTORY_CASE` and `HISTORY_REPEATS` select pilot runs. The summarizer requires the
complete 54-case clean-production run. Screenshots in `/tmp/history-final/` are
temporary. Retained evidence includes source/build and fixture hashes, raw timings,
request outcomes and body sizes; only synthetic identities and data are used.

## Validation

- Fresh production build (including TypeScript) and bundle attribution passed;
  all 55 hashes equal the previous build and measured assets. Backend/frontend
  source status is clean at `d1e3d32`.
- 94 focused tests across seven files passed: card/list/table decoding, transport
  cancellation, shared management reads, live invalidation, private-result denial,
  route ownership and gated observer/privacy/freshness behavior.
- 54 final browser measurement cases passed. All four isolated decoder pairs pass
  exact full-to-subset semantic parity, including restricted-final metadata and
  writable IDs. Native byte equality and required request counts are asserted.
- Reviewed phone/desktop screenshots and a supplemental three-case restricted-final
  pass showing the final cards at the bottom. The supplemental pass adds screenshots
  after timing collection; its timings are excluded from the retained 54 cases.
- Independent read-only consumer, contract and harness review; script syntax,
  artifact audit and diff checks passed.
- Full frontend unit/lint and backend/PostgreSQL ladders were not repeated: only
  documentation and standalone measurement scripts changed. No backend/database
  service participates; those performance and authorization conclusions are deferred.

## Limits

No backend/database latency, SQL count reduction, server filtering correctness or
production speedup is established. An offline subset quantifies potential bytes
and decoder work, not an implemented endpoint's latency. Compression ratios depend
on actual data; long-name numeric draws do not cover every scoring event mix.
Warmed isolated timing does not model cold JIT, memory pressure or all concurrent
application work. Three route repetitions are descriptive, not a statistical
performance guarantee. Desktop viewport emulation is not physical-phone or BFCache
validation. Required live authority refreshes remain unchanged throughout.
