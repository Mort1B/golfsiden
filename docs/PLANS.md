# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. A1l acceptance is in the [invitation metadata report](validation/invitation-read-session-2026-10-15/README.md).

## Next candidate — requires approval

### A1m — P2: bind final-visibility metadata GET to its session

Evidence: [remaining private-read inventory](validation/private-read-inventory-2026-10-14/README.md#visibility),
R2 (source-supported; post-extractor authority loss needs its own reproduction).

- Goal: prevent final-round-visibility metadata reads from succeeding with an
  expired, revoked or credential-replaced session after extraction.
- Scope: `GET /api/tournaments/{tournament}/final-round-visibility`; only its
  API/repository read boundary, focused evidence and docs. Exclude visibility
  PATCH behavior, score/result projections, other reads/mutations, schema,
  frontend source, dependencies and other findings.
- Behavior: pass session identity into a session-bound read owner; retain active
  session/user and exact tournament-admin membership through the complete
  metadata read, then recheck database wall-clock session validity before success.
  Preserve stored tournament lookup and error precedence, repeatable-read
  coherence, the three-field DTO and version precision. Allow one fresh complete
  read after `40001`, then generic private 503; reuse no discarded authority,
  context or metadata. Apply `private, no-store` to all GET statuses, including
  extraction failures, while preserving PATCH/helper contracts and headers.
- Validation: reproduce lost authority during a controlled real database wait
  first; cover natural expiry, post-extractor revocation/credential changes,
  membership loss/demotion and fresh visibility on retry, exact roles and
  cross-tournament denial, missing/invalid identities, full JSON/version parity,
  retry limits/nonserialization errors, cancellation/concurrent reads and no
  writes/SSE. Run full affected backend/PostgreSQL ladders, mobile/desktop browser
  checks and independent read-only review.
- Invariants: preserve root sporting/handicap/locked-round rules, exact-admin
  scope, hidden-final policy and auditable versioned PATCH behavior. No new
  visibility policy, publication, mutation retry or side effects.
- Stop: publish only this GET repair and evidence; begin no queued family.

## Later queue

### Severity-ordered review follow-ups

All findings and individual acceptance criteria are in the
[review report](validation/application-review-2026-10-14/README.md#findings-and-acceptance-criteria).
Select one bounded step at a time; none is implemented by this plan.

1. **P2 / A4:** update reachable Rustls to a compatible patched release, retain
   the SQLx cancellation patch and recheck TLS behavior/advisory reachability.
2. **P2 / A5:** require and bind the restore checksum before invoking restore;
   preserve empty-target and single-transaction protections.
3. **P2 / A6:** cap password-work admission while retaining four active Argon2
   workers and cancellation-safe capacity release. Capacity impact is unmeasured.
4. **P2 / A7:** upgrade affected development tools in compatible, validated
   dependency steps. Scanner critical labels do not establish production RCE.
5. **P3 / A8:** batch team memberships, preserving ordering and authority; measure
   query count and end-to-end cost. No current frontend latency impact is proven.

6. **P3 / A1 course reads:** bind the three lower-impact course GETs in a
   separately selected step; provider-success impact remains conditional in the
   [source inventory](validation/private-read-inventory-2026-10-14/README.md#courses).

Keep legacy team/round write-expiry inventory separate; the private-read inventory
never assessed those writes. Selecting the final-visibility candidate does not
complete it or authorize the later queue.

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
