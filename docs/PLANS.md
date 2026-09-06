# Plans

`PLANS.md` contains only the active implementation step and a short work queue.
Completed behavior belongs in `Documentation.md`; durable technical decisions
belong in `ARCHITECTURE.md`.

## Active step

None. Awaiting approval of the next bounded step.

## Next candidate

**Flight progress:** Build a member-visible progress overview using stored
flight membership and visibility-projected owner progress. Count shared team
cards once; do not infer hidden final completeness from flight totals. Reuse
the existing admin readiness where appropriate. Bound any missing read
contract before implementation and validate redaction plus live updates.

## Later

- **Tournament closure contract decision:** Tournament start is implemented,
  but `backend/src/api/tournaments.rs` has no complete/archive action. Define
  whether and when a tournament closes, required round states, invitation and
  correction behavior, archive visibility, and any reversal policy before
  planning backend/database enforcement and then UI. This is not a missing
  button over an existing endpoint.
- Configurable tie-breaks, public share links, offline scoring, and account
  recovery.
- Decide whether PostgreSQL should enforce at most one open round per tournament;
  reads currently select the highest-numbered open round deterministically.
- After roadmap completion, performance work, and security review, decide the
  contracts for four-ball, Stableford, and match play separately.
