# Performance baseline before optimization

**The investigation is complete — READY WITH KNOWN LIMITATIONS.** It measures
the production frontend build and synthetic populated/long-content browser
workloads, traces match-list database work, and proposes one bounded next repair.
No runtime, dependency, API, schema, scoring or authorization behavior changed.

The [baseline report](performance/README.md) retains reproducible scripts, asset
hashes, every timing sample, request/byte counts, workload definitions and explicit
measurement limits. Source examined was `7fe6bcb1ffd6751d6621aba19293a801a885e81f`.

The production frontend has one 782,064-byte JavaScript entry chunk, 222,606 bytes
gzipped. Login loads it too. Under the fixed 100 ms/1.6 Mbps/4× CPU Chrome profile,
390px median cold readiness was 1.73 seconds for login, 2.37 seconds for 12 match
cards and 2.50 seconds for 72 cards across three rounds. These are synthetic
navigation metrics, not field Core Web Vitals or production service-level claims.

For example, selecting one player's history displays three cards but still fetches
and decodes all 72 cards. The long workload issued three to nine list requests;
the stream-opening control consistently issued three when opening was delayed
beyond the observation window. Initial stream recovery must continue to clear
private projections and refresh authority. This finding does not authorize
removing freshness checks or accepting stale results.

Backend source tracing found `5 + 17M` SELECTs per fully authorized administrator
list request, including HTTP authentication. Privileged opponent checks also
materialize `2MP` snapshot owner IDs (M matches, P players); 100 two-player matches
mean 1,705 SELECTs and 40,000 authorization rows. These are source-derived counts.
Real SQL timing, pool contention and production impact were not measured.

The first proposed repair is route-level JavaScript splitting. It can address
the measured startup payload while keeping shared auth/offline providers intact.
The plan defines comparison and regression gates, including chunk-load recovery.
Database authorization reuse, compact/player-scoped listings and stream-refresh
coalescing remain separate candidates. No high-severity production defect was
established; findings are prioritized medium/low with their evidence limits.

## Validation and review

- `npm --prefix frontend run build` passed with the existing bundle warning.
  A fresh in-memory attribution build matched every emitted file byte-for-byte;
  browser asset SHA-256 hashes matched the recorded bundle.
- The reviewed browser run passed 62 navigations: five cold/warm pairs per 390px
  workload and three pairs for long content at both 320 and 1280 pixels. A separate
  six-navigation delayed-stream control passed. Every final table/card count was
  correct, every sample ended with zero pending non-SSE requests, and console,
  page errors, HTTP failures, unexpected fixture requests and horizontal overflow
  checks were clear.
- Mobile and desktop screenshots were inspected; long names wrap. This is a
  performance/layout sample, not a new interaction/accessibility acceptance suite.
  Full request artifacts and screenshots are in `/tmp/golf-performance-final/`
  and `/tmp/golf-performance-delayed-stream/`; essential samples and conditions
  are retained under `docs/performance/`.
- Read-only specialist review checked the SQL trace and measurement design.
  Settled-content/pending-request assertions and build-provenance checks were
  added after its findings, then the baseline was rerun. Highly compressible
  fixtures, optimistic warm static caching, desktop throttling and custom timing
  metrics are explicitly documented. Final read-only review independently checked
  all 68 retained samples and the SQL trace; no actionable findings remained.
- Script syntax checks and `git diff --check` passed. The production build includes
  TypeScript compilation. Frontend unit/lint and backend/database ladders were not
  rerun: only documentation and measurement scripts changed, with no product code
  or dependency changes.

Real API/PostgreSQL measurements were unavailable: no local PostgreSQL binaries
were found, Docker socket access was denied, and passwordless sudo was unavailable.
No shared or production database was used. Physical phones, production delivery,
hidden-final/revocation behavior, mixed formats and sustained live-event load are
outside this measurement. Browser fixtures cannot validate backend privacy or
authorization. The next step remains unstarted pending the user's instruction.
