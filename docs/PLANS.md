# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. The [A1w acceptance](validation/sqlx-verify-ca-2026-10-15/README.md)
records the P3 SQLx compatibility repair, 25-case TLS before/after controls,
preserved cancellation override and passing Rust/PostgreSQL/release gates.

## Next candidate — requires approval

### A1x — repair retained leaderboard browser assertions

This selects the existing
[obsolete score-status assertions](validation/leaderboard-session-2026-10-14/README.md#browser-evidence-and-remaining-test-maintenance)
before using that suite as a release gate. It is test maintenance, not a confirmed
application synchronization defect.

- Goal: require the actual acknowledged score state so the retained browser
  suite can exercise its later reconnection and hidden-final scenarios.
- Scope: the two obsolete assertions in
  `frontend/e2e/leaderboardLive.browser.ts`, narrowly needed test support and
  evidence/docs. No application UI/API/SQL/schema/dependency change.
- Behavior/invariants: retain explicit server-acknowledgement checks and the
  existing live-update, crash/restart, privacy and final-visibility expectations.
  Do not weaken them to accept pending, failed or merely local saves.
- Validation: reproduce the old label assertion on a fresh owned local fixture;
  pass the repaired complete retained suite in a real browser at mobile/desktop
  widths and the affected frontend ladder. Record exact later-scenario outcomes,
  console/network failures and independent read-only scope review.
- Stop: publish only the test-maintenance repair and evidence. Record any newly
  exposed product defect separately; do not fix it or other queued work here.

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
A1r browser checks separately identified the P3 mobile manual-hole labeling
issue; A1s now closes it with visible headings and frontend/browser acceptance.
Fantasy A1u now has scoped-fact performance/parity acceptance without a new
severity rating. A1v closes the retained P3 test-portability limitation with
two-image acceptance. A1w closes the separate P3 driver compatibility limitation
with retained strict/certificate controls. The next browser-test candidate is
unstarted.
Retired granular team writes receive no reachable-route finding. Provisional
user-only preflight/provider-work observations remain separate and unrated.

Empty-queue polling remains a separate unmeasured candidate. Fantasy refresh
deduplication is not selected by A1t's overlapping completion/cancellation
evidence. No speculative optimization or load claim is authorized.

### Previously deferred work

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
