# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. The next candidate requires the user's instruction to proceed.

## Next candidate — retain gated match-list observers during table loading

**Goal:** avoid cancelling an already-started list refresh solely because its
table temporarily renders loading, with the conditional benefit documented in
[the investigation](performance/remounts/README.md).

**Scope and exact behavior:** adjust `MatchResults` composition and add an explicit,
default-preserving readiness option to its `MatchRound` children. Derive children
only from current rounds data and retain their observers during table-only pending
state. Readiness controls query enablement and every private presentation: start
no initial list read before table readiness, and render no round headings, names,
cards, links or private messages while gated. Existing in-flight reads may finish
while disabled. Parent errors, missing rounds and account/tournament/round removal
still remove old owners. Other callers retain existing behavior. Add tests/docs.

**Invariants:** preserve current authority checks, synchronous projection erasure,
fresh-denial scope and cancelled-response guards, account keys, abort forwarding,
one route-owned live subscriber, queued return drain and shared management reads.
Do not retain server data/descriptors or a prior-authority/admission-history flag.
Do not hide private markup with CSS. Keep the existing 20-second freshness,
retry policies, query keys, API/backend/schema, writable intent, handicap snapshots
and sporting rules unchanged.

**Validation:** test initial pending table, table-first/list-first completion within
and beyond freshness, old held responses, rounds/table/list denials, actual
restricted-final payload changes, disconnect/return, account/tournament/round
changes, other `MatchRound` callers and shared management ownership. Run the frontend
ladder, privacy/cancellation/return regressions and production mobile/desktop
comparison. Count starts, aborts and completions plus absence of private DOM while
gated. Include prolonged table holds: re-enabling a stale completed list may still
refetch, leaving extra completed hidden work. Do not claim a universal reduction.

**Stop:** publish the bounded repair only after conditional request savings and
unchanged privacy/freshness are demonstrated, or record insufficient benefit.
Do not change freshness, add lifecycle history state or redesign invalidation to
force a three-request result. Do not begin payload/database/security work.

## Later queue

1. **Remaining performance findings:** separately assess full-card history reads.
   Acquire disposable PostgreSQL measurements before any list-authorization
   repair. Preserve authority refresh and fail-closed behavior in every proposal.
2. **Security review:** perform the separately scoped wider application/operational
   review after agreed performance repairs.

No automatic opponents/byes/brackets, team match play, extra holes, new scoring
rules, public match sharing, cold offline launch or background sync is included.
