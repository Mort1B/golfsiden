# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. The next candidate requires the user's instruction to proceed.

## Next candidate — one live owner per match-results route

**Goal:** remove duplicated live invalidation on delegated match-only results,
using the [investigation evidence](performance/subscribers/README.md).

**Scope and exact behavior:** move `useTournamentLive(tournamentId)` from shared
`MatchResults` into the direct `MatchResultsPage` wrapper in the same file. Keep
`PlayerHistoryPage` and `LeaderboardPage` parent subscriptions unchanged. Direct
and delegated routes each retain one live owner while shared result queries
load, fail or remount. Add ownership/regression tests and update affected docs.
No global event deduplication or production changes outside this boundary.

**Invariants:** preserve every required authority refresh, score/match target,
synchronous disconnect/visibility projection erasure, account isolation,
transport cancellation, late-denial suppression, queued browser-return drain and
shared management-read ownership. Preserve writable intent and all sporting
rules; no API, schema, backend, query-key, retry or freshness change.

**Validation:** use the real hook and controlled EventSource in tests of direct,
filtered history, delegated history and global results. Cover early/delayed open,
match/visibility events, child loading/error/remount, tournament switching,
disconnect/reconnect and logout/account replacement. Assert fresh denial clears
related projections while cancelled success/denial cannot affect newer data.
Run the frontend ladder, existing queued-return/denial/cancellation checks and
production mobile/desktop browser suites. Replay the retained subscriber harness:
compare starts, aborts, completions and fresh final content. Initial effect timing
can change; demonstrate one owner and fewer settled-event starts without demanding
identical total navigation counts. Record remaining remount work and limitations.

**Stop:** publish the validated ownership repair and evidence. Do not redesign
parent loading, transport sharing or the return drain; do not start history-payload,
PostgreSQL authorization or security work.

## Later queue

1. **Remaining performance findings:** separately reassess transient list remount
   fetch starts and full-card history reads. Acquire disposable PostgreSQL
   measurements before any list-authorization repair. Preserve authority refresh
   and fail-closed behavior in every proposal.
2. **Security review:** perform the separately scoped wider application/operational
   review after agreed performance repairs.

No automatic opponents/byes/brackets, team match play, extra holes, new scoring
rules, public match sharing, cold offline launch or background sync is included.
