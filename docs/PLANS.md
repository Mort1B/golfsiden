# Plans

`PLANS.md` contains only the active implementation step and a short work queue.
Completed behavior belongs in `Documentation.md`; durable technical decisions
belong in `ARCHITECTURE.md`.

## Active step

No active implementation step or queued implementation candidate.

## Shared completion requirements for future implementation

Each active step must satisfy `docs/AGENT_WORKFLOW.md`, including applicable
read-only specialist review for contract, handicap, scoring, or synchronization
changes. Browser checks cover relevant loading, error, empty, populated, and
long-content states. Record exact blockers for skipped checks. Update
`Documentation.md` and `LatestExplanation.md` when behavior changes, and
`ARCHITECTURE.md` when a durable boundary/contract changes. Validate any necessary
migration against PostgreSQL after reading `migrations/AGENTS.md`.

## Later

- Configurable tie-breaks, public share links, offline scoring, and account
  recovery.
- Decide whether PostgreSQL should enforce at most one open round per tournament;
  reads currently select the highest-numbered open round deterministically.
- After roadmap completion, performance work, and security review, decide the
  contracts for four-ball, Stableford, and match play separately.
