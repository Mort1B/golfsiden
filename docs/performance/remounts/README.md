# Transient match-list remount investigation — 2026-09-22

**READY WITH KNOWN LIMITATIONS.** The remaining extra list generation comes from
removing list observers while the table reloads. Evidence supports a bounded
proposal to retain disabled observers during table-only pending state, with all
private presentation gated. The benefit is conditional on response ordering and
the existing freshness window; no repair is implemented here.

Application source is unchanged at `61d53da12c3ccb26e70af8a4de4d65410d8a62b9`.
This step adds documentation and observation/model scripts only. The earlier
[ownership comparison](../live-ownership/README.md) remains the retained baseline;
its production assets match this investigation's fresh build and browser hashes.

## Cause and boundaries

The source-supported sequence is:

1. [`handleTournamentLiveSignal`](../../../frontend/src/api/liveInvalidation.ts)
   cancels and erases private projections for open/visibility, then invalidates
   active queries. Lists and table start the required authoritative refresh.
2. [`MatchResults`](../../../frontend/src/pages/MatchResultsPage.tsx) returns its
   loading branch while table data is absent. Its `MatchRound` children disappear.
3. Each [`MatchRound`](../../../frontend/src/features/matchPlay/MatchRound.tsx)
   owns a protected list query observer. Removing the last observer cancels the
   consumed signal, including the refresh that just started.
4. Fresh table data restores the children; their remount starts a replacement
   list generation. The required refresh plus remount therefore starts six reads
   for three rounds, of which three abort.

The stream has one route owner after the previous repair. A native reconnect
following an already-cleared page begins with no list observers, so it does not
have the same immediate cancellation sequence. Ordinary match updates keep table
data and list components present. Neither duplicate subscriptions nor native
connection count explains the table-loading remounts.

The browser records native callback IDs, fetch starts/aborts, and replacement DOM
list-node IDs. These observations corroborate the source trace; they do not
inspect React fibers or patch TanStack's query cache. An absent `.match-list`
node alone does not prove a query observer was removed: source ownership and the
last-observer cancellation path establish that part of the explanation.

## Production-browser protocol

There are **18 distinct cases**: direct results, delegated player history and
global match-only results at 320×600, 390×844 and 1280×900, each with either settled
or pending initial lists. The `repeat` field is a compatibility ordinal for those
two modes, not a second identical run. Each case uses a fresh context and the
same three-round/24-matches-per-round fixture as the ownership baseline. History
shows three cards and one row; full views show 72 cards and 48 rows. Payloads
remain full round lists even for filtered history.

The fixture serves native HTTP/gzip/SSE with 100 ms latency, 200,000 B/s download,
93,750 B/s upload and 4× CPU slowdown. It advertises a 200 ms SSE retry. The browser
waits 700 ms after readiness and checks zero pending non-SSE requests. Phases:

| Phase | Controlled condition |
| --- | --- |
| Initial/open | Settle initial content or hold all three old lists; open at epoch 1, then release obsolete bodies. |
| Visibility/table release | Emit visibility at epoch 2, holding only the table. Observe an empty page and cancelled lists; release the table after 400 ms, then require fresh results. |
| Pending visibility | Hold three match-event lists at epoch 3, then emit visibility at epoch 4 with its table held. Release all held bodies after clearing; require only epoch 4. |
| Disconnect/reconnect | Hold epoch-5 lists/table, disconnect and require empty projections. Allow native reconnect at epoch 6, release old bodies, and require fresh content. |
| Return/account | Persisted `pageshow` refreshes the same account at epoch 7. Hold epoch-8 reads, switch the synthetic session, dispatch return and require cleared old state plus epoch 9. |
| Denial/recovery | After visibility clearing, deny table with 403 at 320px, rounds with 401 at 390px, or one list with 404 at 1280px. Require no private content alongside/after the rendered denial, then restore access and require epoch 10. |

This is a selected dependency/status matrix, not every status for every endpoint
at every width. Visibility precedes each denial, so the browser cases establish
non-repopulation and recovery. The unchanged route tests separately establish
that fresh match-triggered 401/403/404 denials themselves erase sibling projections.
Expected HTTP denial responses are recorded separately; other HTTP failures,
page errors and unexpected console errors fail the run. Matching resource-error
console lines for those denial statuses are expected, not silently treated as a
clean network.

All obsolete held responses must close before release without finishing. The two
held *current* tables at epochs 2/4 must finish successfully. Playwright request
callbacks determine browser-row phase labels; server arrivals and native fetch
dispatch have independent timestamps. Native fetch starts are also retained per
phase to expose notification-delivery races.
DOM epochs reject older content after the checked open, visibility, disconnect
and account-clear transitions, including recorded transient mutations. The first
rendered denial is also included in the transient content check.

## Browser findings

All 18 cases pass. Open and both visibility paths reproduce the independent
remount cost. For a populated visibility event, the native callback starts three
list reads and one table read. The table is deliberately held; all three list
reads abort while the page has no list nodes. Releasing the table starts three
new list reads and creates replacement DOM nodes. Their content is fresh.

Counts per three-round case, `S/C/A` = Playwright-observed starts/completions/aborts:

| Phase | Lists S/C/A | Table S/C/A |
| --- | --- | --- |
| Initial stream open observation phase | 6/3/3 or 9/3/6 | 1/1/0 |
| Visibility while current table is held | 3/0/3 | 1/1/0 |
| Release current table | 3/3/0 | 0/0/0 |
| Visibility superseding pending old lists | 3/0/3 | 1/1/0 |
| Release replacement table | 3/3/0 | 0/0/0 |
| Reconnect after projections cleared | 3/3/0 | 1/1/0 |

Native fetch instrumentation records exactly six new list starts in every open
phase. In two pending-initial samples (global 320px and direct 390px), Playwright
delivers the three initial request notifications only after the host phase changes
to open; their earlier native fetch and server-arrival timestamps identify them
as initial work. Thus 9/3/6 in that observation phase is not a third new open
refresh. The CSV/summary retain both native and Playwright counts rather than
silently reassigning events or inferring extra observers.

Table completion is attributed to its start phase even though its body is released
in the following phase. Pending initial lists and held match/account requests
also abort, independently of the three refreshes cancelled by each remount.
All **225 obsolete held responses** abort before release; all **36 required held
tables** finish, and all 18 expected denials are recorded. Final content is epoch 10; source replacement, account clearing and return session
checks pass. See [samples.csv](samples.csv), [summary.json](summary.json) and
[traces.jsonl](traces.jsonl) for the full timings and every request.

## Installed-library probe and important limit

[`observer-probe.mjs`](observer-probe.mjs) is an **isolated model**, not a production
change or proof that a UI repair works. It uses the installed `QueryClient` and
`QueryObserver`, the same cancel/clear/invalidate order, a single synthetic list,
and production `staleTime: 20_000`. It compares removing the observer with leaving
it subscribed but setting `enabled: false` during table pending. A logical clock
advance tests freshness without a 21-second sleep. Both ownership policies are
applied before the modeled refresh resolves, isolating the observed unmount race
rather than every possible scheduling order. No browser or server is involved.

| Ownership / response order | Starts | Aborts | Completions |
| --- | ---: | ---: | ---: |
| Remove observer, any model order | 2 | 1 | 1 |
| Disabled observer, table first | 1 | 0 | 1 |
| Disabled observer, list first and still fresh | 1 | 0 | 1 |
| Disabled observer, list first then 21-second delay | 2 | 0 | 2 |

A disabled cold observer starts no read until enabled. Disabling a subscribed
observer does not cancel a refresh already running. However, false→true can
refetch completed stale data: production freshness is 20 seconds, not zero or
infinite. A prolonged table hold can therefore leave completed hidden work and
still require a second read. The retained [probe result](observer-probe.json)
asserts these counts. The proposal must preserve this freshness behavior rather
than increasing stale time to manufacture a reduction.

## One bounded implementation proposal

Retain result-only `MatchRound` observers during **table-only pending** state,
deriving their identities exclusively from current rounds data. Pass an explicit
readiness option from `MatchResults` that controls query enablement and all private
presentation. When not ready, the list component still calls its query hook but
returns no private DOM. Keep its query disabled until current rounds/table are
ready; an existing in-flight refresh may finish while gated. Parent errors,
missing rounds, round removal and scope changes must still remove old owners.

This is a resource-lifetime decision, not authorization. Do not keep copies of
rounds, names, cards or prior authority, and do not add an admission-history flag.
Round headings, links, empty/error messages and card markup must also stay absent
while the parent loading view is shown; hiding stale markup with CSS is insufficient.
Other `MatchRound` callers keep their existing behavior by default. Preserve
one route-owned live subscription, cancellation/denial handling, account-rooted
keys, shared management ownership, queued returns and all sporting invariants.

Required implementation tests include no initial list request before table success;
table-first and list-first completion within/beyond the freshness window; old
response cancellation; fresh rounds/table/list denial; actual restricted-final
payload transitions; disconnect/return; account/tournament/round changes; and
other callers/shared management readers. Validate both request work and absence
of private content while gated. Stop after this bounded repair and comparison,
or record insufficient benefit. Do not add lifecycle history state, change
freshness or redesign invalidation to force a universal three-request result.

## Validation, limits and reproduction

Build and fresh attribution passed with all 55 asset hashes matching the retained
ownership build. All 79 focused tests, six observer models plus cold gate, 18
production-browser investigation cases and 11 browser-return regressions passed.
Independent source/harness/proposal and final evidence reviews passed with no
blockers. The audit checked all cases, held-response outcomes, native/browser
count attribution, DOM replacement, denials, account clearing and build hashes.
Representative mobile/desktop screenshots, local links, script syntax and diff
checks passed. Frontend/backend production source and dependencies are unchanged;
full frontend unit/lint and backend/PostgreSQL ladders are not repeated for this
documentation/harness-only step. Build includes TypeScript compilation.

This finite synthetic run does not verify actual hidden-final server filtering,
backend membership enforcement, SQL cancellation, PostgreSQL timing, production
latency, physical phones, actual BFCache restoration or sustained SSE load.
Explicit persisted `pageshow` and desktop viewport sizes are controlled probes;
existing frozen-return browser coverage provides a separate lifecycle regression.
Instrumentation can affect scheduling and DOM mutations are not physical frames.
The history denial view also displays two generic error panels in the inspected
320px screenshot; no private results remain. That existing presentation duplication
is outside the proposed observer repair.

No production speedup or universal request reduction is established for the
unimplemented proposal. Full-card payload/database/security work remains separate.

```sh
npm --prefix frontend run build
node docs/performance/bundle.mjs /tmp/remount-bundle.json
node docs/performance/remounts/observer-probe.mjs /tmp/remount-probe.json
node docs/performance/remounts/browser.mjs /tmp/match-remounts
node docs/performance/remounts/summarize.mjs /tmp/match-remounts/trace.json /tmp/match-remounts
```

Optional `REMOUNT_WIDTH`, `REMOUNT_ROUTE` and `REMOUNT_INITIAL` select a pilot.
Do not run heavy checks during measurement. The summary retains source/build
provenance; the JSONL retains API/DOM timelines, omitting static request rows.
Raw trace and screenshots from this run are in `/tmp/remount-final/` and may be
cleaned. [bundle.json](bundle.json) records fresh attribution.
