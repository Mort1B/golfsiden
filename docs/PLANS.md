# Plans

`PLANS.md` contains only the active implementation step and a short work queue.
Completed behavior belongs in `Documentation.md`; durable technical decisions
belong in `ARCHITECTURE.md`.

## Active step

None. Approval is required before starting the next candidate.

## Next candidate

**Archive administrator UI and history filtering:** reuse the guarded archive
action with exact-admin confirmation, live private-query reconciliation and
deliberate archived/current list selection. Retain member history access and
independent final visibility; no deletion or reversal.

## Later

- Configurable tie-breaks, public share links, offline scoring, and account
  recovery.
- Decide whether PostgreSQL should enforce at most one open round per tournament;
  reads currently select the highest-numbered open round deterministically.
- After roadmap completion, performance work, and security review, decide the
  contracts for four-ball, Stableford, and match play separately.
