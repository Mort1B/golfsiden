# Latest iteration — frozen Sunday release

DEPLOY-6 completes the six planned repository readiness iterations. The verdict
is **READY WITH KNOWN LIMITATIONS** for the exact locally accepted single-host,
Linux-amd64, schema-39 candidate. The
[release report](validation/release-freeze-2026-10-16/README.md) rates findings,
records remaining advisory conditions and identifies immutable images. The
[operator checklist](validation/release-freeze-2026-10-16/OPERATOR.md) provides
launch, upgrade, backup/restore and rollback actions. No hosted deployment ran.

The advisory review found confirmed public ingress P1/P2 gaps and PG17.10
security fixes missing from the previous runtime. Only three production
Dockerfiles changed: PG17.11, updated available Debian packages, and standard
Caddy2.11.7 built with Go1.27.2/x/net0.60.0/quic-go0.63.0. Stock Caddy2.11.7's
x/net0.59.0 was insufficient for the HTTP/2 fixes. Runtime base indexes are
digest-pinned. Actual binary metadata verifies the selected versions; the custom
Caddy CLI reports `unknown`, so its build-info and image ID identify the build.
All three Rust executables and all 65 web assets are byte-identical to the
accepted prior application. No business rule, schema, lockfile or UI changed.

The refreshed images passed 23 packaged/startup guards and a complete fresh
production/recovery sequence: 60 source and16 target checks, five ordered critical
browser phases, secure sessions/CSRF/native SSE, real schema/identity/grant
failure controls, and actual proxy502 secret/canary log filtering. The actual
current backup/restore scripts preserve all 61 public tables, 623 synthetic rows
and actual sequence state before API startup/login. Restored gross/net 73,
Fantasy 88/94, exact replay, API/web stream restart and DB outage/reconnect pass.
Real Chrome at 320/390/1280px has no unexpected console/network/body failures;
mobile and desktop screenshots were inspected. All owned runtime fixtures,
credentials, archives, aliases and API sockets were cleaned.

The original accepted images cannot be the frozen Sunday candidate because of
the repaired advisories. A fresh source build could also change package bytes.
The handoff therefore packages the exact three tested image archives plus
runtime Compose files, scripts and a manifest. The verifier checks image config
IDs, platform, archive/file digests and existing tag mappings before loading;
it explicitly creates Docker-compatible unqualified aliases. One common tag
also binds the migration/permission tools used by scripts. Startup disables
builds and pulls. An explicitly expected project preserves existing volumes and
allows isolated restore targets through consistent `COMPOSE_PROJECT_NAME`.
Ten transfer checks pass, including real load, collision/corruption rejection,
private config/password/project binding and optimized-Python rejection.

Fresh full/production npm audits report 0. Fresh Cargo audit retains four
optional Quinn/RSA entries and a yanked chacha20 warning; normal dependency
paths are absent in both native and all-feature/all-platform trees. The web
image has 0 high/critical matches. Remaining API/PG package-version matches are
conditionally triaged using actual inventory/configuration and primary sources;
the images are not scanner-clean. No additional configured P0–P2 path is
established. Fresh frontend 1,133 tests, typecheck, lint and build pass. Unchanged
Rust/domain/database suite evidence is reused explicitly; the full old PG suite
is not represented as rerun on 17.11. The affected runtime/PG/browser/recovery
ladder ran fresh on the new images. Read-only review findings are resolved.

Compose CLI 5.6.0 used rootless Podman 5.8.7 via owned private API sockets, not
native Docker. Public DNS/ACME/ports, the actual playing phone and protected
off-host restore remain operator launch gates. The P3 acknowledged Fantasy
read-retry limitation, unclassified earlier full-default lifecycle test failure
and expanded device limits remain explicit; the lifecycle failure's cause and
product severity remain unknown. No capacity or hosted security claim is made.
For example, an
acknowledged Fantasy entry whose view failed to refresh uses **Oppdater Fantasy**
for a read retry, preserving the single write.

No next implementation step is selected. Contingency iterations 7–8 are reserved
for a newly evidenced serious or unusable required-flow blocker. Optional
features, broad polish and speculative optimization remain deferred.
