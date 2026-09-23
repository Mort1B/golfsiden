# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. No implementation step is currently approved.

## Next candidate

**PERSIST-2: bound score-queue retries after device storage failure (proposed).**

Goal: prevent the score delivery runner from repeatedly draining eligible in-memory
work when IndexedDB fails. Scope the existing score runtime retry scheduling and
focused regression tests, based on the
[confirmed assessment](validation/browser-persistence-2026-09-23/README.md).

Behavior: retain durable/local intent and a visible storage error, yield browser
execution between bounded retry attempts and recover on an explicit retry or
existing reconnect/return signal. Preserve immutable conditional requests,
account/session fences, leases and conflict handling. No queue format, server API
or match-rule change. Keep PERSIST-3 separate.

Validation: reproduce the failing storage/timer schedule with bounded synthetic
faults; prove responsiveness, bounded attempts, preserved queue identity and
recovery after storage returns. Run focused tests, the affected frontend ladder,
Chrome and independent read-only review. Use only disposable local services and
synthetic accounts. Security validation remains local-only: no production,
external test targets or real credentials. Commit completed, validated work to
`main` and push to `origin/main`, as explicitly requested by the owner. Stop after
the repair, evidence, documentation and verified publication.

## Later queue

- PERSIST-3: fence administrator mutation callbacks across session teardown;
  confirm related source-supported paths before expanding the repair.
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
