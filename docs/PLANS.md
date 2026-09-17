# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. The next candidate requires the user's instruction to proceed.

## Next candidate — performance baseline

**Goal:** identify evidence-backed performance findings before selecting an
optimization. The existing frontend bundle warning and bounded match-card reads
are investigation candidates, not proof of a user-visible performance defect.

**Scope and behavior:** measure representative populated and long-content
workloads, including mobile page loading and match-card request volume. Record
baseline conditions, timing, bundle contribution and request counts. Classify
confirmed findings as high, medium or low, and define one bounded repair candidate
from the evidence. This step is investigation and documentation only.

**Invariants:** preserve private-read authorization, scoring correctness, account
isolation, historical snapshots, offline drafts and administrator-managed teams.
Do not weaken correctness checks or change runtime behavior to improve a metric.

**Validation:** retain reproducible commands and workload descriptions; distinguish
cold/warm runs and synthetic/local results from production evidence. Repeat key
measurements sufficiently to expose variability. Review findings against current
source and record unavailable measurements and their blockers.

**Stop:** publish the baseline and prioritized findings with a bounded next-step
proposal. Do not implement optimizations or expand into the security review.

## Later queue

1. **Performance repairs:** select one bounded change from measured findings.
2. **Security review:** perform the separately scoped wider application/operational
   review after the performance investigation and agreed repairs.

No automatic opponents/byes/brackets, team match play, extra holes, new scoring
rules, public match sharing, cold offline launch or background sync is included.
