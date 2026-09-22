# Match-result live subscriber investigation — 2026-09-22

**READY WITH KNOWN LIMITATIONS.** One settled match event starts twice the
list/table reads on delegated match-only results as on the corresponding direct
route. The extra reads are cancelled; this is duplicate invalidation work, not a
second live connection. The evidence supports one small route-ownership repair.

This step changes documentation and an observation harness only. Application
source remains at `72f8aa968232095b2d43f76c15a3fa551f0ca3bf`. No subscription,
query policy, privacy guard, API, scoring rule or backend behavior is changed.

This is the preserved pre-repair snapshot. The subsequent
[ownership comparison](../live-ownership/README.md) records the implemented
repair; the original measurements and proposal below remain historical evidence.

## Ownership established by source

| Entry point | Live owners once results are mounted | Visible fixture |
| --- | --- | --- |
| Direct match results | `MatchResults` | 72 cards, 48 table rows |
| Direct filtered match history | `MatchResults` | 3 cards, 1 row |
| Match-only player history | `PlayerHistoryPage` and `MatchResults` | 3 cards, 1 row |
| Global match-only results | `LeaderboardPage` and `MatchResults` | 72 cards, 48 rows |

[`tournamentLive.ts`](../../../frontend/src/api/tournamentLive.ts) shares one
EventSource per account/tournament, then synchronously notifies **every** listener.
Each [`useTournamentLive`](../../../frontend/src/features/live/useTournamentLive.ts)
listener invokes `handleTournamentLiveSignal`. Both
[`PlayerHistoryPage`](../../../frontend/src/pages/PlayerHistoryPage.tsx) and
[`LeaderboardPage`](../../../frontend/src/pages/LeaderboardPage.tsx) retain their
parent hook when delegating to
[`MatchResults`](../../../frontend/src/pages/MatchResultsPage.tsx), which has its
own hook. Direct filtered history is the like-for-like rendering control for
player history; different amounts of displayed content do not explain the result.

Repeated invalidation is not necessarily twice the HTTP work. TanStack's active
refetch defaults can cancel and restart a cached query, while an uncached pending
query can reuse its promise. `open` additionally cancels and erases projections
before refetch. Separately, erasing the table makes `MatchResults` render loading,
unmounting list observers; their remount starts another generation. Neither
mechanism creates another EventSource instance. `resume` already coalesces
same-turn subscribers through its per-client/account drain, with a queued trailing
pass for later returns. That drain must remain unchanged.

## Controlled production-browser protocol

The harness serves the unchanged production assets with native HTTP, gzip and
EventSource, using synthetic public identities from the existing fixture. Four
routes run twice at 320×600, 390×844 and 1280×900, for **24 navigations**. Every
navigation uses a fresh browser context, 100 ms latency, 200,000 B/s download,
93,750 B/s upload and 4× CPU slowdown. Three rounds each contain 24 full match
cards; filtering changes displayed content but not the list payload.

Each navigation runs these phases after its initial requests settle:

1. Delay initial stream headers until epoch-0 content is complete, then open with
   epoch 1. This ensures both delegated subscribers exist before delivery.
2. Emit one ordinary `match` event at epoch 2 and let the refresh settle.
3. Emit another match event with held epoch-3 bodies, disconnect, verify all
   projections disappear, and wait for the browser's native reconnect request.
   Open at epoch 4, then release held bodies after a bounded 400 ms delay.
4. Dispatch persisted `pageshow` at epoch 5 with the same account. Verify one
   session read, fresh results and no new live connection.
5. Hold epoch-6 match reads, change the fixture session to another synthetic user,
   then dispatch persisted `pageshow`. Verify the old projections clear, no old
   epoch reappears after that clearing, the old source closes and a new source
   supplies epoch-7 results. Release obsolete bodies after 400 ms.

The fixture advertises a 200 ms native retry. After readiness, each phase settles
for 700 ms and waits for zero pending non-SSE requests. Browser requests are
attributed to the phase in which they start; server metadata records arrival,
which can occur in a later phase. The held-response assertion covers **all**
server-observed held responses, regardless of arrival phase.

Observation wraps native `fetch` and the existing EventSource listener callbacks,
preserving their calls. Each callback gets a dispatch ID; fetch starts made
synchronously inside it retain that ID. This distinguishes immediate invalidation
from later rendering/remount work without patching application source or query
cache. The wrapper observes the single transport listener, not each inner
subscriber: the two subscriber callbacks are established by source tracing.
MutationObserver records content epochs and empty projections, not physical
screen frames. Browser completion/abort and server close/release are separate
measurements. No response bodies, session tokens or credentials are retained.

## Results

All 24 navigations passed freshness, cancellation, account clearing, source
ownership, console/network, no-overflow and pending-request assertions. See
[samples.csv](samples.csv), [summary.json](summary.json) and
[traces.jsonl](traces.jsonl) for every sample and phase.

The following controlled counts hold across widths and repetitions. `S/C/A`
means browser starts/completions/aborts, per navigation and phase.

| Phase | Direct routes: lists; table S/C/A | Delegated routes: lists; table S/C/A |
| --- | --- | --- |
| Settled match | 3/3/0; 1/1/0 | 6/3/3; 2/1/1 |
| Delayed initial open | 6/3/3; 1/1/0 | 9/3/6; 2/1/1 |
| Native reconnect after empty projection | 3/3/0; 1/1/0 | 3/3/0; 2/1/1 |
| Same-account return | 3/3/0; 1/1/0 | 3/3/0; 1/1/0 |

During the settled match event, all 3/1 direct or 6/2 delegated starts occur
inside **one native callback**; no list/table disappearance occurs. Delayed open
starts 3/1 versus 6/2 in that callback, followed by three list starts after the
loading transition. This separates the duplicate subscriber cost from the
independent list-remount cost. Reconnect begins with empty projections and does
not reproduce the same list cancellation pattern; only the table restarts.

Every navigation creates two EventSource instances: the first survives native
reconnect, and the second belongs to the replacement account. There are three
`/live` HTTP requests in total, and exactly one session read in each return/account
phase. Account-transition list/table counts depend on initial-open timing and
parent composition; they are retained per sample, not treated as a fixed
optimization target. All **279 server-observed held responses** close before body release without a
server finish event. Final visible content is exclusively epoch 7.

## One bounded implementation proposal

Move `useTournamentLive(tournamentId)` from shared `MatchResults` to the direct
`MatchResultsPage` wrapper in the same file. Keep the existing history/global
parent hooks. Each route then owns live authority throughout child loading,
errors and remounts; shared result rendering owns queries and presentation.
Future consumers of shared `MatchResults` must provide a route-level live owner.

Preserve all event targets, synchronous projection erasure, reconnect authority
refresh, account-rooted keys, abort forwarding, private-result late-denial guards,
queued return revalidation, writable score intent and sporting rules. Do not add
a global debounce, skip events, change freshness or redesign loading in this step.

Acceptance needs real-hook route tests with a controlled EventSource: all entry
points, early and delayed initial opening, match/visibility events, child
loading/error/remount, disconnect/reconnect, tournament switch and account change.
Existing route tests mock the hook, so they cannot establish ownership. Retain
fresh-denial and cancelled-denial checks, shared management-read ownership and
the existing return-order tests. Run the frontend ladder and replay production
mobile/desktop evidence; compare starts, aborts, completions and fresh content.
A move can alter initial effect timing, so identical total navigation counts are
not required. Demonstrate one owner and the settled-event reduction, then stop.

## Validation and limits

The production build and fresh attribution build pass and agree with browser
asset hashes. Focused private-result/live/return/cancellation checks pass (51 tests
in seven files). All 11 existing production-browser return-loading/return-order
tests also pass, including frozen overlapping returns at all three widths. Independent read-only review traced ownership and reviewed
the harness; its recommendations strengthened account-clearing and held-response
assertions before the retained run. Final evidence review passed with no blockers,
independently checking all 24 cases, 279 held cancellations and 55 asset hashes.
Full frontend unit/lint suites are not
repeated because no production source or frontend test changed; build includes
TypeScript. Backend/PostgreSQL checks are outside this documentation-only scope.

The harness uses synthetic compressible data, desktop Chrome viewports, a finite
observation window and instrumented callbacks. Persisted `pageshow` is dispatched
explicitly; this does not demonstrate actual BFCache restoration. Existing
return-order browser coverage separately exercises freezing. No backend
membership enforcement, production latency, physical phone, database timing,
SQL cancellation, Caddy delivery or sustained live load claim is made. The
implementation proposal has **not** been applied or benchmarked. Transient
remounts, full-card history payloads and PostgreSQL authorization timing remain
separate work.

## Reproduce

```sh
npm --prefix frontend run build
node docs/performance/bundle.mjs /tmp/subscriber-bundle.json
node docs/performance/subscribers/browser.mjs /tmp/match-subscribers
node docs/performance/subscribers/summarize.mjs /tmp/match-subscribers/trace.json /tmp/match-subscribers
```

Optional `SUBSCRIBER_WIDTH`, `SUBSCRIBER_ROUTE` and `SUBSCRIBER_REPEATS` select a
pilot. Do not run heavy checks concurrently with measurement. The summary retains
source commit, clean frontend status, conditions and asset hashes. Full traces
and screenshots for the retained run live in `/tmp/subscriber-final/`; that
location may be cleaned. Committed JSONL retains API/DOM timelines while omitting
static request rows; the summary retains all asset hashes.
