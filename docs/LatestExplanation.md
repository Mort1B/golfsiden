# Session-bound private leaderboard reads

The first planned repair binds all four private round/tournament gross/net
leaderboard endpoints to the active session inside their result transaction.
Handlers pass session IDs instead of trusting the user ID extracted earlier.
Session/user and exact membership locks remain held through assembly, with a
final database wall-clock expiry check before commit.

A revocation or credential change committed before transactional authorization
now denies results. An already-authorized reader finishes before a later
revocation can commit. For example, a reader blocked behind a transaction that
revokes its session previously returned 200 with private scores; the same retained
regression now returns 401. Expiry during later membership or score-loading waits
also returns 401, including for match-only not-applicable envelopes.

A changed authority row can invalidate a repeatable-read snapshot. The server
retries the whole read once with fresh session, membership, role, visibility and
facts. Repeated conflict returns a generic private 503 without standings. Every
reply from the four routes is `private, no-store`. Gross/net calculations,
handicap snapshots, public sharing and the other private read families retain
existing behavior. No frontend, schema or dependency changed.

Validation passed: 259 backend tests without the database feature, 748 tests in
the PostgreSQL-feature ladder, formatting, Clippy, API build and schema-39
migration/seed checks. The database total includes ordinary tests; four manual
performance probes were ignored. Ten new race/coherence tests cover revocation,
credentials, expiry, membership removal/demotion, cancellation and parallel reads;
additional tests cover bounded retry exhaustion and the private 503 contract.
Independent read-only review found no actionable issue.

Focused Chrome checks passed at 320, 390 and 1280px across loading, error/retry,
empty/populated results, long names and authentication denial. The broader retained
suite stopped on an existing obsolete score-status text assertion. That failure
is recorded and its test-only maintenance remains queued; no full-suite browser
pass is claimed. Physical-device, hosted and load acceptance remain separate.

The [validation report](validation/leaderboard-session-2026-10-14/README.md)
contains reproduction details and limits. [PLANS.md](PLANS.md) selects A2's round
lifecycle expiry repair next. Sibling private reads and all other assessment
findings remain queued; no adjacent implementation was started.
