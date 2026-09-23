# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None.

## Next candidate

**PERSIST-1: preserve transient match notes across same-account sessions (proposed).**

Goal: prevent silent loss of unsaved or failed-save match notes when a renewed
session for the same account remounts the private workspace. Scope an account-owned
transient match-intent boundary, matching UI recovery/guards and regression tests,
based on the [confirmed assessment](validation/browser-persistence-2026-09-23/README.md).

Behavior: preserve local note values and explicit unsaved/recovery state across
same-account CSRF changes. Clear transient state on logout/account change; prevent
late writes/callbacks from restoring another account's state. Do not claim durable
storage until IndexedDB commits. Preserve durable queue ownership, conditional
revisions, existing match rules, round locks and administrator corrections.

Validation: failing-first provider/lifecycle tests and real Chrome probes for
unsaved and failed-save notes, same-session return, same-account replacement,
account teardown and durable-queue controls; affected validation ladder and
independent read-only review. Use only synthetic disposable local services.

Stop after this repair, evidence and documentation. Keep PERSIST-2/PERSIST-3 and
operational review separate. No production, external targets or real credentials;
retain local-only scope.

## Later queue

No queued work is currently approved.

- PERSIST-2: bound score-queue retries after device storage failure; preserve local
  intent/error and browser responsiveness. Define its own repair step.
- PERSIST-3: fence administrator mutation callbacks across session teardown;
  confirm related source-supported paths before expanding the repair.
- Remaining broader security assessment: production proxy/database privileges,
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
