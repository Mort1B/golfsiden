# Patched Vitest major migration — 2026-10-15

P2 A7b closes the Vitest-family findings left by the
[compatible A7 repair](../development-dependencies-2026-10-15/README.md).
Baseline: `a7fb638454384669753b1f37972753c3c05f035e`.

## Exact migration

The manifest changes only `vitest: ^3.2.4` to `^4.1.11`. The lock selects Vitest
4.1.11 and all seven internal Vitest packages at that same version. The
[lock comparison](lockfile-changes.json) records 26 changed entries including the
root declaration: 13 updated, two added and ten removed development packages.
The removals include Tinypool and vite-node. No affected Tinypool path remains.

Chai 6, es-module-lexer 2, std-env 4, tinyexec 1 and tinyrainbow 3 are required
transitives of the selected Vitest family. They are not additional direct-tool
upgrades. All ten application runtime package entries, Vite 7.3.6 and the four
prior A7 patch entries remain identical. No Vitest 5, override, application source,
test source, script, TypeScript/ESLint/Vite/Playwright configuration, Rust lockfile,
backend or schema change is included.

Fresh [registry metadata](registry-vitest.json) confirmed 4.1.11 as the latest
available stable 4.x at selection, its integrity and Node/Vite ranges. Node
22.22.2 and the existing Vite 7 satisfy those requirements. The
[maintainer advisory](https://github.com/vitest-dev/vitest/security/advisories/GHSA-82fw-gwwq-j7x9)
lists 4.1.11 as patched. The [version-4 migration guide](https://v4.vitest.dev/guide/migration)
was used because the default site now documents version 5. It identifies mock
restoration/constructor changes, broader default discovery and replacement of
Tinypool/vite-node. Existing tests/configuration were compatible and need no
mechanical edits.

## Resolver provenance

The original npm 10.9.7 crashed inside `loadPeerSet` with
`Cannot read properties of null (reading 'edgesOut')` while generating a major-
upgrade proposal. Direct, manifest-first, explicit-root-Vite/dedupe and installed-
baseline attempts all failed in isolated temporary copies; no failed proposal
was applied to the worktree. [Attempt output](resolver-attempts.txt) retains the
observed errors. [Upstream npm issue 9787](https://github.com/npm/cli/issues/9787)
reports a similar optional-peer resolver failure; an identical root cause was
not established here.

A verified, temporary npm 11.21.0 CLI generated the lockfile through `npm exec`
with normal peer validation, no lifecycle execution and deduplication preference.
It was used only for proposal generation; the temporary manifest/lock workspace
was removed. The project/global npm version remains 10.9.7; npm 11 is not a
project dependency or new installation requirement. [Resolver results](resolver-results.json) and
[CLI metadata](registry-resolver.json) retain the exact command and provenance.

Standard npm 10.9.7 subsequently completed clean `npm ci`, installed-tree
validation and every frontend check. No `--force`, `--legacy-peer-deps`, graph
hand-editing or incompatible override was used. This checks the repository's
normal clean-install path rather than requiring the temporary resolver in CI.

## Fresh audits

| Snapshot | Full lockfile audit | Production-only audit |
| --- | --- | --- |
| Before | [Three affected packages](audit-before.json): two critical labels and one moderate; exit 1 | [Zero](audit-production-before.json); exit 0 |
| After | [Zero](audit-after.json); exit 0 | [Zero](audit-production-after.json); exit 0 |

The removed entries were Vitest's aggregate, @vitest/mocker and Tinypool. The
critical labels included inherited findings, not independent reproduced public
application exploits. No hostile-input exploit was replayed. These are dated npm
dependency snapshots, not proof that all software, production images, Rust
packages or the hosted application are vulnerability-free.

The frontend image still builds under Node and copies only `dist` into Caddy.
Its runtime dependency boundary is preserved, and the local built assets are
unchanged. No image build, image advisory scan or hosted action occurred.

## Compatibility validation

| Check | Result |
| --- | --- |
| Original clean npm 10 installation, tests and production build | Passed; 147 files / 1,133 tests |
| Patched clean npm 10 installation | Passed; 263 platform-installed packages |
| Installed dependency/peer tree | `npm ls --all` exit 0 |
| Focused mock/timer/jsdom compatibility | 11 files / 176 tests passed |
| Complete Vitest 4 suite | 147 files / 1,133 tests passed |
| Exact discovery comparison | Same file, ancestor and case-title multiset; all passed before/after |
| Strict typecheck | Passed |
| ESLint | Passed |
| Production build | Passed |
| Local production output comparison | All 65 files byte-identical |
| Final full/production-only audits | Both zero, exit 0 |
| Independent read-only review | No manifest/lock/evidence/documentation/next-plan findings |

The focused cases retain fake timers/request timeouts, sharing/session fences,
prepared score recovery, module mock factories, account invocation ordering,
EventSource ownership and score-page state. They are existing tests, not new
assertions mirroring the framework implementation. No tests were weakened,
deleted, renamed or ignored.

[Baseline results](baseline-results.json) and [frontend results](frontend-results.json)
retain exact commands and timings. [Discovery comparison](test-discovery-comparison.json)
compares normalized file/ancestor/title identities, not only totals; no case is
pending or todo. [Build comparison](build-comparison.json) retains per-file SHA-256
under the same local source/configuration/environment after clean installations.
No artifact content, request payload or environment dump is retained.

Browser checks were not rerun because application sources, runtime delivery
configuration and every local production asset are unchanged. Backend/PostgreSQL
ladders are unaffected and not rerun. This is local test/build evidence, not a
browser, production-image, hosted-readiness or performance acceptance claim.

## Stop condition

Publish only the patched Vitest family and evidence/docs. Team member batching
remains a separately selected P3 candidate in [the plan](../../PLANS.md); no query,
course-read, driver or other major upgrade is included.
