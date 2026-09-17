# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. The next bounded candidate below awaits an implementation instruction.

## Next candidate — investigate the intermittent offline-return browser check

**Evidence:** the existing `returnLoading.browser.ts` offline/frozen-page return
case timed out before read-only display in one full run; three unchanged isolated
repeats passed. Its cached enabled-input assertion may precede completion of the
online refresh, allowing a subsequent return to coalesce with it. That cause is
not yet confirmed.

**Goal and scope:** reproduce and identify the ordering behind this specific
failure. Inspect lifecycle events, in-flight reads and the test's freeze/return
barriers. If evidence proves a test synchronization issue, repair that barrier
without weakening the user-visible assertions. Record a separately bounded repair
if a production lifecycle defect is found.
**Invariants:** preserve pending verification, read-only behavior before fresh
success, offline draft durability, query deduplication and private-data clearing.
**Validation:** capture the failing ordering where reproducible; rerun the focused
case and affected return/lifecycle browser coverage, plus the frontend ladder for
any test changes. Record exact evidence and any remaining reproduction limit.
**Stop:** this one validation investigation and any demonstrated test-only repair;
no speculative production lifecycle changes or broader performance work.

## Later queue

1. **Performance work:** measure representative workloads before scoping changes,
   including the existing frontend bundle warning and bounded match-card reads.
2. **Security review:** perform the separately scoped wider application/operational
   review after the above correctness repairs.

No automatic opponents/byes/brackets, team match play, extra holes, new scoring
rules, public match sharing, cold offline launch or background sync is included.
