# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. A1o assessment and validation are in the
[legacy-write report](validation/legacy-write-inventory-2026-10-15/README.md).

## Next candidate — requires approval

### A1p — P2 direct round-creation final session expiry

The [legacy-write inventory W1](validation/legacy-write-inventory-2026-10-15/README.md#w1--direct-round-creation-incomplete-p2)
supports this candidate. It is not implemented or started by the assessment.

- Goal: reject direct round creation when its initially authorized session expires
  during membership or later INSERT work before the final commit call.
- Scope: only `POST /api/tournaments/{tournament_id}/rounds`,
  `rounds::create_authorized`, focused PostgreSQL/API regression support and
  affected documentation. Exclude pairing/course writers, preflight redesign,
  retained helper cleanup, UI, dependencies/schema, provider and hosted work.
- Behavior: retain existing session/user and exact-admin membership locks, then
  recheck the same session with database wall-clock time after INSERT/RETURNING
  and immediately before commit. Expiry returns the existing unauthenticated
  error, rolls back the new round and conditional child rows, and emits no event.
  Preserve CSRF, preflight/body/semantic validation order, parent/domain guards,
  successful response and one-event behavior. Add no mutation retry; the check
  does not freeze time inside COMMIT/deferred constraints or network delivery.
- Validation: first reproduce membership-wait and late INSERT expiry against the
  current implementation using deterministic PostgreSQL/API gates. Assert 401,
  unchanged persisted state and no event, plus valid-session controls creating
  one round/event. Exercise pending child-row rollback where applicable. Run
  focused authorization regressions, backend formatting/tests/strict Clippy,
  the full PostgreSQL-feature/migrate/seed ladder, independent read-only
  authorization review and documentation/diff checks. The caller inventory finds
  no production frontend consumer of this direct POST; UI creation uses excluded
  aggregate APIs. No frontend/browser ladder is selected for this API-only step.
- Invariants: preserve round ownership/configuration, parent lifecycle, roster,
  historical handicaps, locked-round protections and atomic child effects.
- Stop: publish this single validated repair and a concise next candidate; do
  not begin W2/W3, provider-preflight, helper, driver, test-maintenance or
  performance work.

## Later queue

### Severity-ordered review follow-ups

All findings and individual acceptance criteria are in the
[review report](validation/application-review-2026-10-14/README.md#findings-and-acceptance-criteria).
Select one bounded step at a time; none is implemented by this plan.

The selected private-read findings have linked acceptance. The separate
[legacy-write inventory](validation/legacy-write-inventory-2026-10-15/README.md)
identifies three source-supported P2 expiry gaps. W1 is the sole next candidate
above. W2 (pairing replacement) and W3 (course configuration) need subsequent
separate bounded repairs; no runtime race was reproduced by the assessment.
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
