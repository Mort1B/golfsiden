# Latest iteration — SQLx verify-ca compatibility

A1w closes the retained P3 SQLx compatibility gap. A trusted certificate used
through a different hostname previously failed in intentionally chain-only
`verify-ca` because SQLx 0.8.6 ignored only Rustls's older hostname-error variant.
The local core patch also handles `NotValidForNameContext`. All other results,
TLS signature delegation and ordinary `verify-full` verification are unchanged.

The exact published core release is preserved under `vendor/sqlx-core`; one
source pattern differs. Cargo/Docker wiring includes it, with package versions,
dependency lists and active features unchanged. All 115 existing PostgreSQL
vendor files remain byte-identical, retaining the cancellation backport. Future
TLS updates must revalidate the pinned chain-before-hostname ordering and retained
certificate matrix. No default TLS mode, system roots, application SQL/API/UI,
schema or provider HTTPS trust changes.

Twenty-five actual TLS cases run before and after. Only two outcomes change:
trusted direct/intermediate-chain aliases now execute encrypted SQL in `verify-ca`.
`verify-full` still rejects those aliases. Both modes still reject an alias with
unknown issuer, expired/future leaf, bad certificate signature, invalid EKU or
missing intermediate; complete chains succeed. Plaintext-only servers remain
rejected by verification modes/require. Successful controls verify `pg_stat_ssl`
and `SELECT 42`, negotiating TLS 1.3. TLS 1.2 delegation is unchanged source;
there is no separate forced TLS 1.2 matrix.

Validation passes: five cancellation regressions, 264 Rust tests, 953
PostgreSQL-feature tests, formatting, strict Clippy, 39 migrations, seed and the
locked release build on host Rust 1.99.0. Five existing manual probes remain
ignored in the PostgreSQL ladder. Independent source/harness review found no
actionable issues. All owned fixtures/temporary credentials/PKI are removed,
including a controlled startup-failure cleanup check. Exact source delta,
receipts, review and limits are in the
[acceptance report](validation/sqlx-verify-ca-2026-10-15/README.md).

Container image/pinned-toolchain builds, deployment and hosted TLS remain
operator-owned. Frontend/browser checks do not apply to this driver-only change.
The next candidate is the retained leaderboard browser-test maintenance; it has
not started. No adjacent implementation or broader optimization begins here.
