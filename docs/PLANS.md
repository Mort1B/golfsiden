# Plans

`PLANS.md` owns current and queued work. Current behavior belongs in
`Documentation.md`; deployment and recovery belong in `deployment_guide.md`.

## Active step

None. The previous bounded step is published with its
[production integration evidence](validation/production-rehearsal-2026-10-15/README.md).

## Next candidate

### DEPLOY-4 — critical organizer/player acceptance

- Goal: establish that Sunday's required existing journeys work together in the
  production assembly, using distinct organizer/player accounts.
- Scope: isolated synthetic fixtures and real browser/API evidence for login/
  invitation, roster/round setup, score save/recovery, gross/net, lock/correction,
  final visibility and the required existing Fantasy flow. Reuse unchanged
  artifact and focused acceptance evidence. Repair only confirmed serious defects
  or unusable required flows; no new features, schema redesign or broad polish.
- Behavior/invariants: retain session/membership fencing, exact uncertain-write
  replay, acknowledged-write reconciliation, owner-exclusive scores, historical
  handicaps, locked-round audit paths and final-round privacy. Fantasy remains
  separate from sporting scores/completion and keeps versioned rules/receipts.
- Validation: actual production settings/proxy and real restricted grants;
  distinct-account successful and denied reads/writes, authoritative saved state,
  relevant failure/retry states, mobile 320/390 and desktop widths. Run the full
  affected ladder for any necessary source repair; retain exact engine/physical-
  device limits. No combinatorial format/device matrix is required this step.
- Stop: publish critical-flow acceptance or exact remaining blockers, remove owned
  fixtures and define DEPLOY-5 recovery. Do not start recovery or administer the
  operator's host in this iteration.

## Sunday readiness path

Iterations 1–3 are complete. Work one bounded step per iteration; each next candidate
is defined before implementation. The six-iteration target is:

| Iteration | Required outcome |
| --- | --- |
| 4 | Critical organizer/player acceptance with distinct accounts: login/invitation, roster/round setup, score save/recovery, gross/net, lock/correction, final visibility and required Fantasy flow, at mobile/desktop widths |
| 5 | Actual current-application backup/checksum/restore into a fresh isolated project, state parity, real grants, readiness/login and restart/reconnect; short operating handoff |
| 6 | Release freeze: affected final gates and current dependency/image advisory triage, immutable tested candidate, READY / READY WITH KNOWN LIMITATIONS / NOT READY verdict, concise operator launch/rollback checklist |

Iterations 7–8 are contingency for confirmed correctness, security, essential-flow
or recovery blockers. The target does not turn an unresolved serious defect into
acceptance. Existing evidence is reused when relevant source/configuration has
not changed; skipped checks and limits remain explicit.

### Launch triage

- Block sign-off for P0–P2 security, correctness or data-loss defects;
  unusable required flows; failed image/startup/schema/grants; or failed recovery.
- Classify test scheduling failures separately from application defects. A
  bounded non-reproduction is a limitation, not proof of a repair or full default
  success. Another test-maintenance iteration needs actionable evidence or an
  essential release-validation blocker; it is not queued automatically.
- Defer new features, speculative optimization, unmeasured empty-queue polling,
  broad polish and the expanded device matrix. Do not reopen repaired findings
  without new evidence. Keep existing sporting/session/replay/privacy invariants.

### Operator prerequisites — prepare before the final iteration

Hosting remains operator-owned. Confirm new install versus upgrade, the chosen
host/domain/DNS, reachable ports, private secrets, persistent storage and an
off-host backup destination. Use the existing
[deployment guide](deployment_guide.md); no hosted action is authorized here.
Public HTTPS/readiness, the actual playing-phone workflow and off-host backup
acceptance must be checked on the chosen environment. Local Podman or browser
acceptance is not public Docker Engine/DNS/ACME acceptance.

## Deferred work and retained limits

- The [application review](validation/application-review-2026-10-14/README.md)
  links the selected finding repairs. The separate
  [legacy-write inventory](validation/legacy-write-inventory-2026-10-15/README.md)
  retains W1/W2/W3 acceptance. The
  [DEPLOY-2 report](validation/production-images-2026-10-15/README.md)
  retains the original image acceptance. The
  [DEPLOY-3 report](validation/production-rehearsal-2026-10-15/README.md) records
  integrated acceptance and the repaired P2 proxy-secret logging leak; recovery,
  native Docker and hosted/final release acceptance remain open.
- [A1z](validation/lifecycle-expiry-investigation-2026-10-15/README.md) retains the
  unexplained full-default lifecycle test failure and passing focused controls.
  Revisit only with actionable evidence or a required release-validation blocker.
- Empty-queue polling remains unmeasured. A1t did not select Fantasy refresh
  deduplication. Neither is a prerequisite for Sunday's release without new
  measured user impact.
- Cold offline launch, browser-closure preparation and background sync remain
  outside the existing scorecard-return/durable-edit contract.
- Full physical iPhone 15 Pro/newer-large-model, orientation/keyboard/zoom and
  Android coverage remains unverified; retain the
  [device checklist](validation/iphone-layout-2026-10-14/README.md#outstanding-physical-device-acceptance).
  This is not replaced by local Chrome/WebKit or marked complete by deferral.
- PERSIST-1 same-account/actual-login scenarios retain their
  [specific limits](validation/match-note-retention-2026-09-23/README.md). Revisit
  only if required-flow feedback supplies evidence.

No new sporting rules, automatic opponents/byes/brackets, team match play, extra
holes, public match sharing or provider features are selected. Fantasy remains
separate from sporting scores and completion rules. No hosted security/capacity
claim is made from local acceptance.
