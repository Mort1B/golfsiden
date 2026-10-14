# Ordered browser authentication

All six cookie-changing frontend requests now share one in-memory ordering
boundary across route lifetimes: login, logout, anonymous onboarding, invitation
registration, player-claim registration and own profile password changes.
For example, after leaving a held creation response and submitting a newer
login, the login waits until the earlier response can no longer overwrite its
cookie. The departed form cannot publish old private caches or navigation.

Intent generations fence session reads and callbacks, while ordinary credentialed
HTTP cannot cross an unsettled identity boundary. A read begun under an earlier
settled identity is discarded rather than sent with a newer cookie. Idle requests
retain immediate dispatch and cancellation, and cookie-free capability/token
reads remain independent. Failed/ambiguous cookie writers reconcile read-only;
a failed reconciliation clears private data and exposes authentication retry.
No write is automatically replayed. Queued logout checks the actual cookie,
and queued password changes retain their original CSRF authority.

Submitting forms remain mounted long enough to retain accepted receipts.
Onboarding/invitation continuations require their lifetime, intent and canonical
origin. Claim lost-response recovery still checks the prepared player and chosen
username. Accepted invitation success wins over preview errors. A failed newer
login remains visible even when reconciliation finds an earlier creator cookie;
the user can retry deliberately. Coordination covers the current document/runtime,
with no persistent queue or cross-tab/reload mechanism.

Three new regressions failed against the original source. The full frontend suite
now passes 1,133 tests in 147 files; TypeScript, ESLint and production build pass.
All 18 real Chrome scenarios pass, covering actual cookie and displayed identity,
request order, departure, logout, renewal with a real CSRF write, late session
reads, ordinary receipts, claim recovery, profile changes and password recovery.
Mobile/desktop loading, error/retry, empty and long-content states were exercised;
representative masked screenshots were inspected. Final read-only source review
found no remaining actionable issue.

Validation caught and repaired an idle-path cancellation regression, a slow
reconciliation settlement race and invitation preview/receipt precedence.
Browser harness corrections and the disposable API's initially missing recovery
origin are recorded in the [validation report](validation/identity-ordering-2026-10-14/README.md).
Backend code, API JSON, schema, dependencies, sporting rules and deployment
configuration are unchanged. Backend ladders were not repeated; the unchanged
schema-39 API was migrated/seeded locally for browser fixtures. No latency,
physical-device, hosted or production-load acceptance is claimed.

[PLANS.md](PLANS.md) selects A1b's three private tournament reads next. Remaining
private families and assessment findings stay queued; no adjacent repair started.
