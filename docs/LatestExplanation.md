# Latest iteration — patched Vitest 4 family

A7b completes the remaining development-dependency repair. The only manifest
change is Vitest `^3.2.4` to `^4.1.11`; the lock selects matching 4.1.11 packages
and their required development dependencies. Tinypool and vite-node are removed.
Every application runtime entry, Vite 7.3.6 and the earlier A7 patches remain
identical. No application, test, script or tool-configuration edits were needed.

Fresh full and production-only npm audits both report zero findings. The prior
three development package entries (two critical labels and one moderate) are
absent. This closes the audited Vitest family, not an assertion that every image,
Rust dependency or hosted application is vulnerability-free. No exploit was
replayed or production workload measured.

The normal npm 10 resolver crashed during isolated proposal generation. A
verified temporary npm 11 CLI generated the lockfile with peer checks intact;
project/global npm stayed at 10.9.7. Standard npm 10 then passed clean `npm ci`,
installed-tree validation and all frontend checks. No force/legacy-peer flag,
transitive override or global toolchain change was used.

The clean Vitest 3 baseline and Vitest 4 run both pass exactly the same 1,133 test
identities across 147 files. A focused set of 176 existing tests covers module
mocks, fake timers, jsdom, request timeouts, identity ordering and live ownership.
Strict typecheck, ESLint and production build pass. All 65 local production
output files are byte-identical after clean before/after installs. No test was
weakened, renamed, deleted or ignored.

Independent manifest/lock/evidence review found no issues and independently
confirmed test identity and build-hash parity. Final documentation and bounded-
plan review also found no issues. Browser/backend/PostgreSQL ladders and image/hosted acceptance are outside
the affected boundary and were not rerun. The [Vitest report](validation/vitest-major-2026-10-15/README.md)
records commands, resolver provenance, audit snapshots and limits. The next
candidate in [the plan](PLANS.md) is P3 team-membership query batching.
