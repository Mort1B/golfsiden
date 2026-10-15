# Independent read-only A1w review

The security reviewer inspected the driver path, pinned Rustls/WebPKI ordering,
published-core snapshot comparison, Cargo/Docker wiring, PostgreSQL fingerprint,
Rust probe, fixture/PKI/matrix/ladder scripts and both retained TLS results.
Only source/file/JSON reads were performed; no builds, fixtures or network
commands and no edits or Git writes. No actionable findings were reported.

- SQLx selects the hostname-exception wrapper only while certificate checking
  remains enabled and hostname verification is explicitly disabled. The ordinary
  `verify-full` path is unchanged.
- Pinned Rustls checks chain/constraints/usage/time/signatures before hostname.
  The added contextual error pattern preserves that boundary; all other errors
  propagate unchanged. TLS 1.2/1.3 signature delegates remain byte-identical.
- The published core snapshot differs only in the intended OR-pattern and
  patch documentation. All 115 PostgreSQL vendor files match baseline evidence;
  the cancellation backport is preserved. Docker copies the additional core and
  `.dockerignore` does not exclude it.
- Both 25-case handshake matrices pass. Only two trusted `verify-ca` aliases
  change from contextual hostname rejection to encrypted-SQL success. Strict,
  invalid-certificate, plaintext and chain-completion controls retain outcomes.
- Fixture/PKI cleanup is confirmed in both matrix receipts. Failures require
  specific certificate categories rather than any unsuccessful connection.

The review explicitly limits runtime protocol coverage to negotiated TLS 1.3.
Unchanged TLS 1.2 delegation is source evidence, not a forced-protocol test.
The primary's [final postconditions](final-postconditions.json) now verify the
completed eight-stage ladder, exact test totals, 39 migrations, unchanged 115-file
PostgreSQL snapshot, both 25-case matrices and all associated cleanup. Final
file-only evidence/documentation consistency review follows separately.

Final file-only review found no findings. Reports match all retained test,
migration/seed/release counts and exact two-case TLS outcome changes. Published
JSON has no detected credential/private-key markers. Protocol, host-build and
hosted-environment limits remain explicit; A1w is closed and A1x is unstarted.
