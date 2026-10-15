# Latest iteration — final-visibility metadata session authority

A1m repairs the final-round-visibility metadata GET. Real controlled PostgreSQL
waits reproduced the original endpoint returning 200 after natural session
expiry, revocation and credential replacement. The repaired GET returns private
401 when authority is lost before transactional authorization or while an
otherwise valid request waits for membership.

The API passes session identity to a dedicated repository entry. Each
repeatable-read attempt loads stored tournament metadata before authorization,
derives the user from shared active-session/user locks, retains exact-admin
membership and makes a final database wall-clock session check before commit.
The lookup/error order and three-field response, including the full timestamp
version, remain unchanged. The final check covers earlier waits; it does not
freeze time through commit, serialization or delivery.

One complete fresh attempt is allowed after a `40001` snapshot conflict. A
second conflict returns generic private 503; unrelated database failures stay
single-attempt generic 500. Real conflict tests change visibility while a read
waits: the retry returns the new flag and version, or denies a newly demoted or
removed admin. Discarded authority and metadata never survive the retry.

All GET statuses now use `private, no-store`, including extraction failures.
PATCH retains its independent versioned write owner, CSRF requirements,
database guards, stale errors and post-commit notifications. Its existing cache
contract is covered explicitly: private success, `no-store` on unauthenticated
rejection, and headerless stale conflict. Existing internal user-only reads
retain their helper contract.

Fourteen new PostgreSQL/API cases and four existing visibility cases pass.
They cover both natural-expiry waits, revocation/credential serialization, exact
roles and other-tournament admin isolation, errors/cache/lookup order, full JSON
parity, fresh bounded retries, concurrent reads, cancellation and read-only
business state/SSE behavior. Independent read-only review found no remaining
issues. Workspace tests passed with 261 tests and one existing ignored benchmark;
strict Clippy and API binaries passed. All 1,133 frontend tests, typecheck, lint
and build passed. The full PostgreSQL suite passed 928 tests with four existing
ignored diagnostic/benchmark cases; migrate/seed passed with all 39 migrations
current. Database-feature totals include ordinary tests and are not additive.

Chrome passed at 320, 390 and 1280 px with actual typed metadata and full version
strings, hidden/released controls, held loading, 500/503 read retry, denied/missing
states and logout. Selected screenshots were visually inspected. There were no
visibility writes during observations, page errors or unexpected network failures.
Error/denial UI states use explicit injections; real PostgreSQL races prove the
authority behavior separately. A singleton metadata response has no empty-list
state. Physical-device, native-zoom, load and hosted acceptance remain separate.

This GET adds two session queries per ordinary success; no speedup is claimed.
The historical A1f private-read inventory now has zero remaining P2 routes and
three P3 course GETs. Other review findings remain open. The next candidate is
a compatible Rustls update in [the plan](PLANS.md).

Detailed evidence, exact validation and limits are in the
[final-visibility report](validation/final-visibility-session-2026-10-15/README.md).
