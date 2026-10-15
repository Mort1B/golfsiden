# Latest iteration — Fantasy result and refresh measurements

A1t measures the existing application without changing production code. Two
disposable tournaments contain one/five rounds, eight golfers, four scramble
teams per round, 18 holes and two managers. Real Chrome, the unchanged API and
PostgreSQL exercise editable saves first, then populated settled-result reads.

A scoped first-round result uses 25 tracked statements at both sizes, but its
canonical bulk query returns four/twenty owners from all non-draft rounds. Mean
bulk SQL time rises from 2.72 to 10.08ms, accounting for 83–95% of tracked SQL
execution. Warm complete HTTP medians with tracking off are 21.28/44.98ms.
SQL server time, application/network time and observer controls remain separate;
no production speedup, capacity or projection-only interpretation is claimed.

The first populated GET materializes one/five Fantasy windows and two/ten locked
selections, including eight carried lineups in the five-round fixture. It is
excluded from steady timing. All subsequent result bodies and materialization
counts remain identical. Six controlled probes directly observe lock waiting;
their HTTP times include diagnostic control overhead, not only the injected wait.

Every fully observed save completes seven/eleven GETs and cancels four/eight
more, consistent with overlapping acknowledgment/SSE invalidation. All measured
saves create exactly one accepted receipt; full observers also prove one PUT.
Both three-second idle windows start no API request. Save timings end at the
rendered authoritative lineup, and counts extend through enabled editing, a
post-ready one-second tail and a quiet barrier. That refined boundary is recorded
as a design deviation, rather than presented as immediate acknowledgment timing.

The unchanged API build, 39 migrations, frontend production build, diagnostic
assertions, browser/privacy gates and owned cleanup pass. Source regression
ladders are unchanged and were not rerun for this probe/documentation-only step.
Independent review and static checks are recorded in the
[measurement report](validation/fantasy-measurement-2026-10-15/README.md), alongside
raw sanitized samples, reproducible probes, pilot repairs and exact limitations.

The plan proposes one separately approved candidate: scope canonical sporting
fact loading for a round result while retaining authorization, all-round locks,
lazy finalization, visibility and session/commit boundaries. It does not address
the all-draft save workload, which has no canonical sporting owners. No runtime
optimization, refresh redesign, schema, dependency or hosted work began here.
