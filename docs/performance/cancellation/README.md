# Cancel superseded match-result HTTP reads

**READY WITH KNOWN LIMITATIONS.** All 48 replay cases pass; controlled superseded
HTTP reads abort before their held bodies are released.

Protected match-list/table consumers now pass their existing query signal through
`matchApi.list`/`table` to `fetch`. The private loader's generation checks,
late-denial suppression, account-scoped keys, retry rules and required authority
refreshes are unchanged. Production changes are four forwarding edits in three
files; there is no API/schema, scoring, dependency or style change.

The [startup investigation](../startup/README.md) is the preserved baseline.
This comparison uses exactly its 48-navigation harness and fixtures: two cold/warm
pairs for natural opening, late opening, overlapping initial reads and reconnect,
at 320×600, 390×844 and 1280×900. Each page must end with 72 fresh cards and 48
fresh table rows. Existing assertions reject transient old epochs after stream
open/disconnect and after complete fresh content.

## Measured outcomes

All 48 samples passed fresh-content, transient-epoch, zero-pending-request,
console/network/fixture error and overflow assertions. All 600 protected
list/table fetch calls in this replay received a signal.

| Whole replay | Before | After |
| --- | ---: | ---: |
| List requests started | 468 | 480 |
| List requests completed | 468 | 216 |
| List requests aborted | 0 | 264 |
| Table requests started | 120 | 120 |
| Table requests completed | 120 | 108 |
| Table requests aborted | 0 | 12 |
| Reported list compressed-body bytes | 2,278,728 | 1,051,728 |
| Reported table compressed-body bytes | 58,920 | 53,028 |

Starts vary with the initial stream race; the candidate starts 12 more list reads
in this finite run. The change does not suppress that lifecycle work. Every
started ordinary list/table request either finishes or is recorded as aborted.

Per three-round navigation, across 12 samples of each case:

| Case | Before completed lists | After completed / aborted lists | Before → after reported list body bytes | After completed / aborted tables |
| --- | ---: | ---: | ---: | ---: |
| Natural | 3 or 9 | 3 / 0 or 6 | 14,607 or 43,821 → 14,607 B | 2 / 0 |
| Late open | 9 | 6 / 3 | 43,822 → 29,215 B | 2 / 0 |
| Overlap | 9 | 3 / 6 | 43,822 → 14,607 B | 2 / 0 |
| Reconnect including startup and match event | 9 or 15 | 6 / 3 or 9 | 43,822 or 73,036 → 29,215 B | 3 / 1 |

The **84 deliberately held old responses** (72 lists and 12 tables) all close
before the harness releases their bodies, with no server `finish` event. Baseline
held responses all completed. Browser request failures independently record the
aborts. Fresh epoch-1/epoch-2 content still populates after initial opening and
reconnection, with no observed stale repaint. This satisfies the planned stop
condition without relying on a lower request-start count.

Required refreshes remain visible: the late-open case completes its original
three lists plus three fresh lists, and reconnect completes fresh lists after the
error/open transition. The 700 ms settling window and synthetic fixtures do not
establish production latency or behavior beyond that window.


Request starts, aborted requests and completed bodies are separate measures.
Resource timing body/transfer fields are retained for every recorded API resource;
an aborted resource may report zero even if some transport work has already
occurred. The reported body sums are browser-reported sizes, not an exact measure
of bytes avoided on the wire. Server socket handoff is not browser completion.
The controlled held-response cases establish cancellation before releasing those
old bodies; no claim is made about cancelling already-started server SQL.

## Provenance and reproduction

The baseline application is `d047830f51f051113701d9fb6e45e4ed9eb6a507`.
Its documentation-only successor `42d813d6e3d6007b5efda304457440e332ce40c9`
is the parent of this repair. Candidate measurements were made before commit;
`sourceCommit` names that parent and `sourceStatus` records the changed frontend.
The actual candidate is identified by the asset hashes in [bundle.json](bundle.json)
and [summary.json](summary.json), checked against the emitted production build.

```bash
npm --prefix frontend run build
node docs/performance/bundle.mjs /tmp/cancellation-bundle.json
node docs/performance/startup/browser.mjs /tmp/match-cancellation
node docs/performance/startup/summarize.mjs /tmp/match-cancellation/trace.json /tmp/match-cancellation
npm --prefix frontend run test:browser:routes
```

Use distinct output directories to preserve another run. No builds or heavy checks
ran alongside the retained request trace. [samples.csv](samples.csv) contains all
48 per-navigation totals; [traces.jsonl](traces.jsonl) retains all API and DOM
timelines. Full raw output/screenshots for this run are in
`/tmp/match-cancellation-final/`; temporary files may not survive cleanup.

## Ownership and behavior acceptance

A protected-origin read stays active while another observer needs the same key,
including management. Its final observer leaving can cancel it. Management's
ordinary `MatchSetup` consumer still starts reads without a signal; attaching a
protected observer to an already-running management request does not retrofit
transport cancellation. The optional adapter arguments preserve that behavior.
This repair does not claim that every list read in every workspace is abortable.

- Full frontend ladder passed: **649 tests across 111 files**, typecheck, lint and
  production build. Browser TypeScript and focused browser-file lint passed.
- 23 new unit tests cover exact signal forwarding, omitted signals, superseded
  success/401/403/404 responses after fresh success, fresh denials after
  cancellation, ordinary 503 errors, remaining/final observers and both read origins.
  Noncooperative transports deliberately resolve after cancellation to prove that
  the independent generation guard is retained.
- **33 production browser checks passed**: the prior 27 loading/return-order,
  scoring-guard and chunk-recovery cases plus six new checks. They cover pending
  results→management and management→results navigation at all three widths,
  logout/account replacement and uncancelled list/table error recovery. Navigation
  and logout assertions require a new abort after the action's captured baseline.
- Production browser mocks verify UI ownership/interaction, while the 48-case
  harness uses native HTTP/SSE to establish transport outcomes. Representative
  mobile/desktop management and result screenshots were inspected.
- Independent read-only review checked production forwarding and ownership tests.
  Its browser assertion finding was fixed before the complete browser run.
  Final evidence review is recorded in the latest explanation.
- Script syntax, artifact consistency and diff checks passed. Backend/PostgreSQL
  ladders were not run: no server, schema or HTTP payload contract changed.

## Limits and deferred work

These are synthetic traces with 100 ms latency, 200,000 B/s download, 4× CPU
slowdown, immutable asset caching and a shortened 200 ms native SSE retry.
Fixtures compress unusually well. Observational wrappers can affect scheduling;
the finite settling window is not a field latency benchmark. No production Caddy,
physical phone, sustained live load, server authorization or database timing is
validated. Ordinary browser-return freshness and projection erasure remain intact.

Duplicate subscriber invalidations, transient remount fetch starts, full-card
history payloads and backend authorization costs remain separate work. The
candidate fixes transport propagation only; it does not eliminate every repeated
request or establish a production speedup.
