# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. A4 acceptance is in the [Rustls update report](validation/rustls-update-2026-10-15/README.md).

## Next candidate — requires approval

### A5 — P2: require a checksum bound to the restore archive

Evidence: [original review A5](validation/application-review-2026-10-14/README.md#a5--p2-checksum-requirement-is-optional-in-the-restore-script).
The current script verifies a sidecar only when present, using its embedded
filenames rather than independently binding the digest to the selected archive.

- Goal: refuse restoration unless the required sidecar validates the exact
  selected nonempty archive before any Docker/restore action.
- Scope: `scripts/restore-production.sh`, its focused local fake-Docker harness,
  evidence and current restore/operator docs. Adjust backup compatibility only
  if necessary to preserve its existing one-archive SHA-256 sidecar contract.
  Exclude hosted restore, production data, unrelated deployment changes and
  other findings.
- Behavior: require a readable, valid checksum sidecar; bind its single checksum
  record to the requested archive and verify that archive's bytes. Reject absent,
  malformed, mismatched, foreign-file or extra-record sidecars before Docker is
  invoked. Preserve valid generated sidecars, quoted paths, explicit confirmation,
  nonempty input, pristine-target guard, transactional restore and permissions
  recovery. A checksum remains transfer-integrity evidence, not authenticity.
- Validation: controlled fake-Docker cases for valid, missing, malformed,
  tampered, foreign-file and extra-record inputs, including spaced paths;
  confirmation, empty input and nonempty-target guards; exact restore flags,
  service-stop ordering and permission recovery. Run shell syntax checks and
  independent read-only review; use only synthetic files/local harnesses.
- Invariants: preserve root sporting/handicap/locked-round rules by performing no
  application mutation; retain empty-target and single-transaction recovery
  protections. No actual hosted restore or server administration.
- Stop: publish only checksum binding, tests/evidence and docs. Begin no queued
  password/dependency/performance/driver work.

## Later queue

### Severity-ordered review follow-ups

All findings and individual acceptance criteria are in the
[review report](validation/application-review-2026-10-14/README.md#findings-and-acceptance-criteria).
Select one bounded step at a time; none is implemented by this plan.

1. **P2 / A6:** cap password-work admission while retaining four active Argon2
   workers and cancellation-safe capacity release. Capacity impact is unmeasured.
2. **P2 / A7:** upgrade affected development tools in compatible, validated
   dependency steps. Scanner critical labels do not establish production RCE.
3. **P3 / A8:** batch team memberships, preserving ordering and authority; measure
   query count and end-to-end cost. No current frontend latency impact is proven.

4. **P3 / A1 course reads:** bind the three lower-impact course GETs in a
   separately selected step; provider-success impact remains conditional in the
   [source inventory](validation/private-read-inventory-2026-10-14/README.md#courses).

Keep legacy team/round write-expiry inventory separate; the private-read inventory
never assessed those writes. Selecting the checksum candidate does not complete it or authorize the later queue.

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
