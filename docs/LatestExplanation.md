# Fantasy rules and deployment guidance

The README now explains the implemented Fantasy game: four free picks and one
double-points captain per round, eligible carry-forward, the full net hole and
placement scales, shared team points, outcome-only match play, non-finish handling,
and independent manager/golfer round and overall leaderboards. It also describes
joining, selection deadlines, privacy and online-only writes.

The production section summarizes the existing backup, matching-build, migration,
runtime-grant and health/readiness sequence and links to the exact commands.
The deployment guide now includes a Fantasy setup and release checklist covering
enablement, two-account draft privacy, locking/carry-forward, both boards,
settlement, concealed results and recovery after an uncertain save. It distinguishes
installing the feature from enabling it in a tournament and records that schema
37 needs no additional Fantasy service, scheduler, secret or environment variable.

This is a documentation-only update. Current source rules and production Compose
services were checked against the text; Markdown references and whitespace were
validated, and the diff was reviewed. Runtime tests, browser checks and deployment
commands were not rerun because no executable files or configuration changed.
The earlier [Fantasy acceptance report](validation/fantasy-release-2026-10-08/README.md)
retains the actual local execution evidence and its limits. No hosted deployment
or server administration was performed.
