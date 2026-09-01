# Latest explanation

## Scorecards are private tournament reads

Reading a scorecard now requires an active session and any exact membership role
in the round's tournament. The repository resolves the round first, revalidates
the session, locks the membership `FOR SHARE`, and assembles the complete card in
one repeatable-read transaction. Missing rounds return `404`, existing rounds
outside the account's memberships return `403`, and successful responses use
`Cache-Control: private, no-store`.

```rust
let summary = scorecards::get_authenticated(
    &state.pool,
    authenticated.principal.session_id,
    round_id,
    owner,
).await?;
```

This changes read visibility only. Player/team ownership, flight-wide write
authority, preserved handicap snapshots, confirmation, audit, and locked-round
rules remain unchanged.

## Live invalidation has an exact tournament boundary

The public global `/api/live` feed is gone. A protected page connects only to
`/api/tournaments/{tournament_id}/live`. The handshake distinguishes inactive
sessions, missing tournaments, and absent memberships with `401`, `404`, and
`403`. Every internal post-commit notification now carries its tournament scope;
the stream filters by that scope and revalidates the active session and exact
membership immediately before emitting a matching event.

Only the SSE event type is serialized. Tournament IDs, resource IDs, players,
owners, and scores remain server-internal, so a frame is simply:

```text
event: score
```

The React application shares one credentialed stream for the current account and
selected tournament, including across Strict Mode cleanup/remount. Route or
selection changes release the old stream. Initial connection and every native
reconnect invalidate the current user's private workspace, while a lagged server
receiver closes so that reconnect performs the same authoritative resync.
Scorecard query keys now live below
`private-workspace/<user>`, so logout or a changed identity removes every prior
card before the next account is published; a same-user session refresh preserves
the active scoring workspace.

## Validation and remaining scope

Focused PostgreSQL tests cover all four tournament roles, global-role non-bypass,
`401`/`403`/`404`, private cache headers, two-tournament event isolation,
payload-free frames, session revocation, membership removal, and retirement of
the old live route. Channel-overflow coverage proves a lagged stream closes for
resynchronization. Frontend tests cover user-owned scorecard keys, identity
cleanup, same-user continuity, one shared target stream, all event types,
connection/reconnection invalidation, target changes, and cleanup. The full
Rust/PostgreSQL and frontend ladders plus real browser scoring and live-refetch
checks complete the release gate.

The next isolation slice remains the admin roster mutation, round-creation
authorization ordering, and the exhaustive two-tournament read/mutation audit.
Public scorecards, actor-field minimization, and final-round visibility remain
explicit Phase 7 work.
