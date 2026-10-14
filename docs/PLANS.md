# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. Awaiting approval of one bounded next step.

## Next candidate

### A1g — P2: finish numeric score-read session authority

Evidence: [remaining private-read inventory](validation/private-read-inventory-2026-10-14/README.md#numeric),
R1 (source-supported; a new late-expiry race still needs reproduction).

- Goal: prevent successful numeric score-access/member-card/scoring-card replies
  after session expiry during authority or fact-loading waits.
- Scope: `GET /api/rounds/{id}/score-access`, `GET
  /api/rounds/{id}/scorecards/{owner_type}/{owner_id}` and that card GET's `/scoring`
  variant; their focused API/repository boundary, regression evidence and docs.
  Exclude mutations, conditional delivery, four-ball/Stableford/match siblings,
  completion reads, scoring formulas, schema, frontend changes and other findings.
- Behavior: retain session/user/exact-membership authority and one coherent full
  read, then check database wall-clock session validity after complete assembly
  before success. Include viewer/unlinked empty owners and the singles empty
  score-access branch. Preserve stored ownership, role/flight permissions, final
  visibility, fixed handicap snapshots, payloads, ordering and error precedence.
  Retain the existing whole-scoring-read `40001` retry; allow one fresh complete
  attempt for the other two selected GETs, then generic private 503. Reuse no
  discarded role/context/facts. Cover all selected GET statuses with non-storeable
  responses, without changing adjacent mutation contracts.
- Validation: first reproduce controlled late expiry; cover session/credential and
  membership changes, final loading/empty branches, exact roles and owner formats,
  hidden finals, retry freshness/exhaustion, cancellation, valid concurrent reads,
  full JSON parity, no writes/SSE and retained mutation/lifecycle compatibility.
  Run backend/PostgreSQL ladders using the repository's PostgreSQL 17 Alpine
  fixture, focused mobile/desktop browser evidence and independent read-only review.
- Invariants: preserve root scoring/handicap/locked-round rules and existing read
  versus scoring authority. No mutation retry or side effect; shared mutation
  helpers retain their established behavior.
- Stop: publish only these three GET repairs and evidence; start no queued family.

## Later queue

### Severity-ordered review follow-ups

All findings and individual acceptance criteria are in the
[review report](validation/application-review-2026-10-14/README.md#findings-and-acceptance-criteria).
Select one bounded step at a time; none is implemented by this plan.

1. **P2 / A1 after A1g:** select one bounded remaining repair from the
   [source inventory](validation/private-read-inventory-2026-10-14/README.md):
   match details/completion/table (4), four-ball cards (2), Stableford cards (2),
   invitation metadata (1), or user-only final-visibility metadata (1). Course
   GET binding (3) is P3 and conditional/lower-impact. Keep legacy team/round
   write-expiry inventory a separate slice; this read inventory did not assess it.
2. **P2 / A4:** update reachable Rustls to a compatible patched release, retain
   the SQLx cancellation patch and recheck TLS behavior/advisory reachability.
3. **P2 / A5:** require and bind the restore checksum before invoking restore;
   preserve empty-target and single-transaction protections.
4. **P2 / A6:** cap password-work admission while retaining four active Argon2
   workers and cancellation-safe capacity release. Capacity impact is unmeasured.
5. **P2 / A7:** upgrade affected development tools in compatible, validated
   dependency steps. Scanner critical labels do not establish production RCE.
6. **P3 / A8:** batch team memberships, preserving ordering and authority; measure
   query count and end-to-end cost. No current frontend latency impact is proven.

Only after these, consider measurement-only steps for remaining Fantasy
invalidation/fact-loading cost and empty-queue polling. No speculative optimization
or load claim is authorized.

### Previously deferred work

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
