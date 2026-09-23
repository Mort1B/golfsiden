# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. No implementation step is currently approved.

## Next candidate

**PERSIST-3: fence late administrator mutation callbacks (proposed).**

Goal: prevent a delayed administrator response from recreating an old account's
cleared private query data after logout or session replacement. Start with the
confirmed Stableford-settings path in the
[assessment](validation/browser-persistence-2026-09-23/README.md). Scope that
mutation's identity/lifetime boundary and regression tests; inspect similar
callbacks read-only before proposing any separate expansion.

Behavior: apply cache writes, invalidation and local completion effects only while
the initiating account/session still owns the mounted operation. Current-session
success and deliberate errors remain visible. Session teardown must leave old
private caches cleared even if an earlier request later succeeds. Do not claim
client cancellation undoes a server write; a fresh authorized read recovers actual
server state. Preserve tournament roles, server authorization and settings rules.

Validation: held-response failing-first tests for logout, account change,
same-account replacement and unmount; normal success/failure controls; affected
frontend ladder, real Chrome and independent read-only review. Use synthetic
accounts and disposable local services only. No production, external test targets
or real credentials. Commit completed validated work to main, push origin/main
and verify clean alignment. Stop after that repair, evidence and documentation.

## Later queue

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
