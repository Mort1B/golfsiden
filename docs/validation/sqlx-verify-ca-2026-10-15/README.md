# A1w — SQLx verify-ca contextual-hostname compatibility — 2026-10-15

This closes the existing P3 compatibility gap retained by the
[Rustls dependency step](../rustls-update-2026-10-15/README.md#existing-p3-verify-ca-contextual-hostname-compatibility).
Baseline: `437d6a4ad08fb5bf3a9fd7fa19430a451c86c7ba`. No hosted outage or new
security regression was observed. This step restores the explicitly selected
chain-only mode; it changes no default TLS policy or hosted configuration.

## Patch and verification boundary

The exact published `sqlx-core` 0.8.6 package is preserved in `vendor/sqlx-core`,
excluded from workspace membership. One source pattern adds
`NotValidForNameContext { .. }` beside the existing `NotValidForName` case in
`NoHostnameTlsVerifier`. Every other verifier result propagates unchanged.
Rustls 0.23.46 validates the chain, constraints, usage, validity and certificate
signatures before its name check. SQLx's handshake-signature delegation and
ordinary `verify-full` verifier are unchanged. This narrow exception depends
on that ordering and must be revalidated on future TLS updates.

The [published SQLx source](https://github.com/transact-rs/sqlx/blob/v0.8.6/sqlx-core/src/net/tls/tls_rustls.rs)
contains the original legacy-only match. The
[PostgreSQL mode definition](https://www.postgresql.org/docs/17/libpq-ssl.html#LIBPQ-SSL-PROTECTION)
distinguishes chain-only `verify-ca` from hostname-verifying `verify-full`.
This is a local compatibility patch, not an upstream-backport claim.

[Exact source delta](core-source.patch) and [scope proof](patch-scope.json)
record the verified crates.io archive hash, only changed core source file,
unchanged package versions/dependency lists/features, one resolved core and
every resolved SQLx core consumer. The lockfile only replaces core's registry
source/checksum with the local path. The complete existing PostgreSQL override
is byte-identical to its [baseline fingerprint](baseline-graph.json), retaining
the cancellation backport. Both dependencies have explicit production Dockerfile
COPY instructions. Licenses, upstream organization and unrelated warnings remain.

## Actual TLS before/after matrix

The [Rust probe](../../../backend/examples/sqlx_verify_ca.rs) uses the application's
exact SQLx dependency graph. Every successful connection checks `pg_stat_ssl`,
TLS protocol and `SELECT 42`. Every failed connection must match a whitelisted
certificate or negotiation category; arbitrary connection errors/timeouts cannot
count as a certificate rejection.

The [matrix runner](tls_matrix.py), [PKI generator](pki.py) and
[fixture owner](fixtures.py) create eight fresh PostgreSQL fixtures per phase:
valid direct leaf, expired leaf, future leaf, corrupted certificate signature,
client-only EKU, omitted intermediate, complete intermediate chain and plaintext.
All certificates use synthetic names/keys. Roots are never installed system-wide.

| Control | Baseline | Candidate |
| --- | --- | --- |
| Trusted matching host, both modes, direct/complete chain | Encrypted SQL succeeds | Same |
| Trusted alias, verify-ca, direct/complete chain | Contextual hostname rejection | Encrypted SQL succeeds |
| Trusted alias, verify-full, direct/complete chain | Contextual hostname rejection | Same |
| Alias + unknown issuer, both modes | UnknownIssuer | Same |
| Alias + expired/future leaf, both modes | Expired / NotValidYet | Same |
| Alias + corrupt certificate signature, both modes | BadSignature | Same |
| Alias + client-only EKU, both modes | InvalidPurpose | Same |
| Alias + missing intermediate, both modes | UnknownIssuer | Same |
| require/prefer with untrusted root, TLS available | Encrypted SQL succeeds | Same |
| Plaintext-only server, verify-ca/verify-full/require | TLS negotiation refused | Same |

All 25 exact cases pass in each phase. The [matrix comparison](matrix-comparison.json)
confirms that only the two trusted `verify-ca` aliases
change outcome. The baseline fails the desired alias-success contract with the
specific contextual error; the patched build satisfies it. Exact observations,
protocols, image IDs, PostgreSQL versions and confirmed cleanup are in
[baseline TLS](tls-baseline.json) and [candidate TLS](tls-candidate.json).
Successful controls negotiated TLS 1.3. TLS 1.2 signature-delegation source is
preserved; this run does not establish a separate TLS 1.2 handshake matrix.

The earlier `tls_compatibility` example now expects alias success too. The A4
report remains a historical snapshot of its original lock/source state.

## Validation and review

| Check | Result |
| --- | --- |
| Actual TLS matrix, baseline / candidate | 25 exact cases pass in each phase |
| Existing PostgreSQL cancellation regressions | Five passed |
| Formatting | Passed |
| Workspace/all-target Rust tests | 264 passed, one existing benchmark ignored |
| Strict all-target/all-feature Clippy | Passed |
| Workspace/all-target PostgreSQL-feature tests | 953 passed, five existing manual probes ignored |
| Forward migrations / seed | 39 migrations successful; seed passed |
| Locked release API binaries | Built on host Rust 1.99.0 |
| Fixture/temporary credential/PKI cleanup | Confirmed for both matrices, controlled startup failure and ladder |

[Ladder results](validation.json) retain exit codes, counts, durations and local
PostgreSQL version. The PostgreSQL total includes ordinary backend tests and is
not additive with 264. The five ignored cases are the existing projection
benchmark and four PostgreSQL profiling probes; exact names and final count,
scope and cleanup checks appear in [final postconditions](final-postconditions.json).
No new ignored case or advisory allowlist is added. Cargo compiles the examples
as zero-test targets; the matrix executes the real probe assertions separately.

Four core and three PostgreSQL unchanged upstream warnings are visible from
the preserved dependency snapshots. Application/workspace strict Clippy passes;
unrelated vendor source is not refactored to hide those warnings.
Independent [source/harness review](review.md) found no actionable findings.
Final evidence/documentation consistency review is recorded there separately.

## Reproduction and limits

```bash
cargo build --locked -p golf-api --example sqlx_verify_ca --example tls_compatibility
python3 docs/validation/sqlx-verify-ca-2026-10-15/tls_matrix.py candidate
python3 docs/validation/sqlx-verify-ca-2026-10-15/validate.py
```

For baseline reproduction, copy the new probe/harness into the pinned checkout
before adding the core override, build the probe and select matrix `baseline`.
The runner asserts exactly 25 cases, every expected reason, encrypted SQL
controls and removal of all eight exclusively owned CID-recorded containers.
Credentials/PKI live in a private temporary directory, random database credentials
are passed through a mode-0600 file/environment, ports bind only loopback and
PostgreSQL data uses tmpfs. No retained data or unrelated container is targeted.
Tracked output contains synthetic categories and sanitized validation summaries;
private mode-0600 raw ladder logs stay local in `/tmp`.
A [controlled post-create startup failure](cleanup-control.json) also confirms
that the helper removes only its recorded container and temporary credentials.

No application SQL/API/UI, migrations, provider HTTPS trust or operator TLS
configuration changes. Frontend/browser checks do not apply to this driver-only
contract. Local PKI acceptance does not establish public-host TLS, image security,
CRL/OCSP behavior, mutual TLS, every algorithm/provider/platform or production
capacity. Container image build/deployment and the pinned image toolchain remain
operator-owned; host release-build acceptance is recorded separately.
