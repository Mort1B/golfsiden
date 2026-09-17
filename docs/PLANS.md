# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. The next candidate requires the user's instruction to proceed.

## Next candidate — route-level JavaScript splitting

**Goal:** reduce cold-entry JavaScript transfer and startup cost using the
[measured baseline](performance/README.md). Login currently loads the same
222,606-byte gzip JavaScript body as the complete workspace.

**Scope and behavior:** defer route-only page modules through the existing router.
Keep shared authentication, scoring guards, account isolation, offline queues and
provider lifetime intact. Provide accessible route loading and recoverable chunk
load failures. Preserve direct links, history navigation and pending-edit guards.
Do not change API/schema contracts, scoring, authorization, query staleness,
SSE refresh policy, dependency versions or global styling.

**Invariants:** preserve private-read authorization, account isolation, historical
handicap snapshots, offline drafts, scoring correctness and administrator-managed
teams. Code splitting must not restart providers or bypass existing route gates.

**Validation:** run the complete frontend ladder. Repeat the baseline production
build/browser workloads and record total initial compressed JavaScript across all
loaded chunks, request counts and cold/warm ready-time ranges. Validate direct
links and in-app navigation across account, management, scoring, match and shared
result pages at mobile/desktop widths; exercise chunk-loading failure/recovery,
account changes and pending edits. Do not count merely moving bytes between
eagerly downloaded chunks as an improvement.

**Stop:** publish a measured reduction in startup bytes with no material ready-time
regression and resolved behavior checks, or explicitly report that the candidate
did not improve the baseline and re-bound the plan. Do not begin backend read
repairs or the wider security review.

## Later queue

1. **Remaining performance findings:** acquire disposable PostgreSQL measurements
   before choosing any list-authorization repair; separately reassess full-card
   history reads and live-startup request amplification from the baseline.
2. **Security review:** perform the separately scoped wider application/operational
   review after agreed performance repairs.

No automatic opponents/byes/brackets, team match play, extra holes, new scoring
rules, public match sharing, cold offline launch or background sync is included.
