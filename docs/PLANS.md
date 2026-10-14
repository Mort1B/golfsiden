# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. Awaiting approval of one bounded next step.

## Next candidate

### A3 — P2: coordinate identity-changing requests across route lifetimes

Evidence: [application review](validation/application-review-2026-10-14/README.md#a3--p2-late-account-creation-replaces-newer-authentication).

- Goal: prevent a departed anonymous creation or invitation registration from
  replacing a newer deliberate authentication choice.
- Scope: existing frontend authentication/request coordination, onboarding and
  invitation registration continuations, necessary login/logout/session-refresh
  integration, focused regressions and documentation. No new product feature,
  sporting rule, schema or deployment change.
- Behavior: coordinate cookie-changing HTTP operations across route lifetimes,
  fence late cache/navigation publication and reconcile authoritative cookie
  identity. Ignoring a React callback alone is insufficient. Preserve immutable
  mutation recovery and deliberate valid-session behavior.
- Validation: reverse-response-order and navigation races for creation and
  invitation registration, newer login/logout and same-user renewal. Assert
  actual cookie/canonical identity, cache ownership and navigation. Run focused
  tests, the affected full ladders, real mobile/desktop Chrome and read-only review.
- Invariants: retain private cache/session isolation, exact uncertain-write
  replay, acknowledged-write outcomes and existing API contracts.
- Stop: publish this authentication-ordering repair only; do not begin sibling
  private-read/write, dependency, recovery or capacity work.

## Later queue

### Severity-ordered review follow-ups

All findings and individual acceptance criteria are in the
[review report](validation/application-review-2026-10-14/README.md#findings-and-acceptance-criteria).
Select one bounded step at a time; none is implemented by this plan.

1. **P2 / A1 follow-up:** inventory and repair the remaining user-only private
   tournament/round/roster/pairing/team read families in bounded slices; inventory
   analogous expiry gaps in legacy team/round writes as a separate slice.
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
