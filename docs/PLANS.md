# Plans

`PLANS.md` owns current and queued work. Current behavior belongs in
`Documentation.md`; deployment and recovery belong in `deployment_guide.md`.

## Active step

None. DEPLOY-6 completes the six planned repository readiness iterations.
The [frozen release](validation/release-freeze-2026-10-16/README.md) is
**READY WITH KNOWN LIMITATIONS** for local acceptance. The
[operator handoff](validation/release-freeze-2026-10-16/OPERATOR.md) owns remaining
native-Docker/public-host, actual-phone and off-host recovery launch checks.
No hosted deployment was performed or authorized here.

## Next candidate

None selected. Iterations 7–8 remain contingency only for a newly confirmed
P0–P2 correctness, security, data-loss, essential-flow or recovery blocker.
Do not open another maintenance iteration or optional feature automatically.

## Sunday readiness policy

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

### Operator prerequisites — complete before launch

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
  physical phones and off-host recovery remain operator gates. Local final
  acceptance and the exact frozen delivery are recorded in DEPLOY-6.
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
