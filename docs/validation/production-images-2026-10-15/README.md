# DEPLOY-2 — production image and deployment contract baseline

**Image/build gate passed on rootless Podman; Docker Engine acceptance remains
unverified.** All three unchanged production Dockerfiles build successfully from
one immutable tracked candidate. The production API compiles on its actual Rust
1.88 image toolchain; no compiler, dependency or application repair was needed.
This is iteration 2 of the six-iteration Sunday plan, with no contingency used.

The local Docker API socket denies this user's access. The noninteractive sudo
probe requires a password. No socket permissions, groups, daemon configuration
or deployed services were changed. Rootless Podman performs the actual image
builds and isolated preflights; the installed Docker Compose CLI validates the
real Compose file offline. This does not establish Docker Engine runtime,
public-host, database-backed startup or HTTPS/browser/SSE acceptance.

## Candidate, artifacts and provenance

Source: `b73d13e7f3aa8ee3508e306fdbb0b20584595937`.
Context SHA-256:
`a32af6b17ec12f63abd5570d582c1ca49711a14a601de640aff0e1a8363ecea3`.
The runner extracts `git archive` into a private temporary directory, with no
working-tree/untracked secrets or files and no source overlay. Current build,
runtime, schema, dependency and deployment inputs still match that candidate.

All three images use tag `deploy2-b73d13e7f3aa-a32af6b17ec1`:

| Image in local rootless store | ID prefix | Build result |
| --- | --- | --- |
| `localhost/golfsiden-postgres` | `10318b2908d8` | Passed, 1.467s |
| `localhost/golfsiden-api` | `1a6524c27275` | Passed, 114.803s |
| `localhost/golfsiden-web` | `3482595fde95` | Passed, 39.148s |

[Build receipts](baseline-builds.json) retain full image IDs, local manifest
digests, creation times, Dockerfile hashes, source/context labels and cached
base-image IDs/digests. `--pull=never` deliberately uses those recorded cached
bases; these are not claims about the newest registry tags or image advisories.
Sanitized build diagnostics include ordinary layer progress and upstream source
warning locations. Successful build exit codes and exact image IDs determine
acceptance. Dependency locks and both SQLx overrides are unchanged.

Actual executable versions from the built images/base images are recorded in
[inspection.json](inspection.json): Rust 1.88.0, Node 22.23.2, npm 10.9.8,
PostgreSQL 17.10 and Caddy 2.10.2. Builds and checks are native amd64 only.

## Compose contract — 11 passing cases

[compose.py](compose.py) renders the actual `compose.production.yml` with a
private synthetic environment; it never prints the expanded graph or secrets.
The [receipts](compose.json) establish:

- Exactly five services, tools-profile migration/permissions actions, separate
  owner/runtime identities and URLs, and consistent shared proxy identity.
- Production API flags, secure cookies, startup migrations disabled and the
  readiness healthcheck remain configured.
- PostgreSQL is private on the internal data network. API has no published
  port; only web publishes TCP 80/443 and UDP 443.
- All ten required-variable omissions fail Compose interpolation with the
  expected variable-specific reason.

Compose accepts supplied values structurally; it does not replace image startup
guards or validate database permissions, public DNS/TLS or real proxy traffic.
The CLI version is 5.6.0. No Compose services were started.

## Packaged artifacts and guards — 23 passing probes

[preflight.py](preflight.py) creates each probe with network `none`, read-only
filesystem and image volumes ignored. It publishes no ports and removes only
its recorded container ID. No PostgreSQL cluster or Caddy server is started.

- API runs as UID 10001. API, migrate and password-recovery are executable with
  all dynamic libraries found; curl is present. Seed and the build source tree
  are absent. Both operator binaries execute and reject missing prerequisites.
- The real API binary rejects insecure production cookies, startup migration,
  malformed proxy secret, missing runtime identity and port zero. Secret-error
  output remains redacted.
- The PostgreSQL production wrapper accepts valid distinct credentials for a
  `postgres --version` call and rejects equal identities/passwords, placeholders,
  short passwords and a missing password. Version mode does not initialize data.
- Web contains 65 built assets, including a nonempty index, with no build source
  or node_modules. [Asset hashes](web-assets.json) retain packaged file identity.
- The real Caddy config adapts successfully, with the expected `api:3000`
  upstream, overwritten client/proxy headers and negative flush interval for
  unbuffered streaming. The web wrapper rejects invalid hostnames/secrets and
  a missing secret. This is config acceptance, not a live HTTP/SSE result.
- Four version probes establish the actual builder/runtime executables above.

An initial [harness correction](harness-correction.json) was necessary: the helper
filename shadowed Python's standard `inspect` module and was renamed, with its
owned bytecode removed. A missing-variable shell guard returns exit 2 while the
first assertion required 1. The final check requires exit 1/2 plus the exact
expected rejection reason. No production failure was inferred from that harness
mistake, and the complete corrected 23-probe sequence passed.

## Audit, review and validation scope

[audit.py](audit.py) verifies the exact local image IDs/source labels, all three
builds, all 11 Compose cases, all 23 probes, asset count, source equality and
cleanup. [Postconditions](postconditions.json) all pass. The final inventory finds
no remaining probe containers carrying the owned DEPLOY-2 label. Private contexts,
logs and environment files are removed; three explicitly tagged images are
retained for the next rehearsal. No unrelated containers/images/volumes are removed.
[Availability](availability.json) records the actual Docker permission limit and
rootless builder architecture.

Independent read-only review covers the image/config boundary and the final
harness/evidence. No application, Dockerfile, Compose, migration or dependency
change is selected. The affected image builds and preflights ran; unaffected
full Rust, PostgreSQL, frontend-test and browser ladders were not repeated. Prior
[Rust/database acceptance](../leaderboard-expiry-timing-2026-10-15/README.md#validation)
and [frontend/browser acceptance](../leaderboard-browser-maintenance-2026-10-15/README.md)
remain separately scoped evidence, not fresh runs or full Docker Engine sign-off.

## Reproduction and next gate

```sh
python3 docs/validation/production-images-2026-10-15/build.py baseline
python3 docs/validation/production-images-2026-10-15/compose.py
python3 docs/validation/production-images-2026-10-15/preflight.py baseline
python3 docs/validation/production-images-2026-10-15/audit.py
```

Use a checkout at the source revision above to reproduce the same context.
Builds refuse to replace existing candidate tags; use the audit for the already
built images or a separate owned image store for a fresh same-candidate build.
The runner permits only explicit in-scope source overlays for an evidenced
repair; this run uses none. It never pushes images to a registry. Public package
fetches in the Dockerfiles still occur as needed; cached base IDs are recorded.

DEPLOY-3 will use the matching artifacts in an isolated production-mode Compose
rehearsal: real schema 39 migration/grants, least-privilege runtime startup,
readiness, secure-cookie browser access and SSE through Caddy. A temporary local
Podman API socket may provide Compose-compatible rehearsal where Docker access
is unavailable; that result must remain distinct from native Docker Engine
acceptance. No host permissions or daemon changes are part of that fallback.

Actual host/Docker access, install versus upgrade, host architecture, DNS/TLS,
off-host backups, playing-phone acceptance and current-image advisory triage
remain release checks. Integrated rehearsal and current-schema restore remain
open; this step is build/contract acceptance, not final production readiness.
