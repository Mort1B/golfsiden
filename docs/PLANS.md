# Plans

`PLANS.md` contains only the active implementation step and a short work queue.
Completed behavior belongs in `Documentation.md`; durable technical decisions
belong in `ARCHITECTURE.md`.

## Active step

None. Awaiting approval for the next bounded implementation step.

## Next candidate

- Partner-repeat-aware team generation and handicap balancing. Define the exact
  balancing objective, repeat constraints, admin controls, persistence effects,
  and acceptance examples before implementation.

## Later

- Flight progress and missing-score alerts.
- Configurable tie-breaks, public share links, offline scoring, and account
  recovery.
- Rate limiting, deployment, backups, and production database roles.
- Decide whether PostgreSQL should enforce at most one open round per tournament;
  reads currently select the highest-numbered open round deterministically.
- After roadmap completion, performance work, and security review, decide the
  contracts for four-ball, Stableford, and match play separately.
