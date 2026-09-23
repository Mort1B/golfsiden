# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. See the [latest explanation](LatestExplanation.md) for the completed step.

## Next candidate

**Local recovery capability and credential-revocation assessment (awaiting approval).**

Goal: assess administrator-assisted recovery issuance, preview, redemption,
expiry, revocation, single-use behavior and resulting session invalidation.
Scope source, existing tests and bounded disposable local validation with
synthetic accounts; report confirmed findings separately from unverified concerns.

Behavior/invariants: assessment only. Preserve exact tournament-admin authority,
account identity and privacy. Do not change application, migration, dependency or
operator code. Do not access production, external targets or real credentials.

Validation: inspect capability generation/storage and transport, CSRF/authorization,
credential-generation effects and transactional races. Use controlled local
reproductions where needed, record exact evidence and limits, and obtain
independent read-only review. Report any safeguard notice verbatim with its step.

Stop after the report and one bounded next candidate. No remediation or broader
security work is included. Keep work and commits local under the existing scope.

## Later queue

No queued work is currently approved.

- Remaining broader security assessment: private/public
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
