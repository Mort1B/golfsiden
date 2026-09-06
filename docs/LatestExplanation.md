# Latest iteration: Tournament completion backend and database

The approved closure contract now has a backend action:
`POST /api/tournaments/{tournament_id}/complete`. An exact tournament admin with
an active session and CSRF token can explicitly complete an active tournament
only when its entire configured round plan exists and is locked. The body contains
only `expected_tournament_updated_at`; success uses the existing tournament DTO.
Completed retries are idempotent and emit no duplicate event or completion record.

## Enforcement and preserved behavior

The repository locks rounds in UUID order, reauthorizes after waits, locks the
parent and checks wall-clock session expiry and current readiness. Stale active
requests, invalid source states and incomplete plans have distinct conflict codes.
A committed transition publishes one tournament invalidation event.

Migration 0019 independently enforces exact session context and locked-round
readiness. Workflow-created completion records preserve actor and database time;
runtime DML cannot forge, edit or delete them. Deleting an otherwise-deletable
account may null its audit actor FK without deleting completion identity/time.
Runtime roles cannot create triggers; schema owners remain trusted migration
authority. Closed round plans are protected against insertion, moves and deletion.

Invitation creation/rotation and new joins hold a shared parent lock until commit.
If joining wins, it may finish before closure; if completion wins, joining fails
and any new registration identity data rolls back. Identity-changing membership
and entrant updates cannot bypass the closed-parent check. Completion does not
lock or revoke invitation rows. Existing members retain private reads; invitation
revocation and independent administrator final-result visibility remain available.
No score, snapshot, team, confirmation or final-visibility data changes on closure.

Valid schema-18 completed/archived history is preserved without invented audit
actors. Incompatible closed history fails the upgrade atomically with a named
constraint; it is not automatically rewritten. The deployment guide contains a
read-only preflight and recovery guidance. Archiving and reopening remain
unavailable. Completion UI is a separately queued step.

## Validation and review

- Formatting, 114 backend unit tests and strict all-target/all-feature Clippy
  passed. The final full PostgreSQL-enabled ladder passed all 315 tests: those
  114 unit tests plus 201 integration tests.
- Twelve new completion tests cover API auth/CSRF/shape/stale/readiness/errors,
  exact actor and idempotent event behavior, preserved scores/visibility/member
  reads, direct-SQL guards, concurrent completion, administrator demotion after
  round-lock waits, session expiry after repository and SQL-trigger waits,
  append-only audit behavior, and account-deletion FK policy.
- Closed invitation accept/register/rotate reject with the established conflict;
  revoke and visibility updates still work. An actual registration blocked on its
  parent membership lock was overtaken by closure and rolled back all new
  user/player/session data. Invitation insertion winning the parent lock was also
  tested before waiting completion.
- Fresh PostgreSQL 17 migration and two seed runs passed. Schema-18 upgrade tests
  cover unchanged valid archived history and atomic failure for incompatible
  completed history. The full suite includes existing round, membership, private
  read, final visibility, invitation concurrency and schema-readiness regressions.
- Review identified three P2 database bypasses: mutable completion evidence,
  identity-changing membership moves and session expiry during SQL lock waits.
  All were fixed with regression coverage. Final read-only review had no findings.

Legacy start/readiness tests now explicitly construct synthetic pre-schema-19
terminal states before testing defensive behavior. The invitation rollback test
injects the named database closure error instead of forcing an illegal closure.
The new completion tests exercise the real guarded workflow with no bypass.
An initial unit run hit sandbox-denied loopback mock servers; the complete ladder
passed with the required local-network permission. Earlier fixture/compiler
failures were corrected before the final passing runs.

Frontend tests/browser flows and production deployment/restore drills were not
repeated: this step has no frontend changes and was validated against disposable
PostgreSQL, not a deployed environment. The new migration must be applied by the
owner, followed by the permissions action; runtime startup migrations stay off.

## Example and release verdict

Locking the final round does not automatically close the tournament. The admin
explicitly completes it with the current tournament timestamp; another identical
request returns the same completed resource without a second audit entry/event.
Hidden final results remain hidden until the separate visibility action is used.

**READY WITH KNOWN LIMITATIONS:** backend/database checks pass; completion UI,
archive behavior, production deployment and recovery drills remain outside this
bounded iteration. No archive action or UI was introduced.
