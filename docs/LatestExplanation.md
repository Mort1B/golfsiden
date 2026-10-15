# Latest iteration — production images and deployment contract

DEPLOY-2 builds all three unchanged production Dockerfiles from immutable source
b73d13e and one tracked-only context. PostgreSQL, API and web images succeed on
rootless Podman/amd64. The API compiles on the actual Rust 1.88 image toolchain;
frontend builds use Node 22.23.2/npm 10.9.8. No source, compiler, dependency,
Dockerfile, Compose or schema repair was needed.

Eleven offline checks of the real Compose file verify production settings,
private database/API exposure, owner/runtime configuration and all ten required
variable failures. Twenty-three real-image preflights pass with network disabled,
read-only filesystems and no image volumes. API UID10001, required binaries and
dynamic libraries, no seed/source, 65 built web assets and actual startup guards
are checked. Caddy config adaptation preserves the proxy identity headers and
negative flush interval; it does not establish live HTTP or SSE behavior.

The preflight harness needed one correction: its initial name shadowed Python's
standard inspect module, and its missing-variable shell assertion expected exit
1 while the shell returned 2. The renamed helper requires a nonzero 1/2 exit plus
the exact denial reason. The complete corrected probe sequence and final receipt
identity/source/cleanup audit pass. No image or product defect was inferred.

This workspace cannot access its local Docker API socket; noninteractive sudo
requires a password. No socket, group or daemon change was made. Podman image
acceptance and offline Docker Compose validation remain distinct from native
Docker Engine execution. Recorded cached base IDs were used, not claims about
latest registry tags or image security. Actual database initialization/grants,
production API startup, HTTPS/browser cookies and live SSE remain next-step gates.

All 23 final owned probe containers and private contexts/logs/configuration are
removed. Three uniquely tagged images with exact IDs and source/context labels
remain for the next isolated rehearsal. Production build/runtime inputs still
match the tested candidate. Unaffected full Rust, PostgreSQL, frontend-test and
browser ladders were not repeated; actual release image builds and their scoped
preflights were run. The [report](validation/production-images-2026-10-15/README.md)
retains versions, IDs, receipts, harness correction, review and limits.

Two of the six planned Sunday iterations are complete, with no contingency used.
DEPLOY-3 is bounded to isolated production integration and real proxy/database
acceptance; no hosted deployment or new feature is authorized. The operator's
install/upgrade choice, actual host architecture/Docker access, DNS/TLS, off-host
backups and playing-phone checks remain explicit prerequisites.
