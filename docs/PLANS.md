# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None.

## Next candidate

R1: make the Fantasy preset browser acceptance deterministic (test-only).

- Goal: resolve the reproduced 503-only assertion failure before treating the
  browser suite as an unattended release gate; see the
  [assessment](validation/predeployment-review-2026-10-14/README.md#r1--p2-browser-acceptance-rejects-recoverable-projection-conflicts).
- Scope: `frontend/e2e/fantasyPreset.browser.ts` and a focused response-observer
  helper/test if needed. No production, migration, dependency or hosting changes.
- Behavior: accept the deliberately injected 503 and verified retryable
  `409 fantasy_conflict` only on expected reads; assert affected projection
  recovery. Preserve rejection of unexpected status/code and transport failures.
  Permit unread bodies only with positive evidence of expected read cancellation.
- Validation: deterministic conflict/recovery and unexpected-409 rejection cases,
  repeated real-API Chrome runs, strict browser TypeScript and affected frontend
  checks. Preserve existing draft, privacy and closed-destination assertions.
- Invariants: exact write requests, recovery, session/privacy boundaries, scoring
  rules and production retry behavior remain unchanged.
- Stop: document/publish only the test repair, then stop before optimization.

Every implementation step updates affected durable docs and LatestExplanation,
commits only its scope and stops before the next step.

## Later queue

### Assessed optimization candidates

- R2: avoid redundant Fantasy target reads after verified reconciliation;
  three/five-round batches started 24/60 round reads in the local measurement.
  Preserve authoritative preflight and all session/privacy/recovery fences.
- R3: profile round-results projection before specializing its all-tournament
  calculation. No locking or authorization changes are approved.
- Evidence and boundaries are in the
  [pre-deployment assessment](validation/predeployment-review-2026-10-14/README.md).
  These require their own bounded implementation steps.

### Previously deferred work

- Optional offline preparation for browser closure/eviction: requires a separately
  scoped app-shell and private scorecard-retention/reauthorization design before
  implementation. Current in-memory return and durable queued edits remain distinct.

- Deferred broader security assessment: further production proxy/database privilege,
  recovery and dependency-advisory investigation. Revisit for a specific finding
  or a separately requested assessment, rather than making another broad review
  a prerequisite for continued functional testing.

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
