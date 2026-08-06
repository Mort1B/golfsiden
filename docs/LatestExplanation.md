# Latest explanation

## Authenticated team scoring

Scoring no longer trusts a build-time browser user ID. Login verifies an Argon2
hash, stores only a hash of a random session token, and sends the raw token in an
HttpOnly cookie. Save, confirmation, and logout requests also require a derived
CSRF value. The backend supplies audit actors from the session and rejects old
`submitted_by` or `confirmed_by` request fields.

One score-access repository owns the authorization rule. Admins and scorers can
write every eligible card. A player can write their own individual card or the
team containing their linked player in that exact round. This means either
member can enter and correct a scramble team's shared score while unrelated,
viewer, and unlinked users remain read-only. Authorization is repeated inside
the score transaction with a session-row lock, so UI filtering is never the
security boundary.

The React app bootstraps the cookie session, preserves protected score URLs
through sign-in, and selects only owners returned by `/score-access`. It sends no
user IDs in mutation bodies. Auth failures remain explicit, non-retryable score
intent until the scorer discards it or signs in again.

## Compact example

The team rule is based on normalized membership for the selected round:

```sql
SELECT EXISTS(
    SELECT 1 FROM team_memberships
    WHERE round_id = $1 AND team_id = $2 AND player_id = $3
)
```

Future flights will extend the resolver's owner set to both flight teams. They
will not be inferred from a shared starting hole or tee time.

## Invariants

- Raw passwords and session tokens are not logged or stored.
- Score and confirmation actors always come from the active session.
- Both teammates share score access without sharing an account.
- Round locks, audit triggers, snapshots, and backend net calculations remain authoritative.
- Public scorecard and leaderboard reads remain public; other mutation auth is explicitly deferred.

## Validation

- Rust format and Clippy with warnings denied pass.
- 30 Rust unit tests and 40 PostgreSQL/API integration tests pass.
- 25 frontend tests, strict TypeScript, ESLint, and the production build pass.
- A disposable database passes all five migrations plus two idempotent seed runs.
- Real Chrome passes authenticated admin, both-teammate, non-member read-only,
  forced `403`, refresh, logout failure/success, unresolved-write guard, cookie,
  console/network, and 320px/390px/desktop layout checks.
