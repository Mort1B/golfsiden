# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. The next candidate requires the user's instruction to proceed.

## Next candidate — cancel superseded match-result HTTP reads

**Goal:** stop browser transfer/processing of delayed match-list/table requests
whose query generations are already cancelled, using the
[startup investigation](performance/startup/README.md) as the baseline.

**Scope and behavior:** add optional `AbortSignal` parameters to `matchApi.list`
and `matchApi.table`, forward them through the existing HTTP decoder, and pass the
query context signal from the protected `MatchRound` list and `MatchResults` table
consumers. Keep the existing pre/post-load cancellation and late-denial checks.
Include focused transport/private-result tests, production browser evidence and
affected documentation. The ordinary management `MatchSetup` caller retains its
current lifecycle; validate navigation across the shared list key.

Do not change query keys, freshness, retry rules, SSE opening/reconnect refresh,
projection erasure, subscriber fan-out, parent loading composition, API payloads,
backend authorization, schema or scoring. Do not suppress required fresh reads or
change the queued browser-return drain.

**Invariants:** private projections stay erased until fresh authority succeeds;
superseded success or denial cannot overwrite newer authority. Account isolation,
writable score intent, historical handicap snapshots and administrator-managed
teams remain unchanged.

**Validation:** prove both signals reach `fetch`; cover cancellation followed by
replacement success and 401/403/404 denial ordering. Replay cold/warm initial-open,
overlap and reconnect cases at 320/390/1280px, retaining starts, aborts,
completions, bytes and correct fresh content. Re-run existing private-result,
live/return-order regressions and the full frontend ladder. Check account changes,
result-to-management navigation and ordinary uncancelled errors.

**Stop:** publish observable aborts of delayed superseded list/table HTTP reads,
with fresh replacement content and privacy regressions passing. Request-start
counts may stay unchanged; do not claim cancellation of completed responses or
already-started server SQL. If the candidate does not satisfy these gates, record
the limitation and re-bound the plan. Do not begin another performance repair or
the wider security review.

## Later queue

1. **Remaining performance findings:** separately examine duplicate subscriber
   invalidation and transient list remount fetches; reassess full-card history
   reads. Acquire disposable PostgreSQL measurements before any list-authorization
   repair. Preserve authority refresh and fail-closed behavior in every proposal.
2. **Security review:** perform the separately scoped wider application/operational
   review after agreed performance repairs.

No automatic opponents/byes/brackets, team match play, extra holes, new scoring
rules, public match sharing, cold offline launch or background sync is included.
