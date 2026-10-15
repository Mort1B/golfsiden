# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. A8 acceptance is in the
[team-loading report](validation/team-loading-2026-10-15/README.md).

## Next candidate — requires approval

### A1n — P3: bind the three course GETs to complete read authority

Evidence: the remaining [course-read source inventory](validation/private-read-inventory-2026-10-14/README.md#courses).
These expose global bundled/registered course facts; successful provider impact
is conditional. No private-score disclosure or reachable provider-quota abuse
was established by that assessment.

- Goal: retain the actual session and exact tournament-admin authority through
  each selected course response, with a final wall-clock check before success.
- Scope: only course catalog, registered presets and provider-course detail GETs,
  their narrow repository/authority helpers, focused tests and docs. Exclude
  course writes, other route families, frontend/DTO/schema changes, catalog or
  provider credential changes, hosted/provider administration and performance work.
- Behavior: derive authority from the locked session and stored membership,
  preserve payloads, ordering and provider availability/error contracts, and use
  private/no-store responses on all selected-route statuses. Keep local dependent
  reads/assembly in their owning transaction and recheck expiry even for empty
  results. Do not hold database locks across external I/O; separately authorize
  returned provider facts. Do not replay remote fetching or cache writes as a
  database-read retry. Any retry must discard the failed local snapshot's facts.
- Validation: begin with failing revocation/credential/expiry regressions for the
  three actual GET paths; cover exact-admin membership changes, populated/empty
  payload parity, private errors and conditional provider/cache behavior using
  local doubles. Run affected backend/PostgreSQL ladders, migrate/seed and
  independent review. No external provider or hosted acceptance claim.
- Invariants: preserve tournament membership isolation, course fact ownership,
  all handicap/score rules and existing write boundaries.
- Stop: publish only this course-read boundary and evidence; begin no adjacent
  write-expiry inventory, driver/test maintenance or other optimization.

## Later queue

### Severity-ordered review follow-ups

All findings and individual acceptance criteria are in the
[review report](validation/application-review-2026-10-14/README.md#findings-and-acceptance-criteria).
Select one bounded step at a time; none is implemented by this plan.

The remaining P3 course GETs are the next candidate above. Keep legacy team/
round write-expiry inventory separate; the private-read inventory never assessed
those writes. Selecting that read boundary does not authorize adjacent work.

Only after the selected review follow-ups, consider measurement-only steps for
remaining Fantasy invalidation/fact-loading cost and empty-queue polling. No
speculative optimization or load claim is authorized.

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
