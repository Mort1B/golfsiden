# Latest explanation

## Username accounts and fixed tournament handicaps

Account identity now uses a normalized username and password in login, creator
onboarding, and invitation registration. The forward migration derives stable,
collision-safe usernames from existing account emails, preserves every user ID,
password hash, session, and foreign key, then removes account email. Optional
player-profile email remains separate and has no authentication meaning.

A tournament's registered handicap can now change only through an explicit,
audited admin correction before the first round opens. The repository locks
rounds and authorization state in deterministic order; PostgreSQL independently
requires correction context, verifies the tournament admin, writes append-only
history, and records a durable lock marker when opening or snapshot capture first
occurs. Deleting later round data cannot reopen the correction window.

Handicap calculation remains a pure domain concern. Individual stroke play keeps
the full registered index. Team scramble caps each member at `36.0` before the
selected tee's slope/rating conversion, and the resulting snapshot records that
effective input. The React client accepts Norwegian comma or point input, always
displays one decimal with a comma, and refetches authoritative roster state after
correction or an opening race.

## Compact example

The format policy is applied before course-handicap conversion:

```rust
let effective_index = effective_index_tenths(scoring_format, registered_index);
let handicap = calculate(effective_index, slope, rating, par, allowance, true, scoring_format);
```

## Invariants

- User and player UUIDs remain the authorization and identity links.
- Tournament handicap history and round snapshots remain immutable.
- No correction can change an opened or historical round.
- Scramble's cap does not alter the registered tournament value or individual play.
- Tournament players remain independent of changing round teams.

## Validation

- Rust formatting, check, strict Clippy, and 41 unit tests pass.
- The complete PostgreSQL-enabled suite passes 120 tests in total.
- Frontend validation passes 90 Vitest tests, strict typecheck, lint, and build.
- Headless Chrome passes the authenticated correction flow at 320, 390, and
  1280 px with comma input, an audited receipt, no overflow, and 44 px controls.
