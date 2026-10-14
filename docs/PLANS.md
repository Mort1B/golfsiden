# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None.

## Next candidate

R2: avoid redundant Fantasy target reads after verified reconciliation.
Authorized by the user's request to fix all three assessed issues.

- Goal: remove a duplicate target request without weakening acknowledgment or
  authoritative refresh, including batch saves.
- Scope: Fantasy refresh coordinator/target helper, a focused ephemeral fetch-proof
  helper and recovery/request-count tests. Preserve full-root invalidation and SSE.
- Behavior: reuse an initially active target only after observing a new fetch and
  non-manual success during this refresh. Immediately recheck query identity,
  success counter, activity, data, idle/non-invalidated state and canonical
  session. Otherwise perform the existing explicit authorized target read.
- Invariants: every batch write still gets fresh preflight; acknowledgment never
  becomes uncertainty after a refresh failure; no write retries, stale cache reuse,
  retained projections or changes to privacy erasure/session ownership.
- Validation: preexisting in-flight reads, manual cache writes, cancellation,
  erasure/replacement, invalidation, inactive/new targets and activation failures;
  request-count regression; full frontend ladder and real browser recovery checks.
- Stop: document/publish R2, then activate the already-authorized R3 separately.

## Later queue

### Authorized optimization sequence

- R2: avoid redundant Fantasy target reads after verified reconciliation;
  three/five-round batches started 24/60 round reads in the local measurement.
  Preserve authoritative preflight and all session/privacy/recovery fences.
- R3: profile round-results projection before specializing its all-tournament
  calculation. No locking or authorization changes are approved.
- Evidence and boundaries are in the
  [pre-deployment assessment](validation/predeployment-review-2026-10-14/README.md).
  R3 is authorized after R2, with profiling and its own bounded implementation step.

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
