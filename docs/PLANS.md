# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None.

## Next candidate

**SHARE-1: recheck result-link mutation sessions after writes (proposed).**

Goal: prevent public-link mutations from committing when their session expires
during a late grant/audit write wait. Scope the result-sharing management
repository and focused regression tests, based on the
[confirmed assessment](validation/result-projection-security-2026-09-23/README.md).

Behavior: recheck active session validity after writes immediately before commit.
Detected expiry returns 401, rolls back grant/audit changes and emits no event.
Preserve exact tournament authority, lock order, expected-grant intent, valid
issue/replace/revoke behavior and public projection contracts. Do not claim
atomic expiry during COMMIT itself.

Validation: controlled late-wait issue/replacement/revoke expiry cases and valid
controls using synthetic disposable PostgreSQL, relevant authorization and
sharing tests, affected validation ladder and independent read-only review.

Stop after this repair, evidence and documentation. No broader lifecycle,
projection, persistence or operator changes. Retain local-only scope.

## Later queue

No queued work is currently approved.

- Remaining broader security assessment: browser/offline persistence,
  production proxy/database privileges, recovery operations and dependency
  advisories. Define a bounded next scope and authorized environment before proceeding.

### Remaining deployment acceptance

Public-host acceptance still needs a Docker Engine environment and public DNS/TLS
validation. Native 200% browser zoom and physical Android Chrome also remain
unverified. Confirm the available host/browser/device environment before defining
the next bounded acceptance step. Keep the separate security work above
distinct from these operational and device acceptance checks.

No automatic opponents/byes/brackets, team match play, extra holes, new scoring
rules, public match sharing, cold offline launch or background sync is included.
