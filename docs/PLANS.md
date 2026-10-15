# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. A1q pairing-replacement expiry is complete; its reproduction, rollback,
browser and validation evidence is in the
[acceptance report](validation/pairing-write-session-2026-10-15/README.md).

## Next candidate — requires approval

### A1r — P2 course-configuration final session expiry

The [legacy-write inventory W3](validation/legacy-write-inventory-2026-10-15/README.md#w3--course-configuration-incomplete-p2)
supports this separate candidate. It is not started by A1q.

- Goal: reject course attachment if its initially authorized session expires
  during membership or course/revision/round-update work before commit.
- Scope: only `PUT /api/rounds/{round_id}/course-configuration`, its final
  `round_configuration::configure` transaction, focused PostgreSQL/API support
  and affected docs. Exclude preflight redesign, course GETs, provider/cache/quota
  behavior, retained helpers, frontend implementation, schema/dependencies,
  sporting rules and hosted work.
- Behavior: retain stored-round, session/user and exact-admin membership locks,
  then recheck the same session with database wall-clock time after the complete
  hierarchy insertion and round UPDATE/RETURNING, immediately before commit.
  Expiry uses existing private/no-store 401, rolls back course/tee/hole rows and
  attachment/timestamp changes, and emits no event. Preserve draft/version/format
  guards, immutable revision facts, validation precedence, successful 200 model
  and one post-commit event. Add no mutation retry or repeated provider fetch;
  keep external I/O outside the final transaction. COMMIT/deferred-constraint/
  delivery time remains outside the final-check guarantee.
- Validation: first reproduce final membership-wait and late pending hierarchy/
  attachment expiry against current source using deterministic PostgreSQL/API
  gates, distinct from preflight. Assert private 401, complete rollback without
  orphan revisions and no event, plus renewed-session controls creating one
  hierarchy/attachment/event. Run focused configuration/authorization tests,
  backend formatting/tests/strict Clippy, full PostgreSQL-feature/migrate/seed
  validation and independent read-only review. Check the existing configuration
  UI at mobile/desktop widths with relevant loading/error/empty/populated and
  long-content states. Preserve the unusable-catalog/provider coverage limit;
  do not claim a successful remote provider flow from helper fixtures. Check
  documentation, source size and scoped diff.
- Invariants: preserve stored-parent authority, draft-only attachment, historical
  course/handicap data, immutable complete revisions and atomicity.
- Stop: publish this single validated repair and a concise next candidate; do
  not begin preflight/provider, helper, driver, test-maintenance or optimization
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
acceptance. W3 (course configuration) is the sole remaining P2 write-expiry
candidate above and requires a separate approved step. The original assessment
itself reproduced no runtime race.
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
