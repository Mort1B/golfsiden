# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None.

## Next candidate

**Local browser/offline persistence assessment (proposed).**

Goal: verify that private tournament data, queued score mutations and capability
state remain scoped to the correct account and tournament across logout,
account changes, membership loss, page return and offline/online transitions.
Scope existing browser storage, query cache, service worker, queued mutation
ownership and relevant source/tests using synthetic disposable local services.

Behavior/invariants: assessment only; no implementation, dependency or operator
repairs. Preserve score ownership, historical snapshots, round locks, explicit
administrator correction paths and existing offline delivery rules. No
production, external targets or real credentials. Retain local-only scope.

Validation: inspect source and existing tests, reproduce concrete boundary
concerns locally, exercise relevant real Chrome mobile/desktop flows, and obtain
independent read-only review. Separate confirmed findings, source-supported
concerns and untested assumptions; record exact safeguard notices and limits.

Stop after the report and one bounded next candidate. Keep any remediation and
operational/dependency assessment separate.

## Later queue

No queued work is currently approved.

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
