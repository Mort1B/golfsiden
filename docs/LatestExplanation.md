# Latest iteration: Tournament archive backend and database

An exact tournament administrator can explicitly archive a completed tournament
with `POST /api/tournaments/{tournament_id}/archive`. The body contains only
`expected_tournament_updated_at`; the existing session cookie and CSRF token are
required. Success returns the existing tournament object with archived status and
private/no-store caching.

This is a separate terminal action, not deletion or completion. Draft/active
sources conflict; stale completed requests conflict. An authorized archived retry
returns the unchanged current tournament even with the earlier expected timestamp,
without a second audit, timestamp change or live event.

## Enforcement and preserved behavior

The repository locks session/user and exact tournament membership, then the parent
tournament. It rechecks wall-clock session expiry after waits, including idempotent
retries. Round locks are unnecessary: archive neither reads nor writes round rows,
and completed plans are already immutable. A changed commit emits one tournament
invalidation; responses are never sent before commit.

Forward migration 0020 replaces schema 19's temporary archive rejection with an
independent exact-session archive guard. It leaves completion enforcement intact
and records the actor and database time in append-only `tournament_archives`.
Direct inserts, updates and deletes of evidence are rejected. The actor FK may
become null when an otherwise-deletable account is removed, retaining archive
identity/time. Schema owners remain trusted migration authority; runtime roles
cannot create triggers or alter the schema.

Scores, team composition, preserved handicap snapshots, confirmations, memberships
and final visibility do not change. Existing members retain private result and
scorecard access. Final-nine release/re-hide and invitation revocation remain
available; new invitations, rotations and joining remain closed. Archived rows
remain in current list reads. There is no archive UI, filtering or reopening yet.

Existing valid schema-19 completed/archived history is preserved without inventing
actors or requiring completion evidence that predates that workflow. Deployment
requires the owner migration and permissions actions; runtime startup migrations
remain off and exact schema compatibility gating remains in force.

## Validation and review

- Formatting, 114 backend unit tests and strict all-target/all-feature Clippy passed.
- The full PostgreSQL-enabled ladder passed all 331 tests: 114 unit tests and
  217 integration tests, including 16 new archive tests.
- Archive tests cover API auth/CSRF/strict body/stale/source-state errors,
  idempotent and concurrent event/audit behavior, direct-SQL context and real
  non-admin/foreign/revoked/expired sessions, append-only evidence and actor deletion.
- Real lock-wait tests cover membership demotion, session revocation, expiry after
  the parent lock for both initial archive and archived retry, and SQL-trigger
  expiry after a membership wait.
- An 18-hole fixture verifies unchanged gross/net, read-card and completion
  projections for existing members, hidden/released/re-hidden final visibility,
  closed issue/rotate/register, retained revocation, and read denial after membership
  removal. Stored rounds, scores, snapshots, confirmations, entrants, memberships,
  team collections and completion evidence are compared across archive.
- Schema-19 upgrades preserve legacy completed/archived rows without invented audit
  records and preserve workflow-created completion evidence. Fresh PostgreSQL 17
  migration and two seed runs passed; repeat migration reports the schema current.
- Independent read-only source, migration, test and documentation review found no
  unresolved issues. Its direct-SQL authorization coverage suggestion was added.

Initial fixture failures (missing draft UUID and invalid expired-session date range)
were corrected. The first full run also identified an older start test constructing
archived state with unguarded SQL. It now uses the archive repository while retaining
its existing synthetic pre-schema-19 completion fixture and start-rejection checks.
No production guard was weakened to make these tests pass.

Frontend tests and browser flows were not repeated because this bounded step has
no frontend changes; the API and privacy behavior were validated against real
PostgreSQL. Production deployment, runtime rollout and recovery drills were not
performed. No prior migration, runtime dependency or deployment configuration changed.

## Example and release verdict

After explicit tournament completion, archive using its current timestamp.
Repeating that request returns the same archived resource. Existing members still
see private history and any hidden final nine stay hidden until the independent
visibility action is used.

**READY WITH KNOWN LIMITATIONS:** the archive backend/database contract and affected
validation pass. Archive administrator UI and history filtering are the next
approval-gated step. Production migration/deployment and recovery checks remain
operator work, following the updated deployment guide.
