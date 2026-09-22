# One live owner per match-results route

**Completed — READY WITH KNOWN LIMITATIONS.** Direct match results now own their
live subscription in the route wrapper. Match-only history and global results
keep their existing parent subscriptions; shared `MatchResults` owns queries and
rendering. This removes duplicate invalidation while keeping live authority
through child loading, errors and remounts.

The production edit moves one hook within `MatchResultsPage.tsx`. No event fan-out,
query key, retry/freshness policy, projection clearing, cancellation/denial guard,
return drain, API, backend, schema, dependency, style or sporting rule changes.
New consumers of shared `MatchResults` must supply a route-level live owner.

The [comparison report](performance/live-ownership/README.md) replays the unchanged
24-case investigation harness at 320, 390 and 1280px. For one settled match event,
delegated views now start three lists and one table, all completing, compared
with six lists and two tables before, half of which were cancelled. Direct views
retain their three-plus-one refresh. Native callback attribution separates this
reduction from later loading-state remounts.

The separate list-remount cost remains: delayed opening starts six list reads,
three cancelled when table loading temporarily unmounts their observers. Required
refreshes, fail-closed disconnect/reconnect and account-clearing behavior remain.
Same-account return still makes one session read; the shared return drain and its
queued follow-up are unchanged. Account replacement closes the old live source;
native reconnect reuses the current one.

## Validation and limits

- Full frontend ladder passed: **677 tests across 112 files**, typecheck, lint
  and production build. Browser TypeScript compilation also passed.
- **28 new route tests** use the real live hook, source-sharing and invalidation.
  They cover four entry points, early/settled opening, held match refresh,
  visibility clearing, child loading/error/empty/remount recovery, fresh
  401/403/404 denial, tournament switching, old-source suppression and logout/new
  account. The previous source fails the two delegated regression cases while
  both direct controls pass; the repaired source passes all 28.
- **33 production-browser tests passed**, including shared management reads,
  logout cancellation, route recovery and frozen overlapping browser returns.
- **24 comparison navigations passed** using the unchanged harness. All 192 held
  obsolete responses aborted before release, with fresh final content and no
  unexpected errors or overflow. Mobile/desktop screenshots were inspected.
  Fresh bundle attribution and 55 browser asset hashes match. The candidate is a
  modified frontend on parent `36248ba`, with status recorded explicitly.
- Independent source/test and final evidence reviews passed with no blockers.
  The final audit verified all 24 cases, 192 held cancellations, native dispatch
  counts, baseline comparison, account clearing and 55 matching asset hashes.
  Script syntax, local links and diff checks passed.
- Backend/PostgreSQL ladders did not run because these layers and HTTP contracts
  are unchanged. No real database performance is claimed.

The replay uses synthetic compressible data, desktop Chrome viewports, explicit
persisted `pageshow`, shortened SSE retry and finite DOM observation. It establishes
controlled request-work reduction, not production latency, actual BFCache,
physical-phone, server authorization, SQL cancellation or sustained-load results.
Remaining remount work, history payloads and PostgreSQL authorization timing are
separate. The next plan step is investigation only.
