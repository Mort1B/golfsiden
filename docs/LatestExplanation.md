# Latest explanation

## Live leaderboard APIs

The backend now exposes separate gross and net routes for round and tournament
leaderboards. Round entries are format-aware: individual owners come from opening
snapshots, while scramble owners come from frozen teams and use the shared 35/15
handicap formula. Live partial positions compare strokes to par on the holes
actually scored, so entering holes out of order remains correct.

Tournament assembly starts from registrations, then attributes each completed or
locked round once per player. Individual scores follow the snapshot owner;
scramble scores are copied to each member of that round's team. Team changes in
later rounds therefore cannot rewrite earlier attribution. Completed-round count
is ranked before the selected total, and equal selected results use competition
positions without gross/net cross-breaking.

Repositories issue bounded bulk reads inside one repeatable-read, read-only
transaction. Domain assembly rejects unexpected owners, holes, duplicates,
missing scramble snapshots, incomplete completed rounds, and invalid
confirmations. A corrected completed card may be unconfirmed but remains complete
and included, matching the lifecycle contract.

## Compact example

Scramble results are attributed through the membership carried by that exact
round entry:

```rust
for member in entry.members {
    add_result(
        &participants,
        &mut totals,
        &mut attributed,
        round.round.round_id,
        member.player_id,
        entry.gross_total,
        entry.net_total,
    )?;
}
```

## Validation

- Formatting, diff checks, and Clippy with all features and warnings denied pass.
- The standard suite passes 28 tests; the PostgreSQL feature suite passes 63.
- API coverage includes exact round/tournament JSON, gross/net separation,
  completed corrections, multiple open rounds, invalid stored data, and
  repeatable-read concurrency.
- Handicap-disabled scramble scorecards and leaderboards both report zero playing
  handicap with net equal to gross.
