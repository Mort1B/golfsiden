# Session-bound private tournament reads

The private tournament list, own-membership list and tournament detail now carry
the session ID through result assembly. All three previously returned HTTP 200
when session revocation committed while the request waited for tournament data.
The retained regression reproduces that original behavior and now returns the
existing private 401 envelope on every route.

`repositories/tournaments/reads.rs` owns one repeatable-read transaction per
attempt. It derives the user from a locked active session, holds session/user
and exact membership share locks, loads the existing payload in the same snapshot
and rechecks database wall-clock expiry before commit, including for empty lists.
For example, a waiting list cannot return private tournament names after expiry.
A revocation or credential change that reaches authority first denies the read;
a later writer waits behind a read already holding its authority locks.

Collections acquire all existing membership locks in tournament UUID order in
one query. A newly inserted membership can coherently appear on the next read.
Removal or demotion that invalidates the current snapshot discards the complete
attempt and permits one fresh read. A second 40001 returns generic private
`503 service_unavailable`. No partial data or old role survives a retry, and no
mutation retry is added. All statuses on these three GET routes are
`private, no-store`.

Existing list orders, complete/archived membership visibility, roles, conditional
entrant links and missing/cross-tournament errors are preserved. The repair adds
session checks and a constant-count collection lock query; it makes no latency or
production capacity improvement claim. Other private read families remain
separate work. There is no schema, frontend source, dependency, scoring-rule or
hosted configuration change.

Thirteen new PostgreSQL/API regressions cover session revocation, credential
replacement, natural expiry at controlled waits, empty lists, membership changes,
snapshot coherence, twelve concurrent valid reads, cancellation and payload/cache
contracts. Focused suites pass 48 tests and retry/error helpers pass three.
The full backend suite passes 261 tests; the full PostgreSQL-feature suite passes
770 tests with four existing manual performance probes ignored. Formatting,
strict Clippy, all API binary builds, migration and development seed pass.
Independent read-only review found no remaining findings after test fixtures were
corrected to use a typed membership role and a separate unclaimed player.

All nine Chrome flows pass across 320, 390 and 1280px, covering delayed loading,
real database-backed payloads, long text, error/read-only retry, empty or denied
states and private 401 responses after logout. No page exceptions or unexpected
network failures occurred; representative mobile/desktop screenshots were
inspected. [The report](validation/tournament-read-session-2026-10-14/README.md)
contains the original failure, test details, captured browser results and limits.
Physical-device, public-host and production-load acceptance remain unchanged.

[PLANS.md](PLANS.md) selects the roster and round list/detail family as A1c next;
no adjacent repair has started.
