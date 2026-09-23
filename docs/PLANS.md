# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. See the [latest explanation](LatestExplanation.md) for the completed step.

## Next candidate

**Revalidate handicap-correction sessions after waits (AUTH-2; awaiting approval).**

Goal: prevent an initially authorized correction from committing after session
expiry during database waits. Scope the repair to the correction transaction,
necessary existing session-validation helpers, regressions and documentation.

Behavior/invariants: preserve lock order, exact tournament-admin authority,
handicap snapshots and audit semantics. Expiry before commit must return 401,
roll back handicap/audit changes and emit no invalidation. Valid-session and
no-op behavior must remain consistent with existing contracts.

Validation: reproduce the held parent-row wait with a near-expiry synthetic
session, prove rollback/no-event behavior and a nonexpiring control, and cover
membership waits if a shared helper changes. Run affected backend/database
ladders and independent review with disposable local services only.

Stop after AUTH-2; do not broaden into other security or deployment work.
Keep validation and commits local under the existing local-only scope.
Evidence: [authentication assessment](validation/authentication-2026-09-23/README.md).

## Later queue

No queued work is currently approved.

- Remaining broader security assessment: recovery lifecycle, private/public
  projections and offline persistence, production proxy/database privileges,
  recovery operations and dependency advisories. Define a bounded next scope and
  authorized environment before proceeding.

### Remaining deployment acceptance

Public-host acceptance still needs a Docker Engine environment and public DNS/TLS
validation. Native 200% browser zoom and physical Android Chrome also remain
unverified. Confirm the available host/browser/device environment before defining
the next bounded acceptance step. Keep the separate security work above
distinct from these operational and device acceptance checks.

No automatic opponents/byes/brackets, team match play, extra holes, new scoring
rules, public match sharing, cold offline launch or background sync is included.
