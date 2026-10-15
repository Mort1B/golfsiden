# Plans

`PLANS.md` owns current and queued work. Current behavior belongs in
`Documentation.md`; deployment and recovery belong in `deployment_guide.md`.

## Active step

None. [A1z](validation/lifecycle-expiry-investigation-2026-10-15/README.md) records
bounded non-reproduction of the retained lifecycle failure and the Sunday
readiness inventory. No repair was justified and no runtime/test source changed.
The historical full default-concurrency failure remains an explicit limitation.

## Next candidate — requires approval

### DEPLOY-2 — production image and deployment contract baseline

Sunday 18 October 2026 targets one host using the supplied Docker Compose setup,
as confirmed by the user. This is iteration 2 of a six-iteration target, with
iterations 7–8 reserved for confirmed launch blockers.

- Goal: establish that current production artifacts can build and identify exact
  installation/upgrade prerequisites before the integrated rehearsal.
- Scope: the three production Dockerfiles, build contexts/lockfiles, Compose and
  runtime configuration validation, plus narrowly necessary build compatibility
  repairs and operator/evidence docs. No features, broad dependency/toolchain
  upgrades, schema changes or hosted administration.
- Behavior/invariants: one immutable source candidate and matching API/web builds;
  retain both SQLx vendor patches, the production seed exclusion, private database,
  separate owner/runtime authority, exact schema gate, secure cookies and trusted
  proxy handling. Never weaken guards to make an image start.
- Validation: build all three supplied production Dockerfiles with locked package
  inputs; verify build context, installed binaries, Compose configuration and
  migration/runtime-role requirements. Distinguish actual image acceptance from
  host Rust/Node builds. Record exact tooling/network blockers; repair only a
  demonstrated in-scope build defect and run every affected ladder/review.
- Stop: publish reproducible image/configuration evidence and the concrete next
  rehearsal step. Do not deploy to the user's host or start the next iteration.

## Sunday readiness path

Iteration 1 is complete. Work one bounded step per iteration; each next candidate
is defined before implementation. The six-iteration target is:

| Iteration | Required outcome |
| --- | --- |
| 2 | Current production images and deployment contract, above |
| 3 | Local production rehearsal: matching API/web, real schema 39/grants, production startup, health/readiness, secure-cookie browser access and SSE through Caddy |
| 4 | Critical organizer/player acceptance with distinct accounts: login/invitation, roster/round setup, score save/recovery, gross/net, lock/correction, final visibility and required Fantasy flow, at mobile/desktop widths |
| 5 | Actual current-application backup/checksum/restore into a fresh isolated project, state parity, real grants, readiness/login and restart/reconnect; short operating handoff |
| 6 | Release freeze: affected final gates, immutable tested candidate, READY / READY WITH KNOWN LIMITATIONS / NOT READY verdict, concise operator launch/rollback checklist |

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
  [A1z inventory](validation/lifecycle-expiry-investigation-2026-10-15/README.md)
  owns current deployment evidence gaps; it establishes no new P0–P2 product bug.
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
