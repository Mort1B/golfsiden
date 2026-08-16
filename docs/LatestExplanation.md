# Latest explanation

## Reusable invitation onboarding

A tournament invitation is now a reusable, rotatable capability rather than a
recoverable plaintext secret. The URL carries a public UUID plus a 256-bit token
in its fragment; only the token hash reaches PostgreSQL. Preview and registration
authenticate that token before revealing lifecycle or account-specific errors.
New registration creates the linked account, player, handicap histories,
tournament membership, entrant, redemption, and session in one transaction.
Existing-account acceptance uses only the session's exact linked player.

Rotation creates a successor that retains the original series root, expiry, and
maximum uses. Application transactions and a database insert guard use one lock
order across identity, membership, invitation, series root, and entrant rows.
That prevents final-slot over-redemption and direct-SQL bypasses without adding a
second source of identity truth. Redemptions remain append-only, and legacy
revoked links explicitly report that their actor is unknown.

The React join page keeps the fragment through retry and reload recovery, sends
it only in JSON bodies, and clears it after success. TanStack keys and mutation
variables never contain the token. Tournament admins can issue, copy once,
rotate, and revoke links; plaintext exists only in current component state.

## Compact example

Series capacity is checked while the stable root is locked:

```sql
SELECT count(*)
FROM invitation_redemptions
WHERE tournament_id = target_invitation.tournament_id
  AND series_id = target_invitation.series_id;
```

## Invariants

- Tournament player identity remains independent of round teams and flights.
- Handicap snapshots and histories remain authoritative after joining.
- A user and linked player can redeem only once per tournament.
- Revoked, expired, closed, and exhausted links cannot create new redemptions.
- Complete active participation returns `already_joined` without consuming use.
- Raw invitation tokens never enter PostgreSQL, storage, logs, or request URLs.

## Validation

- Rust formatting, check, strict Clippy, and 39 unit tests pass.
- The complete PostgreSQL suite passes 111 integration tests.
- Frontend tests pass: 70 Vitest tests plus strict typecheck, lint, and build.
- Real Chrome flows pass at 320 px, 390 px, and desktop for join and admin
  workflows with no overflow, sub-44 px controls, console failures, storage
  leakage, or token-bearing request URLs.
