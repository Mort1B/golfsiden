# Latest explanation

## Audited scorecards

The API now saves individual or team strokes through one tagged owner contract.
Every changed value is an audited transaction; an identical mobile retry is a true
no-op. Score identity, round format, opening snapshot eligibility, round tee, and
editable lifecycle state are checked in the repository and again by PostgreSQL.

All mutations serialize through the round row. Repository operations lock it
first. Trigger-protected direct SQL acquires the same lock with `NOWAIT`, so it
cannot create a row-to-round deadlock or race confirmation. Confirmation and
correction therefore have a deterministic order, and corrections remove the
current confirmation before another client can observe a confirmed stale card.

Scorecard reads use one repeatable-read snapshot. Individual net scores allocate
the preserved playing handicap by stroke index. Scramble calculates 35% of the
lower plus 15% of the higher frozen course handicap with integer-ratio rounding,
then applies the round allowance once. Current player handicaps are never read.

## Compact example

The same-value branch commits without touching the row or publishing SSE:

```rust
if let Some(score) = existing.as_ref()
    && score.gross_strokes == input.gross_strokes
{
    transaction.commit().await?;
    return Ok(MutationResult {
        value: score.clone(),
        changed: false,
    });
}
```

## Validation

- Formatting passed and Clippy passed for every target/feature with warnings
  denied.
- Standard tests passed 17 unit tests. PostgreSQL feature tests passed 36 tests:
  17 unit, 4 database rules, 10 round lifecycle, and 5 scorecard tests.
- PostgreSQL 17.10 clean migration and seed passed with migrations 1-3, eight
  players, five rounds, eighteen holes, eight teams, and sixteen memberships.
- An isolated database successfully upgraded seeded migration 2 state to migration
  3 without changing those entity counts.
