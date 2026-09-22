# Retained match-list observers behind table readiness

This bounded frontend repair retains current-round `MatchRound` observers during
match-table loading. `ready` defaults to true for existing callers; shared result
routes pass current table readiness to both query enablement and presentation.
The parent renders no round heading while gated and the child returns no DOM.
No initial list read starts before the table is ready. Existing in-flight reads
can finish without being cancelled merely because the table is loading.

Current rounds alone define children. Parent errors, missing rounds and removed
account/tournament/round identities still remove owners. No saved descriptors,
server data copy, previous-authority flag, freshness change or CSS-hidden private
content is introduced. Account keys, runtime decoding, pre/post cancellation checks,
late-denial suppression, abort forwarding, shared management reads and the single
route-level live subscription remain unchanged.

**Verdict: READY WITH KNOWN LIMITATIONS.** The bounded request reduction is
measured below; privacy and freshness checks pass.

## Reproduction and provenance

The baseline is the [remount investigation](../remounts/README.md), whose production
source at `61d53da` is unchanged by the documentation-only parent `e47fd87`.
The candidate is `e47fd87` plus this commit's two production frontend edits.
The unchanged baseline browser/server/observer/summarizer scripts are reused:

```sh
npm --prefix frontend run build
node docs/performance/bundle.mjs docs/performance/gated-observers/bundle.json
node docs/performance/remounts/browser.mjs /tmp/gate-final
node docs/performance/remounts/summarize.mjs /tmp/gate-final/trace.json docs/performance/gated-observers
npm --prefix frontend run test:browser:routes
```

The production comparison uses native synthetic HTTP and SSE, fresh browser
contexts, three rounds with 24 matches each, 100ms latency, 200,000 bytes/second
download, 93,750 bytes/second upload and 4x CPU slowdown. It covers direct results,
player history and global match-only results at 320, 390 and 1280px, each with
settled or held initial lists. These are 18 distinct cases, not repeated identical
samples. No heavy validation commands run concurrently with this replay.

[summary.json](summary.json) retains source/status, browser/runtime, conditions,
build hashes and request counts. [samples.csv](samples.csv) includes both native
fetch starts and Playwright callback phase assignment; [traces.jsonl](traces.jsonl)
retains API/server/DOM timelines. Native dispatch attribution determines which
callback caused a start: delayed Playwright notifications can land in a later
host phase. [bundle.json](bundle.json) verifies a fresh build against emitted
assets. Screenshots and raw trace in `/tmp/gate-final/` are temporary.

## Measured comparison

All 18 candidate cases pass. For each three-round case:

| Native phase | Baseline list starts | Candidate list starts | Outcome |
| --- | ---: | ---: | --- |
| Stream open | 6 | 3 | Three successful current reads; transient table loading no longer cancels three new reads |
| Visibility plus table release | 6 | 3 | Candidate lists finish while gated; reopening starts zero replacements |
| Visibility superseding pending lists plus release | 6 | 3 | Old pending reads still abort; three new reads finish without table-induced cancellation |
| Reconnect | 3 | 3 | Required fresh reads preserved |
| Same-account return | 3 | 3 | Required fresh reads preserved |

Both visibility variants change from three cancelled plus three completed reads
to three completed reads and zero aborts in their new generation. Table-first
and fresh list-first unit cases establish both completion orders; the replay holds
the table while lists finish. This is request-count evidence, not latency evidence.

The replay also confirms 225 obsolete held responses abort before release, 36
required held tables finish, and 18 expected denials fail closed. All cases end
with fresh epoch-10 data, no non-SSE pending requests, unexpected browser/server
errors or horizontal overflow. Stream sharing, old-account clearing and one
session refresh per return/account phase pass unchanged harness assertions.
Denials are table 403 at 320px, rounds 401 at 390px and list 404 at 1280px across
routes and initial modes; the unit suite covers the full dependency/status matrix.

Candidate pending-load cases direct/390 and global/1280 assign three delayed
initial Playwright notifications to `open`; native traces still show exactly three
new open starts. The baseline has its own two such phase-boundary cases. Do not
interpret callback-assigned totals as extra native invalidations. The retained
CSV makes both attribution methods inspectable. All 55 browser asset hashes match
the fresh emitted-build attribution.

## Correctness and freshness

`MatchResultGate.test.tsx` adds 31 tests: cold disabled ownership and zero initial
fetches, table-first/list-first completion, completion followed by 21-second clock
advance, gated private-DOM absence, restricted projections, all three dependency
401/403/404 combinations, late superseded success/denial, scope removal, ordinary
parent failure, and the default-enabled standalone caller. The nine ordering
tests fail against the previous production source, then pass with the repair.

`matchResultGate.browser.ts` adds nine production-browser cases across all routes
and widths. Native SSE triggers visibility invalidation; HTTP fixtures pass through
the real decoder from confirmed concession to a restricted nine-hole card with no
completion metadata or scoring permission. Initial/gated/long-name/populated/
restricted/empty states have screenshots and overflow assertions. Phone cards at
the bottom of the page are checked against navigation clearance. Existing route
checks cover error recovery, shared management reads, logout and frozen returns.

The direct 320px browser case deliberately holds the table for a real 21 seconds
after the restricted list response. Reopening starts another list read under the
unchanged 20-second freshness rule. This is expected: retaining the observer can
mean two completed reads instead of one aborted and one completed. The other
focused cases reopen while fresh. This repair does not promise a universal request
reduction or a latency improvement.

## Validation

- Full frontend ladder: 708 tests in 113 files, TypeScript, ESLint and production
  build passed. Browser-specific strict TypeScript also passed.
- Previous-source negative control: all nine new response-order tests fail;
  restoring the candidate passes all 31 focused tests.
- All 42 production route browser tests passed; the nine gate cases were rerun
  after adding the real freshness deadline, ledger-consistent projection and
  explicit completion counts. This includes separate management, cancellation,
  account, error-recovery and frozen-return regressions.
- The unchanged native replay passes all 18 cases; independent read-only review,
  mobile/desktop screenshot inspection, asset equality and diff checks pass.
- No backend/PostgreSQL checks ran: no affected source, schema or API contracts.

## Limits

Synthetic fixtures and desktop viewport emulation do not measure production
latency, PostgreSQL authorization, SQL cancellation, physical phones, actual
BFCache restoration or sustained SSE load. Restricted-final client decoding and
presentation are exercised; real server filtering is not revalidated in this
frontend-only step. Backend/PostgreSQL ladders are outside scope because those
layers, contracts and schemas are unchanged. Full-card history payloads, database
measurements and the wider security review remain separate work.
