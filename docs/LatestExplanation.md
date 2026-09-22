# Cancel superseded match-result HTTP reads

**Completed — READY WITH KNOWN LIMITATIONS.** Protected match-list/table reads now
forward their existing query signal into `fetch`. Obsolete delayed HTTP reads
abort while independent generation/denial guards and required authority refreshes
remain intact. The implementation is four forwarding edits in three production
files; no backend, schema, scoring, authorization, dependency or style change.

The [comparison report](performance/cancellation/README.md) retains all 48 replay
samples, native HTTP/SSE and DOM timelines, observed body bytes and asset hashes.
The unchanged workload covers cold/warm initial open, late open, overlapping reads
and reconnect at 320/390/1280px. All samples end with 72 fresh cards and 48 fresh
table rows, no transient old epoch after the checked transitions, and no pending
ordinary requests or unexpected errors.

The replay records **264 list aborts and 12 table aborts**. All **84 deliberately
held old responses** close before their bodies are released; all completed in the
baseline. For the controlled overlap case, only three lists finish per navigation,
compared with nine before, while fresh content remains correct. Browser-reported
compressed list-body totals fall from 43,822 to 14,607 bytes in that case. Aborted
resource timing can omit partial transport, so this is not an exact wire-savings
claim or a claim that server SQL was cancelled.

Request starts remain governed by the existing lifecycle: 480 list requests start
in this run versus 468 in the earlier sample, due to initial stream timing. The
repair stops superseded work instead of suppressing required refreshes. Initial
opening still erases private projections and requests fresh authority; reconnect
and queued browser-return behavior are unchanged.

Management keeps its existing ordinary consumer. A protected-origin request stays
active while another observer needs the key, and can abort when its last observer
leaves. A management-origin pending request reused by results retains its original
transport behavior. Optional adapter arguments preserve that compatibility.

## Validation and limits

- Full frontend ladder passed: **649 tests / 111 files**, typecheck, lint and build.
  Browser-specific TypeScript and lint checks passed.
- 23 new unit tests cover transport signals, omitted signals, independent guards
  against superseded success/401/403/404, fresh denial ordering, ordinary errors,
  remaining/final observers and management/protected request origins.
- **33 production browser tests passed**, including six new cases for both pending
  management/result navigation directions at all three widths, logout/account
  replacement and recoverable list/table errors. Existing return-order, offline
  scoring guards and chunk recovery also pass.
- **48 native HTTP replay navigations passed**. All 600 protected list/table calls
  receive a signal; 324 complete and 276 abort. Representative mobile/desktop
  management and results screenshots were inspected.
- Fresh attribution output matches emitted production assets and the browser
  hashes. Measurements identify the modified worktree on parent `42d813d`, with
  its source status recorded explicitly, rather than claiming the parent is the
  candidate build.
- Independent source/test review found no production blocker and identified a
  cumulative browser-abort assertion; it now requires a new abort after each
  navigation/logout baseline. Final retained-evidence review passed with no
  blockers, independently confirming request totals, held-response cancellation,
  asset hashes and documented limits. Artifact/link consistency, script syntax
  and diff checks passed.
- No backend/PostgreSQL ladder ran because those layers and HTTP payload contracts
  did not change. No database or production deployment performance is claimed.

The measurements use synthetic, highly compressible data, optimistic immutable
caching, throttled desktop Chrome and shortened SSE retry timing. They do not
validate backend membership/hidden-final enforcement, physical phones, Caddy,
sustained live load or cancellation of already-started SQL. Observation can affect
scheduling and only covers a finite settling window.

The step is closed. The plan proposes an investigation of duplicate invalidation
by shared-stream subscribers; that investigation and further repairs are unstarted.
