# DEPLOY-6 — frozen release and Sunday handoff

> Historical bundle: superseded by the accepted
> [replacement frozen delivery](../release-refreeze-2026-10-16/README.md), which
> contains the award-denial exit and Fantasy four-ball lock guard. Use that
> handoff for the current release. The older identities/evidence below remain
> unchanged and do not contain those application fixes.


**READY WITH KNOWN LIMITATIONS** for the locally accepted single-host,
Linux-amd64, schema-39 candidate, as of **16 October 2026**. All six planned
repository readiness iterations are complete. No configured P0–P2 finding
remains established by this review. This is conditional local release acceptance;
native Docker, the public host, physical playing phone and off-host restore
remain required operator checks. No hosted deployment was performed.

Use the [operator checklist](OPERATOR.md) and
[deployment guide](../../deployment_guide.md). Exact image IDs and file/archive
checksums are in [manifest.json](manifest.json); the staged package path and
SHA-256 are in [bundle.json](bundle.json). Copy that temporary package to durable
operator storage. The package contains the exact tested images, runtime Compose
files, existing backup/restore scripts, example env and verifier. There are no
runtime credentials, browser profiles, database dumps or accounts in the bundle.
Images were not pushed to a registry. A rebuild is a new candidate.

## Severity-rated findings and disposition

| Severity | Finding | Result |
| --- | --- | --- |
| P1 | Public HTTP/2 HPACK encoder crash and SETTINGS CPU exhaustion in old Caddy's Go/x/net runtime | Repaired: compiled Go1.27.2 **and** x/net0.60.0, verified in the actual binary; refreshed full affected acceptance |
| P2 | Public TLS KeyUpdate, HTTP/3 QPACK allocation and static Range parsing CPU advisories in old ingress | Repaired by the Caddy/Go/quic refresh; actual quic-go0.63.0 |
| P2 | PostgreSQL17.10 lacks 17.11 security fixes, including an SQL-user prerequisite for internal-type code execution | Repaired: actual 17.11; no claim the private DB was directly exploitable through a web request |
| P3 | Rebuilding from mutable package repositories cannot reproduce the accepted image bytes | Closed delivery gap: exact image archives, manifest, all-five-service no-pull override and protected alias loader |
| P3 retained | Residual package-version advisories in unused/conditional runtime tools and optional lock entries | Reviewed below; keep inventory, update during maintenance or reassess when configuration changes |
| P3 retained | Acknowledged first Fantasy entry can require **Oppdater Fantasy** after failed read reconciliation | Read retry preserves the acknowledged write; fresh entry/recovery acceptance submits one entry write |
| Unclassified validation limitation | Earlier unexplained full-default lifecycle test failure | Cause and product severity remain unknown; focused controls do not diagnose scheduling or prove a repair; no automatic maintenance step opened |
| Operator gates | Native Docker/host architecture, public DNS/ports/ACME, actual playing phone, protected off-host restore | Unperformed here; complete before launch, stop if a required check fails |

The ingress classifications use configured HTTP/2, default HTTP/3/TLS and static
file serving, not scanner severity alone. No malformed-protocol exploit was run.
Primary conditions and fixes: [HPACK](https://pkg.go.dev/vuln/GO-2026-6617),
[SETTINGS](https://pkg.go.dev/vuln/GO-2026-6611),
[Range](https://pkg.go.dev/vuln/GO-2026-6609),
[TLS KeyUpdate](https://pkg.go.dev/vuln/GO-2026-4870),
[later KeyUpdate](https://pkg.go.dev/vuln/GO-2026-6090),
[QPACK allocation](https://github.com/quic-go/quic-go/security/advisories/GHSA-g754-hx8w-x2g6),
[later QUIC fix](https://github.com/quic-go/quic-go/security/advisories/GHSA-vvgj-x9jq-8cj9),
[PostgreSQL internal type](https://www.postgresql.org/support/security/CVE-2026-14680/)
and [17.11 release notes](https://www.postgresql.org/docs/17/release-17-11.html).

Stock Caddy2.11.7's [module file](https://raw.githubusercontent.com/caddyserver/caddy/v2.11.7/go.mod)
still selected x/net0.59.0 and its
[server](https://raw.githubusercontent.com/caddyserver/caddy/v2.11.7/modules/caddyhttp/app.go)
uses that HTTP/2 implementation. The production Dockerfile therefore builds
Caddy2.11.7 with explicit Go1.27.2 and x/net0.60.0, CGO disabled and standard
modules only. Runtime base indexes are digest-pinned. The API's Debian runtime
also receives current available package updates. No application source, schema,
lockfile, Caddy configuration, grant or sporting rule changed.

## Exact candidate and verification

Source is Git `7cf94bb9b40ada7ebb9b24264048a5935fd85676` plus exactly the three
Dockerfile overlays whose SHA-256 values are in
[candidate-builds.json](candidate-builds.json). Build context SHA-256 is
`ed3fe691022c05fec4a0ca814e2ef4fe4e1726b05437555be253ac42686389d7`.
The common delivery tag is `release-20261016-ed3fe691022c`.

| Service | Image config SHA-256 |
| --- | --- |
| PostgreSQL17.11 | `6f5206f425ce432de2d4d057905caa25c107be89ca7fe9763d6b34f141f77a4c` |
| API | `94bbd82bf4c7e7fb83a04b34102c7aa04e7e55aae44cbaf7dbd48410be1c1703` |
| Web | `83a94ebcf1b4f16aad05d4301661d2d9d12bc7adb0295f4cd4e3287dbc5be0ae` |

[Content comparison](content-equality.json) proves all three Rust executables
and all 65 web files byte-identical to the previous accepted build. Only runtime
components changed. [Preflight](preflight.json) passes 23 packaged/startup guards,
including actual PG17.11, Rust1.88, Node22/npm10, no seed/source in runtime,
rejected insecure configuration and real Caddy config adaptation. The custom
binary's `caddy version` prints `unknown`; its
[build metadata](candidate-web-build-info.txt) identifies module v2.11.7,
Go1.27.2, x/net0.60.0, quic-go0.63.0 and all 135 standard modules.

The fresh [complete sequence](sequence.json) on those exact images includes:

- [Source acceptance](source-cleanup.json): 60 named checks, including five ordered
  [critical-flow phases](source-flows.json) with 25 browser checks and seven
  [additional proxy/browser boundary checks](production-browser.json).
  Invitations, claim, setup, scoring, exact uncertain-write replay, offline
  conflict choice, gross/net, correction/reconfirmation, locked writes,
  two-round Fantasy 88/94, final privacy, restricted identity/grants and all
  schema readiness rejection controls pass. Actual forced proxy502 logs omit
  generated cookie/CSRF/URI canaries and the proxy secret.
- Actual current backup/restore scripts, independent empty target, explicit
  confirmation, corrupted sidecar, late work/transaction rollback and populated
  target rejection. [Parity](parity.json) covers all 61 public tables, **623
  synthetic rows**, row SHA-256 and actual sequence state before API/login.
- [Target acceptance](target-cleanup.json): 16 named checks, including
  [restored browser results and native stream restart](browser.json), followed
  by [fresh login/reads after DB outage/reconnect](browser-post-database.json).
  Restored gross/net 73, Fantasy 88/94, carry-forward and exact receipt replay pass.
- Real Chrome 155 at 320/390/1280px, no unexpected console/network/body failures.
  Fresh [mobile](restored-player-390.png) and
  [desktop](restored-organizer-1280.png) captures were inspected.
- [Transfer controls](transfer.json): offline archive config IDs and checksums,
  actual image load and explicit aliases, conflict rejection before replacement,
  changed-runtime-file rejection, private script-only five-image/role/password
  binding, wrong tag/project rejection, explicit isolated project support and
  rejection of optimized Python. No application was started by transfer checks.

Production acceptance uses Compose CLI 5.6.0 against rootless Podman 5.8.7 via
owned private API sockets. Native Docker socket access is denied and sudo-n is
unavailable. No group/daemon/trust change was made. Local certificate exceptions
prove encrypted proxy behavior, not public trust or real port 80→443 completion.
Both source/target projects (each 3 containers, 3 volumes, 2 networks), generated
credentials/archives/CA/browser profiles and temporary aliases were removed;
the previous four images and new three are retained. See
[ownership](ownership.json) and final [postconditions](postconditions.json).

The unchanged full Rust/domain/PG test evidence is retained from prior steps;
it is **not relabeled as a full database suite on 17.11**. The complete affected
runtime/PG migration/grant/browser/recovery ladder ran fresh on 17.11. Fresh
frontend test/type/lint/build results are in [checks.json](checks.json).
No throughput/capacity improvement is claimed; speculative performance work
remains deferred.

## Current advisory triage and limits

Trivy 0.76.0's archive SHA-256 matches both official checksums and GitHub asset
metadata ([scanner](scanner.json)); its DB was updated 16 October at 01:13 UTC
([database](scanner-db.json)). Unfiltered raw reports are retained. Counts are
package/version matches, not distinct exploitable findings:

| Image | Previous high / critical | Candidate high / critical |
| --- | --- | --- |
| API | 74 / 4 | 69 / 1 |
| PostgreSQL | 33 / 1 | 24 / 1 |
| Web | 81 / 6 | **0 / 0** |

Candidate reports: [API](candidate-image-api.json),
[PostgreSQL](candidate-image-postgres.json), [web](candidate-image-web.json).
The scanner feed does not replace direct primary-source version checks.

The API's 70 high/critical matches represent 19 unique CVEs. The critical
[zlib/minizip match](https://security-tracker.debian.org/tracker/CVE-2023-45853)
is explicitly not built into Debian Bookworm zlib binaries. Five curl CVEs
require conditions absent from the fresh CLI fixed loopback-HTTP readiness
request: [non-CLI cookie](https://curl.se/docs/CVE-2026-6276.html),
[non-CLI proxy](https://curl.se/docs/CVE-2026-8927.html),
[schemeless SSH](https://curl.se/docs/CVE-2026-12064.html),
[STARTTLS reuse](https://curl.se/docs/CVE-2026-8286.html) and
[Negotiate reuse](https://curl.se/docs/CVE-2026-8458.html).
Six util-linux CVEs concern mount hooks/privileged mounts, namespace entry or
partition probing, absent from UID10001 API operation:
[mount advisory](https://github.com/util-linux/util-linux/security/advisories/GHSA-8gj5-72r3-428g),
[partition parser](https://github.com/util-linux/util-linux/security/advisories/GHSA-h4rw-gv36-wmp5),
[namespace prerequisite](https://security-tracker.debian.org/tracker/CVE-2026-78408).
The Rust executable links libc/libm/libgcc, with provider/database TLS via Rustls;
it does not invoke those helpers or the following parser paths:
[OpenSSL suspended DTLS](https://openssl-library.org/news/secadv/20260929.txt),
[systemd-homed](https://security-tracker.debian.org/tracker/CVE-2026-16742),
[infocmp](https://security-tracker.debian.org/tracker/CVE-2025-69720),
[crafted gzip formats](https://security-tracker.debian.org/tracker/CVE-2026-41992),
[Perl Archive::Tar](https://security-tracker.debian.org/tracker/CVE-2026-9538),
[privileged ACL paths](https://security-tracker.debian.org/tracker/CVE-2026-54369)
or [LDAP BER](https://security-tracker.debian.org/tracker/CVE-2023-2953).

All 25 PostgreSQL high/critical matches are Go1.24.6 metadata in gosu1.19,
not the PostgreSQL server. Its reviewed
[main](https://raw.githubusercontent.com/tianon/gosu/1.19/main.go) and
[credential switch](https://raw.githubusercontent.com/tianon/gosu/1.19/setup-user.go)
read local identity, drop privileges and exec; the implicated network/archive/
template parsers are not exercised. This retains helper maintenance risk.
Web's one medium [native zlib issue](https://security-tracker.debian.org/tracker/CVE-2026-85091)
requires nonblocking gzip writes plus gzprintf; Caddy's CGO-disabled Go binary
does not use native zlib. Its unknown
[OpenPGP deprecation](https://pkg.go.dev/golang.org/x/crypto/openpgp) has no
configured input path. These are conditional dispositions, not universal proof
of unreachability under arbitrary execution or future overrides.

Fresh [full](npm-audit.json) and [production](npm-production-audit.json) npm
audits both report 0. [Cargo audit](cargo-audit.json) still exits 1 with four
optional Quinn/RSA vulnerability entries and yanked chacha20; all six native
and all-feature/all-platform normal dependency-tree checks show those packages
absent ([reachability](rust-reachability.json)). No lockfile repair was selected.

No physical iPhone/Android, cold offline launch, browser-closure preparation,
background sync, public certificate issuance, native Engine or off-host recovery
is claimed. Existing device and lifecycle limitations remain linked in
[the plan](../../PLANS.md). A new serious or unusable required-flow finding
reopens only its bounded contingency step; optional work does not delay Sunday.
