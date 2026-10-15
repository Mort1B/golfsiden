# Plans

`PLANS.md` owns current and queued work. Current behavior belongs in
`Documentation.md`; deployment and recovery belong in `deployment_guide.md`.

## Active step

None. Current-application recovery acceptance is recorded in the
[DEPLOY-5 report](validation/recovery-2026-10-15/README.md).

## Next candidate — requires implementation approval

### DEPLOY-6 — release freeze and operator handoff

- Goal: provide an evidence-backed release verdict and concrete launch/rollback
  checklist for Sunday's one-host Compose target.
- Scope: freeze source and exact tested artifacts, triage current dependency/
  image advisories and review the remaining essential launch/recovery gates.
  Reuse unchanged accepted builds, integration, critical flows and recovery.
  Repair only confirmed P0–P2 or unusable required-flow blockers; no features,
  broad polish, speculative optimization or automatic reopening of P3 limits.
- Behavior/invariants: keep schema-39 matching builds, private proxy logs,
  restricted runtime grants, session/membership fencing, exact write replay,
  score/handicap/lifecycle and final-round privacy contracts. Preserve immutable
  artifact identities; any necessary repair invalidates its affected evidence.
- Validation: current primary advisory evidence and compiled/runtime reachability,
  exact source/artifact comparison, all affected gates for a selected blocking
  repair, scoped read-only review and documentation/link/cleanup/publication
  checks. Distinguish local engine/browser acceptance from unperformed operator
  host, physical-phone and off-host checks; never claim them complete by deferral.
- Stop: publish READY / READY WITH KNOWN LIMITATIONS / NOT READY with severity-
  rated remaining issues, immutable candidate details and concise operator
  launch/rollback actions. Clean owned fixtures, publish and stop. No deployed
  host administration, deployment or irreversible release action is authorized.

## Sunday readiness path

Iterations 1–5 are complete. Work one bounded step per iteration; each next candidate
is defined before implementation. The six-iteration target is:

| Iteration | Required outcome |
| --- | --- |
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
  integrated acceptance and the repaired P2 proxy-secret logging leak. The
  [DEPLOY-4 report](validation/critical-flows-2026-10-15/README.md) records the
  complete critical-flow sequence and P3 acknowledged Fantasy entry/read-retry
  limitation. The [DEPLOY-5 report](validation/recovery-2026-10-15/README.md)
  records actual current-script/schema-39 restore, full table/sequence parity,
  restored logins/replay and restart/reconnect. Native Docker, public host,
  physical phones, off-host recovery and final release acceptance remain open.
  No further Fantasy diagnosis is selected without new serious or unusable
  required-flow evidence.
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
