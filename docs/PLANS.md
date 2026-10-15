# Plans

`PLANS.md` owns current and queued work. Current behavior belongs in
`Documentation.md`; deployment and recovery belong in `deployment_guide.md`.

## Active step

None. [DEPLOY-2](validation/production-images-2026-10-15/README.md) records three
successful actual Dockerfile builds, 11 Compose and 23 packaged-artifact/guard
probes on rootless Podman/amd64. Production source/configuration is unchanged.
Native Docker socket access is denied; no host privileges or daemons were changed.

## Next candidate — requires approval

### DEPLOY-3 — isolated production integration rehearsal

Sunday 18 October targets the confirmed single-host Docker Compose model. This
is iteration 3 of six; iterations 7–8 remain contingency for confirmed blockers.

- Goal: verify the built matching artifacts together under production settings,
  including real database authority and browser/proxy behavior.
- Scope: one owned isolated Compose project, private synthetic configuration,
  database/network/volumes and temporary browser trust/profile, with narrowly
  necessary integration repairs and evidence/docs. Reuse DEPLOY-2 image IDs while
  build inputs remain unchanged. No hosted deployment, feature or schema work.
- Behavior/invariants: owner-only migration, real restricted runtime grants,
  exact schema 39 compatibility, no production seed, secure cookies, trusted
  proxy headers, private PostgreSQL and preserved session/scoring rules. Bind
  rehearsal web ports only to loopback. Never weaken production guards.
- Validation: initialize/migrate/grant and start the actual production API/web;
  verify health/readiness, runtime authority and schema failure controls,
  organizer/player login with secure cookies and actual SSE through Caddy.
  Exercise mobile/desktop browser states appropriate to this integration scope.
  Use native Docker when available; an owned temporary rootless Podman API socket
  may support Compose-compatible rehearsal, with exact engine/compatibility limits
  recorded. Do not alter host socket permissions, groups or daemons to proceed.
- Stop: publish integrated evidence or exact tooling blockers, remove only owned
  rehearsal services/data/trust/credentials, and define the critical-flow step.
  Do not start that next iteration or administer the user's host.

## Sunday readiness path

Iterations 1–2 are complete. Work one bounded step per iteration; each next candidate
is defined before implementation. The six-iteration target is:

| Iteration | Required outcome |
| --- | --- |
| 3 | Local production rehearsal: matching API/web, real schema 39/grants, production startup, health/readiness, secure-cookie browser access and SSE through Caddy |
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
  separates current image acceptance from open integration/recovery and native
  Docker/hosted acceptance. No new P0–P2 product bug is established.
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
