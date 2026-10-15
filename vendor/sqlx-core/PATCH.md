# SQLx core verify-ca compatibility patch

This directory contains the crates.io `sqlx-core` 0.8.6 package with its
normalized Cargo manifest and published licenses. Package SHA-256:
`ee6798b1838b6a0f69c007c133b8df5866302197e404e8b6ee8ed3e3a5e68dc6`.

The only source change is the error pattern in
`src/net/tls/tls_rustls.rs::NoHostnameTlsVerifier::verify_server_cert`: handle
Rustls `NotValidForNameContext { .. }` alongside `NotValidForName`. This restores
intentional chain-only `verify-ca` behavior for a trusted certificate through
another alias. All other results propagate unchanged. `verify-full` uses the
ordinary verifier; TLS 1.2/1.3 handshake signature delegation is unchanged.

This is a local compatibility patch, not a claim of an upstream backport. The
pinned Rustls 0.23.46 verifier validates the chain, usage, validity and signatures
before checking the server name. The narrow exception relies on that order;
revalidate it on TLS dependency changes. Rustls/WebPKI versions and features are
unchanged. The existing separate PostgreSQL cancellation override is untouched.

Provenance: [SQLx v0.8.6 source](https://github.com/transact-rs/sqlx/blob/v0.8.6/sqlx-core/src/net/tls/tls_rustls.rs).
Exact snapshot comparison and local handshake evidence are in
`docs/validation/sqlx-verify-ca-2026-10-15/`. No system roots, production TLS
settings, custom chain verifier or application transaction policy are added.

This preserved third-party snapshot is excluded from workspace membership and
the application-owned 400-line source limit, as is the existing PostgreSQL
snapshot. Keep upstream organization, formatting and warnings unchanged; do not
refactor unrelated source. The production Dockerfile must copy this directory.

Remove the override only after adopting a compatible upstream version with the
same behavior and passing the full certificate-mode matrix, cancellation tests,
backend/PostgreSQL ladders and locked production build. A passing general suite
alone does not establish certificate-verification compatibility.
