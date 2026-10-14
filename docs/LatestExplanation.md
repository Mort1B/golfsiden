# Session-bound round readiness reads

The pairing and completion readiness GETs now carry the actual session ID through
complete readiness assembly. A controlled PostgreSQL regression showed both
previously returning HTTP 200 after revocation committed while they waited on
membership. The repaired routes return the existing private 401 response.

Both reuse the unchanged shared session-read boundary: one repeatable-read
attempt, active session/user share locks before exact tournament membership,
a final database wall-clock expiry check and one complete retry on snapshot
invalidation. Pairing's final check follows its additional singles-match
assignment validation. Completion's retry reloads stored round/tournament
context, exact role, owner facts, visible holes and the redacted projection.
Nothing from a discarded attempt can supply the next response.

For example, an admin-to-viewer demotion while a completion read waits also
changes the final from released to hidden. The fresh attempt must return only
front-nine progress, with null completeness/confirmation/readiness and no
hidden-state issue codes. Expiry during later fact loading similarly denies
success. Later revocation or credential writers wait behind an already
authorized reader; wall-clock time is not frozen during COMMIT.

Both selected GETs now apply `private, no-store` to every response. A second
snapshot conflict returns generic 503. Blocked readiness remains typed 200, and
singles completion-validation retains its existing 404 with a separate dedicated
completion endpoint. Fact SQL, blocker ordering, formats, frozen handicap/owner
rules and hidden-final redaction are unchanged. Internal unrestricted validation
helpers and open/complete/lock mutations retain their existing owners. No
mutation gains retries or events. There are two additional session queries per
ordinary successful read, with no measured speedup claim.

Sixteen new PostgreSQL/API regressions cover revocation, credential changes,
later-write serialization, membership removal, changed role/context, expiry at
membership and late-load gates, singles assignments, empty results, all roles,
resource errors, concurrency, cancellation, coherent snapshots and ready,
completed and locked states. Existing four-ball and Stableford privacy tests
supply their existing real session to the renamed completion entry.

Independent review found a test gap: the original context-change fixture repeated
the default visibility value. It now starts explicitly released and changes both
context and role, so either stale value fails. The corrected source/test review has no remaining findings.
An unchanged A1d name-order test also exposed a P3 collation assumption on Debian
PostgreSQL. Native uppercase/lowercase order differs from the repository's Alpine
image. Its test-only portability follow-up is queued; runtime sorting is unchanged.

The focused readiness/lifecycle/completion/four-ball/Stableford suites pass 73
checks before the review fixture tightening. The corrected readiness suite and
unchanged pairing compatibility suite pass 30. The full backend suite passes
261 tests; the final PostgreSQL-feature ladder passes 813 with four existing
manual performance probes ignored on the repository's PostgreSQL 17 Alpine
image. Formatting, strict all-target/feature Clippy, API builds, migration and
seed pass. The final schema is 39. The first Debian full run's existing collation
test failure remains recorded separately.

Chrome passes both readiness flows at 320, 390 and 1280px: delayed loading,
real populated responses, long content, injected blocked/empty/denied states,
error with read-only refresh, private 401 after logout and login redirection.
Lifecycle actions remain disabled while pending or failed; no lifecycle POSTs
occur during UI checks. There are no page exceptions or unexpected network
failures. Representative screenshots were visually inspected. The first probe's
synthetic empty completion response omitted an existing blocker code; runtime
decoding rejected it correctly, and the corrected probe passes all six flows.
[The report](validation/readiness-session-2026-10-14/README.md) retains the original
failure, exact wait gates, review correction, browser evidence and validation
limits. Schema, frontend source, dependencies and hosted configuration are
unchanged; physical-device, public-host and production-load acceptance remain
separate.

[PLANS.md](PLANS.md) queues the remaining private-read inventory and other
severity-rated review findings. No queued repair is started by this step.
