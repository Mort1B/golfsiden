# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. The [A1y acceptance](validation/leaderboard-expiry-timing-2026-10-15/README.md)
records the bounded P3 test setup repair, default-concurrency target acceptance,
full two-thread ladder and separate unresolved lifecycle failure. The historical
leaderboard timeout's exact cause remains unproven.

## Next candidate — requires approval

### A1z — investigate separate match-lifecycle late-expiry failure

The A1y full default-concurrency ladder passed its selected leaderboard case but
stopped at the unchanged
`lifecycle_expiry::scenarios::match_late_expiry_rolls_back_all_actions`.
The retained receipt identifies the test/counts, not its panic location or cause;
the case passes the full two-thread control. No product or fixture cause, or new
severity, is established.

- Goal: capture and explain that separate failure before selecting a repair.
- Scope: that case and narrowly needed diagnostic/setup support plus evidence
  and docs. No production authorization, lifecycle, transaction, retry, scoring
  or timeout-policy changes; no unrelated test repair.
- Behavior/invariants: preserve the intended late-write gate, actual preceding
  work, natural expiry, every denial/rollback assertion and valid-session
  control. Early rejection must not substitute for the intended late-expiry race.
- Validation: retain the actual failure details and database-clock/gate/response
  ordering on owned PostgreSQL under relevant default concurrency with a
  separately identified two-thread control. Repair only an evidenced fixture
  defect, then run affected ladders and independent review; otherwise record
  non-reproduction limits. A product defect requires a separate bounded step.
- Stop: publish this one investigation/justified fixture repair and its limits;
  do not begin optimization or another lifecycle case.

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
without a product change. A1y has linked test-timing acceptance; its separate
lifecycle failure is the next candidate.
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
