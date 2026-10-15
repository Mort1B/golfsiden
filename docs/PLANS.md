# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. A1n acceptance is in the
[course-read report](validation/course-read-session-2026-10-15/README.md).

## Next candidate — requires approval

### A1o — assessment only: legacy team/round write-session inventory

The private-read inventory assessed GETs, not legacy writes. Selecting this
candidate authorizes a documentation-only source assessment, not remediation.
No severity is assigned before evidence establishes reachability and impact.

- Goal: identify which retained team/round mutation entry points are reachable,
  where session/admin authority is retained, and whether a final wall-clock
  session check covers their dependent work before mutation/commit.
- Scope: legacy granular team create/assign/remove and retained round mutation
  routes, their preflight/repository/database guard ownership, existing tests
  and a bounded route/caller manifest. Exclude the already assessed score,
  Fantasy, invitation and profile families, frontend behavior, runtime fixes,
  dependency/schema changes, performance work and hosted administration.
- Behavior: pin the source revision and trace HTTP methods through their actual
  callers. Separate reachable routes, retired/unreachable functions and trusted
  internal helpers. Classify authority as bound/incomplete/unresolved, rating
  concrete findings by severity and evidence strength. Do not infer an exploit
  from an unused helper or equate a final check with freezing time through
  commit/network delivery. Preserve all existing runtime behavior.
- Validation: reconcile route and repository references, source anchors, current
  contracts and existing test coverage; check documentation links/diff and obtain
  independent read-only review. No database/provider/browser/hosted access is
  needed; record that no new runtime race was reproduced.
- Invariants: no data, source, contract, scoring, handicap, locking or session
  behavior changes; report only supported findings and explicit uncertainty.
- Stop: publish the inventory and one evidence-based bounded next candidate.
  Do not implement a fix or begin deferred driver/test/UI/optimization work.

## Later queue

### Severity-ordered review follow-ups

All findings and individual acceptance criteria are in the
[review report](validation/application-review-2026-10-14/README.md#findings-and-acceptance-criteria).
Select one bounded step at a time; none is implemented by this plan.

The selected private-read findings now have linked acceptance. The next candidate
is only a legacy team/round write-session source inventory; those writes were
not covered by the private-read inventory. No remediation is selected by that
assessment plan.

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
