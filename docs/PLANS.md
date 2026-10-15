# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. A7b acceptance is in the
[Vitest major report](validation/vitest-major-2026-10-15/README.md).

## Next candidate — requires approval

### A8 — P3: batch team-member loading without changing read authority

Evidence: [original review A8](validation/application-review-2026-10-14/README.md#a8--p3-team-member-queries-grow-with-team-count).
The current shared team-list helper reads teams once, then fetches members once
per team. This is source-confirmed 1+N data SELECTs; frontend latency impact
remains unmeasured.

- Goal: bound team/member data loading to one SELECT for an empty team list and
  two SELECTs for populated lists, independent of team count, while preserving
  complete response and authorization behavior.
- Scope: the shared team-list repository helper and focused PostgreSQL/API tests,
  a disposable local query-count/cost comparison and docs. Exclude team/round
  mutations, schema/index changes, API/DTO/frontend changes, pagination, course
  reads, generic repository refactors and hosted performance work.
- Behavior: retain the existing ordered team query; load all selected team IDs'
  members in one parameter-bound query and group them without changing team or
  member order, empty-team inclusion or fields. Preserve native SQL collation,
  `display_order NULLS LAST`, display-name and player-UUID tie-breaks, and team
  name/UUID ordering. Keep the same shared transaction/snapshot, session retry,
  round membership authorization and final active-session gate even for empty
  results. Preserve ordinary internal-list behavior and private/no-store HTTP
  mapping. Do not parallelize SQL on the transaction or weaken locking/fences.
- Validation: baseline regression proves per-team query growth; repaired counts
  prove one/two data SELECTs for 0/1/12/100 synthetic teams, separately identifying
  authorization/transaction queries. Compare complete payloads, empty teams,
  null/duplicate member positions, name/UUID ties and round/tournament isolation.
  Exercise revocation/expiry/cancellation and concurrent membership changes using
  the existing pairing/team session suite. Run the complete backend/PostgreSQL
  ladders, migrate/seed and independent read-only review. Measure local complete
  handler/read cost before/after under matching data/session conditions; record
  limits without claiming frontend or hosted speedup from query counts alone.
- Invariants: tournament players remain independent of teams; team/round scope,
  one-team-per-round constraints, preserved handicap/score rules and read authority
  stay unchanged. No data write, SSE event, payload or cache-policy change.
- Stop: publish only bounded team-member batching and evidence. Begin no course-
  read, driver, collation/test-maintenance, UI or other performance work.

## Later queue

### Severity-ordered review follow-ups

All findings and individual acceptance criteria are in the
[review report](validation/application-review-2026-10-14/README.md#findings-and-acceptance-criteria).
Select one bounded step at a time; none is implemented by this plan.

1. **P3 / A1 course reads:** bind the three lower-impact course GETs in a
   separately selected step; provider-success impact remains conditional in the
   [source inventory](validation/private-read-inventory-2026-10-14/README.md#courses).

Keep legacy team/round write-expiry inventory separate; the private-read inventory
never assessed those writes. Selecting team-member batching does not authorize the later queue.

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
