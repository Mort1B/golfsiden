# Plans

`PLANS.md` contains only the active implementation step and a short work queue.
Completed behavior belongs in `Documentation.md`; durable technical decisions
belong in `ARCHITECTURE.md`.

## Active step

None. Approval is required before starting the next candidate.

## Next candidate

**Archive backend/database:** separate exact-admin action for completed tournaments;
retain private history and existing membership, no deletion or reversal. Define
authorization, concurrency, audit and database guards before introducing UI or
history filtering.

## Later

- Archive administrator UI and history filtering after the guarded backend action.
- Configurable tie-breaks, public share links, offline scoring, and account
  recovery.
- Decide whether PostgreSQL should enforce at most one open round per tournament;
  reads currently select the highest-numbered open round deterministically.
- After roadmap completion, performance work, and security review, decide the
  contracts for four-ball, Stableford, and match play separately.
