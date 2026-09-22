# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. The next candidate requires the user's instruction to proceed.

## Next candidate — investigate transient match-list remounts

**Goal:** establish whether the remaining list fetches cancelled by table-loading
remounts can be reduced safely after the route-ownership repair.

**Scope and behavior:** trace the match-results table/list observer lifecycle on
direct and delegated routes during initial open, visibility clearing and
reconnect. Compare populated and pending reads with the retained ownership
baseline; separate required authority work, cancelled refreshes and remount
fetches. Retain production-build mobile/desktop request/content evidence and
propose one bounded repair only if safe. Investigation and documentation only:
do not change production rendering, query subscriptions, fetching or freshness.

**Invariants:** private projections must disappear synchronously when required.
An observer kept alive is not permission to display stale data. Preserve all
required authority refreshes, account isolation, cancellation/late-denial guards,
queued returns, one route-owned live subscriber, shared management-read ownership,
writable intent, historical handicap snapshots and administrator-managed teams.

**Validation:** measure starts, aborts and completions alongside transient content
and observer ownership. Cover held old responses, fresh denials, disconnect,
account change and return ordering. Review any proposal against existing
privacy/cancellation/return tests; record synthetic, finite-window and unavailable
PostgreSQL timing limits. No latency or SQL-saving claim from request counts alone.

**Stop:** publish evidence plus one bounded implementation proposal, or explain
why no safe reduction is established. Do not implement the proposal or broaden
into full-card payload, database authorization or security work.

## Later queue

1. **Remaining performance findings:** separately assess full-card history reads.
   Acquire disposable PostgreSQL measurements before any list-authorization
   repair. Preserve authority refresh and fail-closed behavior in every proposal.
2. **Security review:** perform the separately scoped wider application/operational
   review after agreed performance repairs.

No automatic opponents/byes/brackets, team match play, extra holes, new scoring
rules, public match sharing, cold offline launch or background sync is included.
