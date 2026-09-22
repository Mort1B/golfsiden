# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. The next candidate requires the user's instruction to proceed.

## Next candidate — investigate duplicate live invalidation subscribers

**Goal:** establish whether delegated match-result pages repeat authority
invalidation for a single shared-stream event, and whether one narrowly scoped
repair can remove duplicate work while preserving privacy and freshness.

**Scope and behavior:** investigate match-only player history and global results,
where both parent and `MatchResults` child subscribe to the same tournament stream.
Compare them with the direct match-results route using initial open, match events,
disconnect/reconnect and account transitions. Trace subscriber/event/request
ownership and retain browser evidence. This step is investigation and
documentation only; do not change production subscriptions, fetching, query
freshness, projection erasure, scoring, API contracts or backend code.

**Invariants:** each relevant event must still perform required authority refresh.
Disconnect/visibility transitions continue to erase private projections; stale
success or denial cannot restore or erase newer authorized data. Preserve queued
browser-return revalidation, account isolation, writable score intent, historical
handicap snapshots and administrator-managed teams.

**Validation:** use production-build mobile/desktop browser cases with request
starts, aborts, completions and fresh final content. Distinguish multiple
subscribers from transient parent remounts and native reconnects. Review any
proposed deduplication boundary against existing live/denial/return-order tests;
record synthetic/finite-window and unavailable database timing limits.

**Stop:** publish evidence and one bounded implementation proposal, or record
that no safe reduction is established. Do not implement a proposal, redesign
parent loading or begin the wider security review in this step.

## Later queue

1. **Remaining performance findings:** separately reassess transient list remount
   fetch starts and full-card history reads. Acquire disposable PostgreSQL
   measurements before any list-authorization repair. Preserve authority refresh
   and fail-closed behavior in every proposal.
2. **Security review:** perform the separately scoped wider application/operational
   review after agreed performance repairs.

No automatic opponents/byes/brackets, team match play, extra holes, new scoring
rules, public match sharing, cold offline launch or background sync is included.
