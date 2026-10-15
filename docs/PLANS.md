# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. The [A1x acceptance](validation/leaderboard-browser-maintenance-2026-10-15/README.md)
records the original stale-label failure, current failed-delivery test control,
complete browser sequence and passing frontend ladder. Application behavior is
unchanged.

## Next candidate — requires approval

### A1y — investigate retained leaderboard expiry-test timing

This selects the retained
[default-concurrency test timeout](validation/round-lifecycle-session-2026-10-14/README.md#validation).
The original cause remains unproven; the full two-thread ladders pass.

- Goal: determine why `expiry_during_score_loading_denies_all_boards` missed its
  controlled loading wait under default test concurrency, and stabilize only an
  evidenced fixture/setup timing defect.
- Scope: that one test and narrowly needed setup/diagnostic support plus
  evidence/docs. No production authority, expiry, query, retry, concurrency or
  sporting behavior changes; no blanket timeout increase or unrelated test work.
- Behavior/invariants: the request must begin with valid authority and expire
  during the intended data-loading wait. Preserve every private 401/no-store,
  payload-denial and existing control assertion. Do not replace the expiry race
  with an unrelated early rejection or treat thread-limited success as diagnosis.
- Validation: reproduce or explicitly bound the retained default-concurrency
  failure on owned PostgreSQL; record gate/response/deadline ordering and exact
  scheduling. If a test repair is justified, pass focused repetitions and the
  affected Rust/PostgreSQL ladders, including the relevant default-concurrency
  run and two-thread control, with independent read-only review.
- Stop: publish the bounded test repair/evidence or documented non-reproduction
  limits. Record any product defect separately; do not implement it or other
  queued work here.

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
with retained strict/certificate controls. A1x restores retained browser acceptance
without a product change. The next expiry-test investigation is unstarted.
Retired granular team writes receive no reachable-route finding. Provisional
user-only preflight/provider-work observations remain separate and unrated.

Empty-queue polling remains a separate unmeasured candidate. Fantasy refresh
deduplication is not selected by A1t's overlapping completion/cancellation
evidence. No speculative optimization or load claim is authorized.

### Previously deferred work

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
