# Rustls client dependency repair — 2026-10-15

A4 addresses the P2 reachable Rustls finding in the
[original review](../application-review-2026-10-14/README.md#a4--p2-reachable-rust-tls-advisory).
Baseline: `ac987dedae3f5fb1ab6973775d2ecda2572f68b1`.

## Dependency and advisory evidence

The [runtime dependency diff](lock-diff.json) is exactly two packages and their checksums:

| Package | Before | After | Reason |
| --- | --- | --- | --- |
| rustls | 0.23.43 | 0.23.46 | Smallest release covering all four current Rustls audit entries |
| rustls-webpki | 0.103.13 | 0.103.14 | Minimum version required by Rustls 0.23.46's manifest |

The initially considered 0.23.45 fixes the originally reviewed
[RUSTSEC-2026-0285](https://rustsec.org/advisories/RUSTSEC-2026-0285.html), but the
fresh database also reports three October 14 entries requiring 0.23.46. The
[upstream release](https://github.com/rustls/rustls/releases/tag/v%2F0.23.46)
confirms those fixes. The intermediate resolver choice of WebPKI 0.103.15 was
reduced to the required 0.103.14; no intermediate lockfile was published.

| Advisory | Application evidence and limit |
| --- | --- |
| [0285](https://rustsec.org/advisories/RUSTSEC-2026-0285.html) | TLS 1.3 is reachable through Reqwest/SQLx. This protocol-boundary defect does not bypass transcript authentication; no hostile-record exploit replay was attempted. |
| [0346](https://rustsec.org/advisories/RUSTSEC-2026-0346.html) | Secret exposure requires custom session-store debug logging. No such application store/logging was found; ordinary built-in cache output is outside that condition. |
| [0348](https://rustsec.org/advisories/RUSTSEC-2026-0348.html) | The affected Brotli certificate decompression feature is disabled. |
| [0349](https://rustsec.org/advisories/RUSTSEC-2026-0349.html) | The described confidentiality loss concerns hybrid key-exchange prediction. This build uses the ring provider's classical groups; hybrid/PQ support is not enabled. |

These are four advisory matches, not four demonstrated application exploits.
The [before](audit-before.json) and [after](audit-after.json) summaries use the
same freshly fetched RustSec database: 1,329 advisories, commit
`edb58f5974b83eb8302332821cb6d389777aa351`, updated October 14 at 22:18:32 +02:00.
Rustls matches fall from four to zero. Both audit commands exit 1 because the
same three Quinn and one RSA entries remain in the lockfile; the yanked
ChaCha20 warning remains too. Nothing was ignored or allowlisted.

[Normal before](tree-before.txt), [normal after](tree-after.txt),
[all-feature before](tree-all-features-before.txt) and
[all-feature after](tree-all-features-after.txt) preserve both Reqwest and SQLx
paths. [Reachability checks](remaining-reachability.json) find no normal path to
Quinn, RSA or ChaCha20 with native defaults or all application features and all
target platforms. This does not establish future-feature or image security.

Active Rustls features remain exactly `ring,std,tls12`. Cargo manifests and
feature selections are unchanged. The SQLx workspace override and every vendored
file are unchanged; [the patch fingerprint](sqlx-preservation.txt) records a
whole-directory SHA-256 comparison. SQLx/WebPKI/Rustls remain on the same major
API lines, with patched crates retaining their 1.71 MSRV. No application handler,
repository, scoring rule, schema, frontend contract or TLS setting changed.

## TLS compatibility acceptance

The opt-in [Rust example](../../../backend/examples/tls_compatibility.rs) compiles
against the application's exact dependency graph and production provider client.
The [Python harness](tls_harness.py) uses temporary private PKI, an owned
PostgreSQL 17 TLS container and a loopback HTTPS server. It never installs a
system root or changes production trust settings. The main non-TLS disposable
PostgreSQL instance must already be available at loopback port 55449.

```bash
cargo build --locked -p golf-api --example tls_compatibility
python3 docs/validation/rustls-update-2026-10-15/tls_harness.py
target/debug/examples/tls_compatibility public-provider
```

The successful [local TLS result](tls-results.txt) establishes:

- `verify-full` with the trusted matching hostname executes encrypted SQL;
  mismatched hostname and unknown issuer are rejected for the specific certificate
  reason. SQLx exposes Rustls certificate failures as `Io(InvalidData)`; the probe
  accepts only the expected certificate reason, not arbitrary connection errors.
- `verify-ca` succeeds with the trusted matching name and rejects an unknown
  issuer. Its existing contextual-hostname rejection is preserved, as detailed
  below. `require` and `prefer` still negotiate encryption without verifying this
  private issuer. They are not identity-verification modes in this driver.
- `require` and `verify-full` reject the plaintext-only server rather than
  silently downgrading. Successful SQL checks inspect `pg_stat_ssl` and execute
  a query; they are not connection-construction-only assertions.
- The actual unchanged provider constructor rejects the local untrusted
  certificate with its safe `Upstream` error. The HTTPS server receives no HTTP
  request or synthetic bearer from that client. A separate Reqwest transport
  using an explicit test root accepts the local JSON and rejects a mismatched
  hostname. This positive local transport has extra test trust; it is not a
  claim that the production provider accepts private roots.
- The actual provider's fixed official HTTPS endpoint returns the expected
  HTTP 401/403 mapping with an invalid synthetic key. This proves normal trusted
  HTTPS through that constructor, not paid course access or production quota.

The complete run removes its TLS container and temporary keys. The plaintext
PostgreSQL prerequisite is preflighted with an SSLRequest that must return `N`.
Container creation records a private ID before startup, and cleanup removes only
that owned ID. A [controlled occupied-port failure](cleanup-failure-results.json)
also proves cleanup without stopping the unrelated full-suite database.
Initial harness
iterations needed the SELinux volume label, SQLx's actual error mapping and the
baseline `verify-ca` expectation below. The example's initial compile/lint
issues were confined to probe assertions. Corrected focused checks and final
Clippy passed; these setup failures were not application regressions.

### Existing P3: verify-ca contextual hostname compatibility

The initial probe expected `verify-ca` to accept a trusted certificate whose
DNS name differed from the connection IP. Both the original lockfile in an
isolated temporary copy and the new lockfile instead reject it with
`InvalidCertificate(NotValidForNameContext)`. The
[baseline comparison](verify-ca-baseline.txt) retains that evidence.

SQLx core 0.8.6's `NoHostnameTlsVerifier` suppresses only the older
`CertificateError::NotValidForName`, not its contextual variant. The result is
stricter than the intended chain-only mode, and could reject a legitimate
trusted-server connection through another alias. No actual hosted outage was
observed. This is an existing P3 compatibility finding, not weakened security
or a regression from this update. A driver fix needs a separate approved step;
this update does not alter SQLx source or verification policy. `verify-full`
identity checks remain the accepted strict case.

## Validation

| Check | Result |
| --- | --- |
| Pre/post advisory comparison | Four Rustls matches become zero; overall audit still exits 1 for four unchanged optional-package entries |
| Exact lockfile diff | Only two package versions/checksums change; features/manifests/vendor retained |
| Formatting | Passed |
| Workspace tests | 261 passed, one existing ignored benchmark |
| Final strict all-target/all-feature Clippy | Passed; three unchanged vendored SQLx warnings |
| API binaries | Built |
| Production command: locked release binaries | Passed on host toolchain |
| Final local TLS harness | 14 assertions passed; container/temporary-key cleanup passed |
| Actual official provider HTTPS | Expected 401/403 mapping with synthetic invalid key |
| Controlled fixture startup-failure cleanup | Passed; unrelated database untouched |
| Full PostgreSQL suite | 928 passed, four existing ignored; exit 0, 840 seconds |
| SQLx cancellation regressions | All five passed within full suite |
| Migrate/seed | Passed; all 39 migrations current, seed completed |
| Independent read-only review | No remaining update/harness/documentation findings |

The [validation summary](validation-results.txt) records complete commands and
limits. [Cancellation output](cancellation-results.txt) retains the unchanged
backport's five PostgreSQL cases. Database-feature totals include ordinary tests
and are not additive. Four existing ignored cases are the Fantasy projection
benchmark and three PostgreSQL profiling probes. No new ignored test was added.
The validation example compiles as a zero-test Cargo target; its actual
assertions were executed directly by the opt-in harness.

The disposable database was PostgreSQL 17.10 Alpine/musl with `en_US.utf8`.
Existing collation-dependent test portability remains separately deferred.
No application/vendored source or lockfile changed after the successful full
ladder began. The validation probe's assertion/lint corrections were followed
by its final build, full strict Clippy and successful direct TLS execution.
The complete run's owned database and temporary comparison workspace were
removed after evidence capture.

Independent lock/source and final probe/documentation review found no remaining
actionable update issues. Review caught the harness prerequisite/cleanup gaps;
both were fixed and validated, including startup-failure cleanup. The existing
P3 `verify-ca` finding remains recorded for separate driver work. Reviewers ran
no builds or database/network commands.

## Limits and remaining work

The audit is not globally clean: remaining lockfile entries and yanked warnings
are retained with explicit reachability evidence. The advisory/version checks
are a dated snapshot. Changing dependency features or platforms requires
reassessment.

TLS probes cover actual transport and verification behavior, not hostile
handshake-record reproduction, capacity, public Caddy configuration or hosted
database settings. No production API key, real account, paid data, public-host
administration or database mutation against a hosted environment was used.
The release command runs on the host toolchain; this is not a production image
build or deployment claim. Frontend/browser suites were not rerun because this
step changes no frontend source, HTTP contract or UI state; the affected
client/server TLS paths are exercised directly.

The next security candidate remains the separately bounded A5 restore-checksum
repair. Other P2/P3 findings and operator-owned acceptance remain in
[the plan](../../PLANS.md).
