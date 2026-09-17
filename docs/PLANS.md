# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. The completed route-level splitting is documented in
[LatestExplanation.md](LatestExplanation.md) and the
[comparison report](performance/route-splitting/README.md).

## Next candidate — investigate match-list startup amplification

**Goal:** identify which repeated match-list HTTP reads are necessary authority
refreshes and which, if any, are superseded work that can safely be avoided.

**Scope and behavior:** trace the existing match-result startup waterfall and
query/SSE lifecycle. Reproduce initial stream opening, delayed reads and reconnect
with request timing and cancellation evidence. Produce one bounded repair
proposal only if the evidence supports it; this candidate is investigation and
documentation only. Do not change runtime fetching, authorization, projection
clearing, query staleness, scoring or database code in this step.

**Invariants:** private projections remain erased on relevant visibility and
connection transitions until fresh authority succeeds. Superseded success or
failure cannot restore old private data or erase newer authorized data. Writable
score intent and account isolation stay separate from read-only result caches.
Preserve historical handicaps and administrator-managed teams.

**Validation:** repeat relevant production-build browser workloads at mobile and
desktop widths. Retain request start/finish/abort evidence, final correct content
and error counts; distinguish cold/warm and initial/reconnect cases. Review any
proposed cancellation/coalescing boundary against existing denial and return-order
regressions. State the synthetic API/PostgreSQL and observation-window limits.

**Stop:** publish evidence and one precisely bounded next proposal, or record that
no safe reduction is established. Do not implement the proposal or begin the
wider security review without a subsequent instruction.

## Later queue

1. **Remaining performance findings:** acquire disposable PostgreSQL measurements
   before choosing any list-authorization repair; separately reassess full-card
   history reads after the startup investigation.
2. **Security review:** perform the separately scoped wider application/operational
   review after agreed performance repairs.

No automatic opponents/byes/brackets, team match play, extra holes, new scoring
rules, public match sharing, cold offline launch or background sync is included.
