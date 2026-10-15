# Compatible development-dependency repairs — 2026-10-15

This is the compatible-patch portion of P2 [A7](../application-review-2026-10-14/README.md#a7--p2-development-tool-advisories-need-a-separate-upgrade).
Baseline: `88e80f0e8e4891c5f7988a9c8137391d80b6dfe3`.
The remaining Vitest-family findings require a separately approved major upgrade;
A7 as a whole is not closed.

## Exact repair

Only four `frontend/package-lock.json` entries change, each only in `version`,
`resolved` and `integrity`. All remain development-only and satisfy their existing
parent ranges. The package manifest, scripts, every other lock entry, frontend
source, TypeScript/ESLint/Vite/Playwright configuration, backend, schema and Rust
lockfile are unchanged. No override or forced dependency update is used.

| Installed path/package | Before | After | Parent requirement |
| --- | --- | --- | --- |
| Root brace-expansion | 1.1.18 | 1.1.21 | minimatch 3.1.5: `^1.1.7` |
| typescript-estree's nested brace-expansion | 5.0.9 | 5.0.12 | minimatch 10.2.6: `^5.0.8` |
| js-yaml | 4.3.1 | 4.3.2 | @eslint/eslintrc 3.3.6: `^4.3.0` |
| source-map-js | 1.2.1 | 1.2.2 | PostCSS 8.5.26 / css-tree 3.2.1: `^1.2.1` |

The [lock comparison](lockfile-changes.json) records exact paths and integrity
values. Fresh `npm view` metadata confirmed each version, digest, dependency
requirements and engine compatibility; Node 22.22.2 satisfies the selected engine
ranges. npm generated the proposal in a temporary manifest/lock copy before the
four-entry diff was applied. That temporary directory was removed. Clean `npm ci`
then installed the exact lockfile successfully, and `npm ls --all` passed.

[Before](reachability-before.json) and [after](reachability-after.json) parent
inventories show these are tool dependencies. A parent's `type: prod` describes
its ordinary dependency edge; the installed project node remains `dev: true`.
This does not turn that package into an application runtime dependency.

The selected brace versions address the current nested-recursion and quadratic
expansion ranges; [the maintainer's patch advisory](https://github.com/juliangruber/brace-expansion/security/advisories/GHSA-q2hr-2g5m-vwhr)
lists 1.1.21 and 5.0.12. The other patch ranges are recorded in the
[js-yaml advisory](https://github.com/advisories/GHSA-2883-xcg3-v3hh) and
[source-map-js advisory](https://github.com/advisories/GHSA-68fv-2mgg-jv7q).
This is version/advisory and compatibility evidence; no hostile-input exploit or
production denial of service was reproduced.

## Fresh audit and unresolved findings

| Snapshot | Full npm audit | Production-only npm audit |
| --- | --- | --- |
| [Before](audit-before.json) | Six affected packages: two critical, three high, one moderate; exit 1 | [Zero](audit-production-before.json); exit 0 |
| [After](audit-after.json) | Three affected packages: two critical, zero high, one moderate; exit 1 | [Zero](audit-production-after.json); exit 0 |

These are npm's package-entry severity labels, not six or three independent
exploits. In particular, Vitest inherits Tinypool's critical label. The full
audit deliberately remains nonzero. The production-only result is dependency
scanner evidence, not an image, compiled-artifact or hosted vulnerability audit.

The unresolved installed paths are unchanged:

- `vitest` 3.2.7: critical aggregate entry, including inherited findings.
- `@vitest/mocker` 3.2.7: moderate redirect-mock file-read entry.
- `tinypool` 1.1.1: critical entry covering two prototype-pollution gadgets.

The [Vitest maintainer advisory](https://github.com/vitest-dev/vitest/security/advisories/GHSA-82fw-gwwq-j7x9)
requires particular exposed development-server mock registrations; it states that
older 3.x releases will not receive this fix. The two Tinypool advisories require
an upstream prototype-pollution primitive: [worker options](https://github.com/tinylibs/tinypool/security/advisories/GHSA-5gmw-xhrv-c9v3)
and [supplied run options](https://github.com/tinylibs/tinypool/security/advisories/GHSA-85c8-ppgw-ccpr).
No such application exploit chain was reproduced. Scanner critical labels do not
establish public application remote execution.

The production frontend Dockerfile runs Node/npm only in its builder and copies
`dist` into the final Caddy image. Its application runtime package entries and
local built assets remain unchanged in this step. Build/test tooling still needs
the separately planned Vitest upgrade; the final-image boundary does not erase
that maintenance requirement.

Only read-only [candidate metadata](major-candidate-metadata.json) was retrieved
for patched Vitest 4.1.11. Its Node/Vite peer ranges overlap the existing Node 22
and Vite 7 toolchain. It was neither installed nor resolved here, and its actual
resulting tree and test compatibility remain unverified. A fresh audit must gate
that separate major upgrade rather than blindly accepting npm's 5.0.4 suggestion.

## Validation

| Check | Result |
| --- | --- |
| Original lockfile clean installation and production build | Passed |
| Patched lockfile clean installation | Passed; 271 platform-installed packages |
| Installed dependency tree | `npm ls --all` passed |
| Full frontend unit suite | 147 files, 1,133 tests passed |
| Strict typecheck | Passed |
| ESLint | Passed |
| Production build | Passed |
| Local before/after production artifacts | All 65 files byte-identical |
| Fresh full/production-only audits | Exact unresolved findings above |
| Independent read-only review | No lock/evidence/documentation/next-plan findings |

The reviewer's separate registry refresh was unavailable. Registry provenance
rests on the primary's captured fresh `npm view` checks and successful clean
installations; the reviewer independently checked the lock comparison and
recorded evidence, not a second registry fetch.

[Frontend results](frontend-results.json) retain commands and timings.
[Build comparison](build-comparison.json) records every emitted file's SHA-256
under the same source, configuration and local environment before/after clean
installs. This is local production-mode build evidence, not a container build or
hosted deployment check. No asset content is copied into the report.

Browser checks are not rerun: application source, runtime delivery configuration
and all local production assets are unchanged. Backend/PostgreSQL ladders are
unaffected and not rerun. No test/configuration was weakened, new ignored test
added or unrelated source repaired.

## Stop condition

Publish only the compatible four-entry patch and evidence/docs. Leave the
Vitest-family major migration as P2 A7b in [the plan](../../PLANS.md); do not start
it, query optimization, course-read repair or hosted work in this step.
