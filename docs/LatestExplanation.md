# Duplicate match-result live invalidation investigation

**Completed — READY WITH KNOWN LIMITATIONS.** Delegated match-only player history
and global results have two live subscribers on one shared EventSource. A settled
match event therefore starts six list reads and two table reads, compared with
three and one on direct results. The obsolete half aborts. This step documents the
cause and one bounded repair; it changes no production application behavior.

The [investigation report](performance/subscribers/README.md) retains the harness,
per-phase counts, native HTTP/SSE and transient DOM timelines, source commit and
asset hashes. It compares direct full results, direct filtered history, delegated
history and global results at 320, 390 and 1280px, twice each: **24 navigations**.
Direct filtered history controls for the smaller displayed player selection.

Source tracing establishes two hook owners on delegated routes. Native callback
instrumentation shows the extra match-event fetches start synchronously within
one transport callback. There is no second connection or intervening content
unmount in that phase. Opening the stream also causes a separate table-loading
transition that unmounts and remounts lists: direct pages start six list reads,
delegated pages nine. That remaining remount cost is outside the proposed repair.

Every replay ends with fresh epoch-7 content, no older epoch after the checked
open/disconnect/account-clearing transitions, and no pending non-SSE requests,
unexpected errors or overflow. All 279 server-observed held responses close before
body release without finishing. Native reconnect reuses the first EventSource
instance; account change closes it and creates a replacement. Same-account and
changed-account returns each perform one session read, confirming the existing
return drain already coalesces subscriber callbacks.

The next candidate moves the live hook from shared `MatchResults` into the direct
`MatchResultsPage` wrapper. History and global results keep their existing parent
hooks. This gives each route one live owner throughout loading/error/remounts,
without changing event fan-out, query freshness, projection erasure, cancellation,
private-result denial guards or queued return ordering. The proposal is not
implemented; initial effect timing and all entry points require regression tests.

## Validation and limits

- Production build passed, including TypeScript compilation. A fresh attribution
  build and the observed browser assets agree; frontend source is clean at
  `72f8aa968232095b2d43f76c15a3fa551f0ca3bf`.
- **51 focused tests across seven files passed:** private-result denial,
  invalidation targets, queued return drain, shared transport, match mutation
  separation, list/table cancellation and shared management-read ownership.
- **24 production-browser investigation navigations passed**, covering delayed
  open, ordinary match, held match/error/native reconnect, same-account return and
  changed-account return. Mobile and desktop screenshots were inspected.
- **11 existing production-browser return-loading/return-order tests passed**,
  including frozen overlapping returns at mobile and desktop widths. Independent
  source/harness and final retained-evidence review passed with no blockers. The
  final review independently verified all 24 cases, 279 held cancellations,
  synchronous dispatch counts, account clearing and 55 matching asset hashes.
- Script syntax, document links and diff checks passed. Full frontend unit/lint
  suites were not repeated for a documentation/harness-only change. Backend and
  PostgreSQL ladders were not run because those layers are unchanged and real
  database timing is outside this step.

Measurements use synthetic data, instrumented native callbacks, throttled desktop
Chrome and a finite settling window. DOM observations are not physical display
frames; explicit persisted `pageshow` is not an actual BFCache restoration. No
production latency, database authorization/cancellation, physical phone, Caddy or
sustained live-load improvement is established. The existing return-order suite
separately exercises a frozen browser. The proposed repair has not been benchmarked.

The investigation is closed; the ownership repair and later performance/security
work remain unstarted.
