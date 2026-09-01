# Latest explanation

## Score access now requires exact tournament membership

`GET /api/rounds/{round_id}/score-access` previously treated a missing target
membership like an authorized non-writing role and returned `200` with an empty
owner list. That still confirmed the requested round existed and differed from
the private-read contract used by scorecards, leaderboards, pairings, and live
events.

The repository now holds the repeatable-read transaction and active-session lock,
resolves the round, and requires the share-locked membership row before owner
assembly:

```rust
let role = membership_role(&mut transaction, tournament_id, principal.user_id)
    .await?
    .ok_or(ScoreAuthorizationError::Forbidden)?;
```

A missing or revoked session remains `401`, a missing round remains `404`, and a
session without exact membership now receives `403`. Exact viewers and exact
player memberships without a usable player link remain authorized and receive an
empty writable-owner list. Score save and confirmation continue using the same
owner resolver and retain their existing denial behavior.

## One identity still means separate tournament participation

The acceptance fixture registers one global account/player independently in two
tournaments with different tournament handicaps. Opening creates distinct round
snapshots and flight assignments. Tournament A uses a player-owned card;
tournament B uses a B-only two-player foursomes team with its own membership and
preserved team-handicap snapshot.

The fixture proves target-local rosters, pairings, teams, writable owners,
player/team scorecards, and gross/net round and tournament results. An A-only
admin is denied B reads, score saves, and confirmations; those rejections persist
no score or confirmation and publish no invalidation event. The live stream first
ignores a distinguishable B-only notification and then emits only the payload-free
A notification.

No schema or frontend production code changed. The next bounded step is the
frontend tournament-target isolation slice: reset tournament-local drafts,
receipts, and invitation secrets on navigation, and reject mismatched target
identities before responses enter the private query cache.
