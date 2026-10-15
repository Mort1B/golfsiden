# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. A5 acceptance is in the [restore-checksum report](validation/restore-checksum-2026-10-15/README.md).

## Next candidate — requires approval

### A6 — P2: bound password-work admission

Evidence: [original review A6](validation/application-review-2026-10-14/README.md#a6--p2-password-work-admission-has-no-explicit-queue-bound).
The current helper limits active workers to four but lets every caller await
that semaphore while retaining its operation and inputs.

- Goal: bound process-local active-plus-waiting password work without weakening
  Argon2, credential checks or cancellation safety.
- Scope: the shared password blocking-work boundary, focused unit/API/PostgreSQL
  tests and docs. Preserve existing callers' API contracts and error mappings;
  exclude authentication-policy changes, rate-limit redesign, frontend work,
  dependencies, hosted configuration and other findings.
- Behavior: admit at most eight password operations in total, retaining the
  existing maximum of four active blocking workers and at most four waiting
  operations. Reject excess admission promptly using the existing capacity error
  and generic non-secret HTTP error mapping. Share the same budget across
  hashing, real-user verification and dummy-user verification. Waiting-request
  cancellation releases admission; already-started blocking work holds all its
  capacity until completion even if its request is cancelled. Errors/panics must
  release capacity. Preserve credentials, hashes, generic login failures and
  account/session write ordering. This finite policy is not a measured service-
  capacity or latency claim.
- Validation: deterministic saturation gates prove the four-active/eight-admitted
  bounds, prompt excess rejection, waiting cancellation, active cancellation,
  error/panic release and later legitimate recovery. Cover known/dummy-user
  parity, unchanged generic HTTP errors and no account/session write on failure;
  run the complete affected backend/PostgreSQL ladders and independent read-only
  review. Record evidence without passwords, encoded hashes or session secrets.
- Invariants: preserve root sporting/handicap/locked-round rules, constant-time
  credential verification intent, existing rate limits and no unauthenticated
  account/session mutation. Capacity belongs to work until it actually ends.
- Stop: publish only bounded password admission and evidence. Begin no queued
  dependency, query-performance, course-read or driver work.

## Later queue

### Severity-ordered review follow-ups

All findings and individual acceptance criteria are in the
[review report](validation/application-review-2026-10-14/README.md#findings-and-acceptance-criteria).
Select one bounded step at a time; none is implemented by this plan.

1. **P2 / A7:** upgrade affected development tools in compatible, validated
   dependency steps. Scanner critical labels do not establish production RCE.
2. **P3 / A8:** batch team memberships, preserving ordering and authority; measure
   query count and end-to-end cost. No current frontend latency impact is proven.

3. **P3 / A1 course reads:** bind the three lower-impact course GETs in a
   separately selected step; provider-success impact remains conditional in the
   [source inventory](validation/private-read-inventory-2026-10-14/README.md#courses).

Keep legacy team/round write-expiry inventory separate; the private-read inventory
never assessed those writes. Selecting the password-admission candidate does not complete it or authorize the later queue.

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
