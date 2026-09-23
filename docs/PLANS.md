# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. No implementation step is currently approved.

## Next candidate

**ADMIN-CB-1: final-round visibility callback ownership (proposed).**

Goal: resolve the source-supported concern that a late visibility mutation writes
private query data or invalidates projections after its initiating session departs.
Scope `FinalRoundVisibilityControl` and its response/error lifetime boundary only;
see the [read-only findings](validation/stableford-settings-lifetime-2026-09-23/README.md).

First reproduce with held success and stale-error responses across logout,
account/session replacement and unmount. If confirmed, fence cache, refetch and
local completion effects to the initiating mounted account/CSRF/target, using the
smallest appropriate boundary. Preserve final-round visibility rules, server
version checks, current-session errors and normal result-projection invalidation.
Do not infer server cancellation or change other administrator workflows. If not
reproduced, document the evidence and stop without speculative implementation.

Validate current-session controls, delayed refresh continuations and replacement
input; run the affected frontend ladder, Chrome and independent read-only review.
Use disposable local services and synthetic accounts only; no production,
external test targets or real credentials. Commit completed validated work to main,
push origin/main and verify clean alignment. Stop after this bounded follow-up.

## Later queue

- Pairing editor and tournament-start mutation callbacks: source-supported lifetime
  concerns awaiting separate reproduction and bounded repair decisions.
- Remaining broader security assessment: production proxy/database privileges,
  recovery operations and dependency advisories. Define a bounded next scope and
  authorized environment before proceeding.

### Remaining deployment acceptance

Repeat the PERSIST-1 actual-login and database-backed match browser controls once
local Docker permissions allow a disposable PostgreSQL service. The repair has
unit/component and real Chrome evidence with synthetic API responses; see its
[validation limits](validation/match-note-retention-2026-09-23/README.md).

Public-host acceptance still needs a Docker Engine environment and public DNS/TLS
validation. Native 200% browser zoom and physical Android Chrome remain unverified.
Confirm the available environment before defining the next bounded acceptance step.

No automatic opponents/byes/brackets, team match play, extra holes, new scoring
rules, public match sharing, cold offline launch or background sync is included.
