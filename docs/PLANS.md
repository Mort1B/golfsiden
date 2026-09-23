# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. No implementation step is currently approved.

## Next candidate

**Preserve active rate limits at capacity (AUTH-1; proposed, awaiting approval).**

Goal: prevent cross-route key churn, including rejected requests, from discarding
unexpired login limits. Scope the repair to limiter admission/eviction behavior,
its regressions and affected documentation. Preserve existing route limits,
expiry recovery, bounded storage and the authentication contract.

Validation: reproduce the confirmed failure, prove both narrow and client limits
survive rejected cross-route churn at small and production capacity, and check
expiry recovery and memory bounds. Run the affected ladder and independent review.
Use only disposable local services and synthetic accounts.

Stop after this repair; do not include AUTH-2 or broader security changes.
Evidence: [local authentication assessment](validation/authentication-2026-09-23/README.md).

## Later queue

No queued work is currently approved.

- AUTH-2: revalidate session expiry after handicap-correction waits and before
  commit, with rollback/audit/event regression coverage.
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
