# Latest explanation

## Membership-scoped private workspace reads

Tournament workspace reads now require both an active session and a membership
in the target tournament. This covers tournament detail and roster, rounds,
teams, readiness and completion validation, and round and tournament
leaderboards. The tournament collection is selected through the caller's
memberships rather than loaded globally. Missing sessions return `401`, existing
cross-tournament resources return `403`, and missing resources return `404`.
Successful private reads use `Cache-Control: private, no-store`.

Repository wrappers open one repeatable-read transaction, resolve round ownership
from stored relations, and lock the membership row for shared access before
loading the response. A concurrent membership removal therefore cannot commit
between authorization and assembly of a multi-query private response. Scoring,
handicap snapshots, round locking, response DTOs, and ordering are unchanged.

React protects tournament, round, score, and leaderboard routes. Every private
workspace query key starts with the session user ID. Initial session resolution
and real identity changes clear the entire private workspace cache before the new
identity becomes visible; same-user background refreshes retain it. SSE refreshes
workspace data but explicitly excludes the authentication query, so live events
cannot unmount an in-progress scoring screen.

## Compact example

The repository holds membership authorization and data loading in one snapshot:

```rust
let mut transaction = read_transaction(pool).await?;
require_round_member_read(&mut transaction, user_id, round_id).await?;
let result = load_private_round_data(&mut transaction, round_id).await?;
transaction.commit().await?;
```

## Invariants

- Global roles never bypass tournament membership for private reads.
- Tournament players remain independent of changing round teams.
- Handicap snapshots, score ownership, locking, audit history, and separate
  gross/net calculations are unchanged.
- Private browser data cannot survive a change of session identity.
- Public invitation preview, global player reads, scorecard visibility, and live
  event visibility remain separate policy decisions.

## Validation

- Rust formatting, strict Clippy, 41 unit tests, and 83 PostgreSQL integration
  tests pass.
- Frontend validation passes 98 Vitest tests, strict typecheck, lint, and build.
- Browser behavior passes at 375 and 1440 px for protected redirects, populated
  member pages, outsider denial, logout/login cache separation, and overflow.
  Empty membership is covered by PostgreSQL/API tests because the safe shared seed
  has no such account. Chrome also exposed a pre-existing invalid username input
  `pattern`; its console-only repair is recorded as the next bounded plan item.
