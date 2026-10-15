# Latest iteration — compatible development-dependency repairs

The compatible portion of A7 updates four development-only lock entries:
brace-expansion 1.1.18 to 1.1.21 and nested 5.0.9 to 5.0.12, js-yaml 4.3.1 to
4.3.2, and source-map-js 1.2.1 to 1.2.2. npm resolves these patches within the
existing parent ranges. Only version, registry URL and integrity fields change;
all other lock entries, the manifest, application source and tool configuration
remain unchanged. Fresh registry metadata matches the selected integrity values.

The full audit falls from six affected development packages to three, removing
all three high-severity package entries. It still exits 1: Vitest 3.2.7,
@vitest/mocker 3.2.7 and Tinypool 1.1.1 remain, labelled two critical and one
moderate. These are package-entry labels with inherited findings, not three
independent application exploits. The production-only audit remains zero. No
hostile-input exploit or production remote-execution chain was reproduced.

The remaining family requires a separately bounded Vitest major upgrade. Read-only
metadata for patched Vitest 4.1.11 overlaps the current Node 22/Vite 7 toolchain,
but that family has not been installed, resolved or tested here. The next step
must verify its resulting dependencies and current advisories before acceptance.
No incompatible override or blind force-update is included.

Clean `npm ci` passed for both original and repaired lockfiles. The repaired tree
also passes `npm ls --all`, all 1,133 tests across 147 files, strict typechecking,
ESLint and production build. All 65 local production-mode output files are byte-
identical before/after clean installs. The browser application therefore has no
changed local asset in this step. No browser/backend/PostgreSQL ladder or image/
hosted deployment acceptance is claimed for this dependency-only repair.

Independent lock/evidence review found no issues. The reviewer's separate registry
refresh was unavailable, so registry provenance rests on the successful primary
metadata checks and clean installations. Final documentation and bounded-plan
review found no issues.
The [A7 report](validation/development-dependencies-2026-10-15/README.md) retains
exact paths, versions, audit snapshots and validation. A7 remains partly open;
[the plan](PLANS.md) selects the major-family migration as the next candidate.
