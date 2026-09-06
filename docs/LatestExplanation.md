# Latest iteration: Self-service user profile

Signed-in users can choose **Profil** to see current/archived tournament
memberships and change name, username, password and current profile handicap.
The page also links to creating another tournament.

## Behavior and boundaries

The API is self-only: no global user/player editing authority is restored.
Names update the account and linked player, including names displayed in past
results. Handicap accepts comma or point and displays Norwegian formatting.
Changes require an actor-attributed reason. Existing tournament handicaps,
including draft entries, and preserved round snapshots, scores, confirmations,
team ownership and membership are never rewritten. An unlinked account can
explicitly create its own player profile without entering existing tournaments;
inactive players cannot self-change handicap or reactivate themselves.

Username and password changes require the current password. Optimistic account
versions and player timestamps reject stale edits. The UI explains duplicate
usernames, incorrect passwords, disabled handicaps, saving, refetch errors and
uncertain outcomes. Successful edits refresh authoritative profile/session data.

Schema 23 adds profile versions and credential generations. Existing valid
sessions survive migration; a password change invalidates every session and
returns the browser to sign-in with confirmation. Central session authorization
and supplemental database lifecycle guards enforce generation equality without
acquiring other sessions' locks. Bounded Argon2 work occurs outside transactions;
the verified hash/generation, version and session are rechecked under locks, with
wall-clock expiry after waits. Login rechecks verified username, hash and
generation through session insertion, preventing stale verification races.

Frontend reconciliation matches user ID and CSRF token even after page departure.
Late private reads reject replacement sessions. Password success leaves the
protected route before publishing null identity, preserving its confirmation
instead of triggering the ordinary route redirect. Inactive credential-bearing
mutation state is removed immediately.

## Validation and review

- Formatting, 114 Rust workspace/all-target tests and all-feature Clippy with
  warnings denied passed.
- All 355 PostgreSQL-backed tests passed. New coverage includes self-only/CSRF
  authority, strict payloads, throttling, duplicate/stale/password failures,
  unlinked/inactive profiles, audited handicap changes, actual locked scores
  and snapshots remaining unchanged, all-device invalidation, controlled login
  lock races, expiry after waits, and completion/archive/visibility guards.
- Schema-22 upgrade preserves valid versus expired/revoked sessions. Fresh
  schema-23 migration and two development seed runs passed on PostgreSQL 17.
  Historical migration fixtures now use their original schema contracts rather
  than current-schema session helpers; preserved-fact assertions remain intact.
- All 359 frontend tests across 60 files, strict typecheck, ESLint and production
  build passed. Tests cover delayed responses, departure, fresh same-user
  sessions, late reads, private cache clearing and authoritative refetch.
- Two real Chrome scenarios passed at 320, 390 and 1280px. Real writes covered
  names, comma handicap, preserved existing tournament handicap, duplicate and
  incorrect-password failures, cross-device stale edits, password confirmation,
  other-device rejection and fresh login. Loading, retry, empty/unlinked,
  archived and inactive states used explicit browser response fixtures.
  Screenshots were inspected; width and primary touch targets were checked.
  No unexpected console/network errors remained. Deliberate 409s, anonymous
  session 401s, SSE closure and Chrome's aborted reads after confirmed 204s were
  classified explicitly.
- Independent read-only review found a late-response navigation issue, which
  was fixed and regression-tested. Final review reported no blocking findings.

An initial concurrent full run hit the existing score-ordering test's 50ms timing
assumption. Its focused rerun and subsequent complete database ladder passed
without changing scoring code or that test. The existing Vite bundle warning
remains non-blocking (approximately 604kB before gzip).

## Example and release boundary

A player entered a trip at 8.2, then changes profile handicap to 14.4 with a
reason. That trip keeps 8.2 and its preserved results; a later entry uses 14.4.
Renaming changes the displayed name, not historical ownership or score data.

**READY WITH KNOWN LIMITATIONS:** the bounded profile workflow is validated.
Email/avatars, account deletion/recovery and editing other users are excluded.
Production migration/deployment is separate: back up, migrate schema 23 with
owner authority, refresh permissions and deploy matching binaries with
RUN_MIGRATIONS=false, following deployment_guide.md. No retained or production
database was migrated or seeded by this task.
