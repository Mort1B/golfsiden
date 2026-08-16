# Latest explanation

## Atomic creator onboarding

A first-time visitor now creates one player identity, one global credential
account, and one tournament-scoped admin membership without receiving platform
authority. The submitted rounds are the source for the server-derived round
count and individual, team, or combined tournament scoring mode. Course and tee
references remain unconfigured in draft, so existing readiness rules still
prevent opening incomplete rounds.

Password hashing runs off Tokio's async executor under a four-task semaphore.
The owned permit moves into the blocking closure, which keeps the limit correct
even when the HTTP request is cancelled. PostgreSQL then creates both handicap
histories, tournament graph, hashed invitation, and hashed session in one
transaction. Cookies and SSE invalidation happen only after commit.

The frontend keeps the password in retryable form state and the returned invite
secret only in the one-time success view. It seeds non-secret tournament caches,
keys membership data by the session user ID, and clears identity-scoped caches
when accounts change.

## Compact example

The database requires an invitation creator to belong to the same tournament:

```sql
FOREIGN KEY (tournament_id, created_by_user_id)
    REFERENCES tournament_memberships(tournament_id, user_id)
    ON DELETE CASCADE
```

## Invariants

- Creator authority is tournament-scoped; the global account role is `player`.
- Player, entrant, team, and account identities remain separate.
- Both global and tournament handicap histories start with the exact submitted
  handicap.
- Raw password, session token, and invitation token never enter PostgreSQL or
  application logs.
- Failed transactions produce no onboarding rows, cookie, or SSE event.

## Validation

- Rust format, check, strict Clippy, and the complete PostgreSQL suite pass: 36
  unit tests and 52 integration tests.
- Frontend tests pass: 45 Vitest tests plus strict typecheck, lint, and build.
- Real Chrome flows pass at 320/390 px and desktop. A two-round mixed-format
  tournament was created and opened with no console errors, failed requests, or
  horizontal overflow.
