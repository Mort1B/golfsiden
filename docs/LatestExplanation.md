# Latest iteration — Rustls client dependency repair

A4 updates the two necessary lockfile entries: Rustls 0.23.43 to 0.23.46 and
WebPKI 0.103.13 to 0.103.14, the minimum compatible verifier required by that
Rustls release. Cargo manifests, features, application source and every vendored
SQLx file remain unchanged. The transaction-cancellation override is retained.
The production locked release build passed.

Fresh RustSec evidence found three additional Rustls entries beyond the
originally reviewed handshake defect, so 0.23.45 alone was insufficient for the
current audit. The same freshly fetched database reports four Rustls entries
before the update and zero after. This is dependency/advisory evidence, not four
reproduced application exploits: Brotli and hybrid/PQ support are disabled, and
no custom session-store debug logging was found. No hostile-record exploit was
replayed. The full audit still exits1 for the same three Quinn and one RSA
lockfile entries, plus a yanked ChaCha20 warning. None has a normal dependency
path under native defaults or all application features/all target platforms.

The actual provider constructor reached its official HTTPS endpoint's expected
401/403 rejection using an invalid synthetic key. Local transport checks proved
trusted encrypted SQL, strict `verify-full` hostname/issuer rejection, required
TLS refusal against a plaintext-only server and unchanged `require`/`prefer`
semantics. Those two modes do not verify identity in this driver. The actual
provider rejects a private untrusted certificate before sending an HTTP request
or bearer header. Separate explicit-test-root Reqwest checks prove local HTTPS
JSON acceptance and hostname rejection without changing production trust.

Testing also discovered an existing P3 SQLx compatibility issue: `verify-ca`
rejects a contextual hostname mismatch even with a trusted certificate. An
isolated original-lockfile probe fails the same way; SQLx core handles only the
legacy hostname-error variant. The final probe preserves that rejection and
checks matching-name success. Repair belongs to a separately approved driver
step; no new verification bypass or SQLx source change was introduced.

Workspace tests passed with 261 tests and one existing ignored benchmark, and
final all-target/all-feature strict Clippy passed. The complete PostgreSQL suite
passed 928 tests with four existing ignored diagnostic/benchmark cases. All five
SQLx cancellation regressions passed; migrate/seed left all 39 migrations current.
Database-feature counts include ordinary tests and are not additive. The probe is opt-in rather than an ignored test. Its
plaintext prerequisite is preflighted, temporary PKI is deleted, and only its
recorded container ID is removed. A forced port conflict verified failed-startup
cleanup while leaving the unrelated database running. Independent read-only
lock/source/probe/documentation review has no remaining actionable issues.

Frontend/browser checks were not rerun: frontend source, API contracts and UI
state are unchanged. TLS behavior is exercised directly. No public-host, image,
paid-provider data, capacity or deployment acceptance is claimed. The API must be
rebuilt from the committed lockfile; no new migration or environment setting is
needed. The next candidate is the bounded restore-checksum repair in
[the plan](PLANS.md).

Exact versions, advisory links, verification limits and results are in the
[Rustls report](validation/rustls-update-2026-10-15/README.md).
