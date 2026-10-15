# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. A6 acceptance is in the [password-admission report](validation/password-admission-2026-10-15/README.md).

## Next candidate — requires approval

### A7 — P2: compatible development-dependency repairs

Evidence: [original review A7](validation/application-review-2026-10-14/README.md#a7--p2-development-tool-advisories-need-a-separate-upgrade).
The original audit found development-only advisories; its snapshot does not
establish today's versions, reachability or production remote execution.

- Goal: remove currently confirmed advisories that can be fixed within the
  existing development-tool majors, preserving application behavior.
- Scope: frontend development dependency declarations/lockfile, necessary focused
  tool configuration and validation/docs. Refresh full and production-only audits
  before selecting exact compatible patched versions. Exclude runtime dependency
  changes, application features, hosted actions, blind force-updates and major
  tool migrations; document a separate candidate if a major migration is required.
- Behavior: preserve strict TypeScript, test isolation, lint/build semantics and
  existing API/UI contracts. Keep fixes confined to development tooling and the
  minimum necessary transitive updates. Distinguish scanner severity from an
  application exploit and record any remaining advisory with its actual path.
- Validation: clean lockfile-based installation, focused compatibility checks,
  complete frontend unit/type/lint/build ladder, refreshed full and production-only
  audits and independent read-only review. Browser checks apply if a required
  configuration change affects runtime delivery or browser behavior. Record exact
  versions, advisory snapshot, reachability, unresolved limits and skipped checks.
- Invariants: preserve root scoring/handicap/locked-round rules, private/session
  handling, source behavior and production dependency boundary.
- Stop: publish compatible development-tool repairs and evidence only. Begin no
  major migration, team-query optimization, course-read or driver work.

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
never assessed those writes. Selecting the development-tool candidate does not authorize the later queue.

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
