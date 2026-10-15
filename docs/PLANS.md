# Plans

`PLANS.md` owns current and queued work. Current behavior belongs in
`Documentation.md`; deployment and recovery belong in `deployment_guide.md`.

## Active step

None. DEPLOY-4 acceptance is recorded in the
[critical-flow report](validation/critical-flows-2026-10-15/README.md).

## Next candidate — requires implementation approval

### DEPLOY-5 — current-application recovery rehearsal

- Goal: prove that the current schema-39 application can recover its acknowledged
  state using the supplied single-host backup/restore procedure.
- Scope: synthetic source data on the tested production images, actual backup
  script and checksum sidecar, restoration into a fresh isolated project/volume,
  owner/runtime grants, state parity and a short operating handoff. Reuse
  unchanged build, integration and critical-flow evidence. Repair only confirmed
  recovery or essential-flow blockers; no feature or schema redesign.
- Behavior/invariants: never target an operator database or reuse a populated
  restore destination. Preserve exact score revisions, confirmations, lifecycle,
  historical handicaps, Fantasy receipts and private session/membership facts.
  Restored runtime remains restricted; unsent browser drafts are outside server
  backups. Never expose archive content, credentials or capability/session data.
- Validation: actual production backup/checksum/restore commands, a fresh target
  volume, representative complete state parity, real runtime grants, schema and
  readiness, existing-account UI login/read, owned service restart/reconnect and
  exact owned cleanup. Run the affected full ladder for any necessary repair;
  keep native Docker, public host and off-host limits explicit.
- Stop: publish recovery evidence or exact blockers, clean owned fixtures and
  define DEPLOY-6 freeze/advisory/launch handoff. Do not start final sign-off or
  administer the operator's host in this iteration.

## Sunday readiness path

Iterations 1–4 are complete. Work one bounded step per iteration; each next candidate
is defined before implementation. The six-iteration target is:

| Iteration | Required outcome |
| --- | --- |
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
  integrated acceptance and the repaired P2 proxy-secret logging leak. The
  [DEPLOY-4 report](validation/critical-flows-2026-10-15/README.md) records the
  complete critical-flow sequence and P3 acknowledged Fantasy entry/read-retry
  limitation. Recovery, native Docker and hosted/final release acceptance remain
  open. No further Fantasy diagnosis is selected without new serious or unusable
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
