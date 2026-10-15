# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. A1r course-configuration expiry is complete; reproduction, rollback,
browser coverage and validation limits are in the
[acceptance report](validation/course-write-session-2026-10-15/README.md).

## Next candidate — requires approval

### A1s — P3 visible mobile manual-hole labels

A1r's [browser evidence](validation/course-write-session-2026-10-15/README.md#separate-p3-mobile-manual-hole-labels)
confirms this separate pre-existing usability issue. It is not started by A1r.

- Goal: let sighted administrators identify Par, Slagindeks and Avstand fields
  while manually configuring holes at mobile widths.
- Scope: only manual-hole label presentation in `ManualCourseForm.tsx` and
  `course-configuration.css` as needed, relevant frontend/browser acceptance and
  affected documentation. Exclude course persistence, authorization, provider,
  mutation state/retry behavior, sporting rules, schema/dependencies and hosted
  work.
- Behavior: retain compact visible column or row labels below 700px, preserving
  accessible input names and hole identity. Keep existing values, validation,
  fixed/variable hole-count behavior, optional distance units, save controls,
  desktop layout and error feedback. Avoid horizontal overflow at 320px.
- Validation: compare against captured mobile/desktop evidence; run the affected
  frontend test/type/lint/build ladder and real-browser checks at 320, 390 and
  1280px. Cover empty/filled rows, validation errors, long course/tee text,
  18-hole fixed and variable layouts, saving/denial feedback and persisted real
  save; inspect screenshots, keyboard focus, control reachability, console and
  network. Use read-only review, documentation/link, file-size and scope checks.
- Invariants: do not change course facts, numeric parsers, stroke-index
  uniqueness, format guards, session isolation or explicit save semantics.
- Stop: publish this single validated presentation repair and a concise next
  candidate; do not begin adjacent form-state, provider, driver or performance
  work.

## Later queue

### Severity-ordered review follow-ups

All findings and individual acceptance criteria are in the
[review report](validation/application-review-2026-10-14/README.md#findings-and-acceptance-criteria).
Select one bounded step at a time; none is implemented by this plan.

The selected private-read findings have linked acceptance. The separate
[legacy-write inventory](validation/legacy-write-inventory-2026-10-15/README.md)
preserves the original three source-supported P2 expiry gaps. The linked
round-creation repair closes W1 with reproduced expiry and rollback evidence.
The linked pairing-replacement repair closes W2 with the same expiry/rollback
acceptance. The linked course-configuration repair closes W3 with complete
hierarchy rollback/control coverage. All three selected P2 write gaps now have
linked acceptance; the original assessment itself reproduced no runtime race.
A1r browser checks separately identify the P3 mobile manual-hole labels candidate
above. No frontend implementation began in A1r.
Retired granular team writes receive no reachable-route finding. Provisional
user-only preflight/provider-work observations remain separate and unrated.

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
