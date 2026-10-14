# Remaining private-read authority inventory

A1f traced all remaining authenticated tournament-scoped JSON GET boundaries at
source revision `21917b6`. Of 46 explicit GET routes, 14 belong to previously
repaired families and five are excluded health/readiness, auth, self-profile or
SSE routes. The remaining 27 comprise 11 source-bound reads and 16 incomplete
boundaries, with no unresolved source paths. No application, schema, dependency
or test source changed, and no endpoint or database was called.

The findings distinguish two P2 authority gaps. Twelve reads retain session/user
and exact membership locks but omit the final database wall-clock expiry check:
three numeric access/cards, four match detail/completion/table reads, two
four-ball cards, two Stableford cards and invitation metadata. Later revocation
and credential changes serialize behind those locks, while natural expiry can
advance during loading. Final-visibility metadata is a separate P2 user-only
boundary: its membership lock cannot revalidate the extracted session.

Three course GETs have P3 session-binding gaps. Their payloads are globally
bundled/registered course facts; all eight current catalog entries are missing
or incomplete, so successful provider fetching and quota impact are conditional.
No current hidden-score disclosure or provider-quota abuse was established.
These findings come from source, not newly reproduced races.

Awards, player-account metadata, result-share status, all seven Fantasy GETs
and match listing already retain authority and check the session after loading.
Their transaction owners differ: some use default isolation and serialized
round locks or one payload statement, others use repeatable read. Cache and
retry policies also differ; `no-store` prohibits storage without literal
`private`. Error-header coverage gaps do not alone establish cached disclosure.
Fantasy selection/results GETs may finalize due windows and emit invalidations,
so the inventory inspected source without executing them.

For example, the numeric score-access read holds an active session while
resolving owners but also commits an empty singles result without a final expiry
check. The next candidate, A1g, is limited to that GET and the ordinary numeric
member/scoring card GETs. It must first reproduce late expiry, preserve owner,
handicap, redaction and retry contracts, and validate without changing mutations
or format-specific siblings. No repair has started.

Two independent read-only reviews covered every inventoried route and confirmed
46 unique route declarations and the manifest classifications. Review corrected
two P3 documentation details about match filtering and the exact unchecked
commit sites; no inventory findings remain. Local links/source-line targets,
manifest consistency, the current catalog and documentation-only diff checks pass.

The [inventory report](validation/private-read-inventory-2026-10-14/README.md)
contains every remaining route, source file/line evidence, severity, transaction,
cache and retry details, and the [46-route manifest](validation/private-read-inventory-2026-10-14/routes.json).
Backend, PostgreSQL, frontend and browser ladders are unaffected by this
source/documentation-only step and were not rerun. Previous A1e acceptance is
linked historical evidence, not new validation of the inventoried routes.

[PLANS.md](PLANS.md) selects one next candidate. Other read repairs, legacy-write
expiry inventory, dependency/recovery/capacity findings and existing deployment
acceptance limits remain queued.
