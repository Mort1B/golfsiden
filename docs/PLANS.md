# Plans

`PLANS.md` contains only the active implementation step and a short work queue.
Completed behavior belongs in `Documentation.md`; durable technical decisions
belong in `ARCHITECTURE.md`.

## Active step

None. Awaiting approval of the next bounded step.

## Next candidate

**Tournament completion UI:** reuse the approved backend action with explicit
confirmation, readiness/blockers, round links, exact-admin authority and live
private-query reconciliation. Bound separately after backend completion.

## Later

- **Archive:** separate exact-admin action for completed tournaments; retain
  private history and existing membership, no deletion or reversal. Implement
  database/backend guards before UI and history filtering.
- Configurable tie-breaks, public share links, offline scoring, and account
  recovery.
- Decide whether PostgreSQL should enforce at most one open round per tournament;
  reads currently select the highest-numbered open round deterministically.
- After roadmap completion, performance work, and security review, decide the
  contracts for four-ball, Stableford, and match play separately.
