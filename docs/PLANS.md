# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. The compatible portion of A7 is accepted in the
[development-dependency report](validation/development-dependencies-2026-10-15/README.md).
The Vitest-family findings remain open as the next candidate.

## Next candidate — requires approval

### A7b — P2: migrate the flagged Vitest family to a patched major

Evidence: [remaining audited paths](validation/development-dependencies-2026-10-15/README.md#fresh-audit-and-unresolved-findings).
The installed Vitest/mocker 3.2.7 and Tinypool 1.1.1 still produce three audit
entries (two critical package labels and one moderate). The maintainer does not
plan a 3.x mocker backport. Read-only metadata for Vitest 4.1.11 matches existing
Node 22/Vite 7 peer ranges; its resolved tree and compatibility remain untested.

- Goal: remove the remaining Vitest/mocker/Tinypool findings through a deliberate
  test-framework major upgrade while preserving application and test behavior.
- Scope: the Vitest development declaration/lockfile family, minimum required
  test/configuration migrations and evidence/docs. Refresh audits/registry and
  maintainer migration guidance before choosing a patched 4.x version at least
  4.1.11 with matching internal packages. Exclude Vitest 5, unrelated tool majors,
  application features/source changes, runtime dependency upgrades, hosted work
  and transitive overrides that violate parent ranges.
- Behavior: keep the existing Node 22/Vite 7 build boundary, strict typing,
  test isolation, mocks/timers and user-visible assertions. Resolve a coherent
  patched tool family; remove or patch affected Tinypool paths rather than hiding
  audit output. Preserve runtime package entries, API/UI contracts and produced
  application behavior. Do not weaken, delete or ignore tests to pass migration.
- Validation: clean lockfile-based install, installed peer/dependency check,
  focused mock/timer/jsdom compatibility cases, complete frontend unit/type/lint/
  build ladder, and fresh full/production-only audits. Verify no remaining
  Vitest/mocker/Tinypool advisory and record other newly reported findings without
  expanding scope. Compare local production assets; browser checks apply if
  runtime delivery or behavior changes. Independent read-only review must cover
  lock changes, any mechanical test migration, evidence and remaining limits.
- Invariants: preserve root scoring/handicap/locked-round rules, privacy/session
  handling, application behavior and the production dependency boundary.
- Stop: publish only the bounded Vitest 4 migration and evidence. Begin no other
  major upgrade, team-query optimization, course-read or driver work.

## Later queue

### Severity-ordered review follow-ups

All findings and individual acceptance criteria are in the
[review report](validation/application-review-2026-10-14/README.md#findings-and-acceptance-criteria).
Select one bounded step at a time; none is implemented by this plan.

1. **P3 / A8:** batch team memberships, preserving ordering and authority; measure
   query count and end-to-end cost. No current frontend latency impact is proven.

2. **P3 / A1 course reads:** bind the three lower-impact course GETs in a
   separately selected step; provider-success impact remains conditional in the
   [source inventory](validation/private-read-inventory-2026-10-14/README.md#courses).

Keep legacy team/round write-expiry inventory separate; the private-read inventory
never assessed those writes. Selecting the Vitest-family candidate does not authorize the later queue.

Only after these, consider measurement-only steps for remaining Fantasy
invalidation/fact-loading cost and empty-queue polling. No speculative optimization
or load claim is authorized.

### Previously deferred work

- P3 SQLx `verify-ca` compatibility: both original and updated TLS dependencies
  reject a trusted certificate's contextual hostname mismatch because SQLx core
  0.8.6 suppresses only the legacy error variant. Repair only in a separately
  bounded driver step; no hosted outage or new security regression was observed.
  See the [baseline comparison and limits](validation/rustls-update-2026-10-15/README.md#existing-p3-verify-ca-contextual-hostname-compatibility).

- P3 test-only collation portability: the unchanged A1d legacy-name ordering case
  assumes uppercase-first native collation. Debian/glibc and the repository's
  Alpine/musl PostgreSQL images disagree while production SQL retains native
  ordering. Adjust the test's contract in a separately bounded step; see
  [captured failure and comparison](validation/readiness-session-2026-10-14/README.md#existing-collation-dependent-test-limit).

- Retained leaderboard browser-test maintenance: update obsolete score-status
  assertions at `frontend/e2e/leaderboardLive.browser.ts:71,103` in a separate
  test-only step before using that suite as a release gate. See the
  [captured failure and focused A1 browser coverage](validation/leaderboard-session-2026-10-14/README.md#browser-evidence-and-remaining-test-maintenance).

- Retained A1 PostgreSQL expiry-test timing investigation: the unchanged
  score-loading expiry case timed out before observing its wait at default test
  concurrency, then passed in isolation and in the full two-thread ladder.
  Investigate its pre-gate expiry window in a separate test-only step; no precise
  cause or product regression was established. See the
  [captured failure and rerun evidence](validation/round-lifecycle-session-2026-10-14/README.md#validation).

- Optional offline preparation for browser closure/eviction: requires a separately
  scoped app-shell and private scorecard-retention/reauthorization design before
  implementation. Current in-memory return and durable queued edits remain distinct.

- Operator-owned security acceptance remains for the actual public proxy/TLS,
  runtime database privileges, image advisories and backup/restore environment.
  The local application assessment does not establish hosted readiness.

### Remaining deployment acceptance

Physical iPhone acceptance remains for Chrome and Safari separately: iPhone 15
Pro and a newer/larger model, both orientations, browser toolbars, software
keyboard, safe areas and native zoom. Local Chrome/WebKit coverage does not
replace those checks; see [the evidence and device checklist](validation/iphone-layout-2026-10-14/README.md#outstanding-physical-device-acceptance).
No further mobile implementation step is selected. Duo/foldables remain excluded.

The remaining PERSIST-1 same-account/actual-login scenarios may be revisited if
testing feedback warrants them; see the original
[validation limits](validation/match-note-retention-2026-09-23/README.md). Local
PostgreSQL validation is now available through rootless Podman. The functional
checks cover real database-backed match recovery after locking, but do not claim
every separate session-replacement scenario.

Public-host acceptance still needs a Docker Engine environment and public DNS/TLS
validation. Native 200% browser zoom and physical Android Chrome remain unverified.
Hosting and deployment checks belong to the user. These historical acceptance
limits do not expand the application-readiness step into server administration
or another assessment. Broader device coverage and polish can follow feedback.

No further sporting scoring rules are planned. No automatic opponents/byes/brackets, team match play,
extra holes, public match sharing, cold offline launch or background sync is included.
Fantasy remains separate from sporting scores and completion rules.
