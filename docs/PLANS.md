# Plans

`PLANS.md` contains only current and queued work. Completed behavior belongs in
`Documentation.md`; durable technical decisions belong in `ARCHITECTURE.md`.

## Active step

None. The next implementation step requires user approval.

## Next candidate

**One open round per tournament:** Decide the product rule before adding a
PostgreSQL constraint. Current reads deterministically select the highest-
numbered open round. Any enforcement needs an existing-data preflight and
concurrency validation without silently closing historical rounds.

## Later, as separate bounded steps

**Additional formats:** After roadmap completion, performance work and security
review, define four-ball, Stableford and match play as separate contracts.
