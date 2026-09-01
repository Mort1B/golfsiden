# Latest explanation

## Tournament participation no longer accepts global player IDs

`POST /api/tournaments/{tournament_id}/players` has been retired. It was an
unused backend-only mutation that allowed an exact tournament admin to submit an
arbitrary global `player_id`. Although the admin check was tournament-scoped,
the write created `tournament_players` and handicap history without creating the
account's matching `tournament_memberships` row.

The roster collection is now GET-only:

```rust
.route(
    "/api/tournaments/{tournament_id}/players",
    get(list_players),
)
```

POST requests on that retained path return `405 Method Not Allowed` for every
caller and perform no database write or live notification. The private roster
read and audited pre-opening handicap-correction endpoint are unchanged.

## Registration is account-linked and tournament-specific

Creator onboarding and invitation redemption are now the only product HTTP
paths that establish tournament participation. Onboarding creates the creator's
linked account/player, exact admin membership, entrant, and initial handicap
history in one transaction. Invitation registration creates a new linked
account/player; invitation acceptance uses only the authenticated account's
existing `users.player_id`. Both invitation paths create or repair the exact
target membership and entrant and append the target handicap and redemption
facts atomically.

No global player directory, account search, or admin-forced player picker was
introduced. Joining still creates no team or flight membership, and tournament
handicap audit and permanent locking rules remain unchanged.

## Validation and remaining scope

Focused PostgreSQL coverage exercises anonymous, invalid-session, member,
scorer, viewer, cross-tournament admin, exact-admin, and global-role callers.
Every retired POST returns `405`; aggregate membership, entrant, handicap-history,
and redemption counts remain unchanged, and no SSE event is emitted. Existing
invitation, concurrency, redemption-guard, onboarding, and seed suites preserve
the supported registration contracts.

The next isolation slice is authorization ordering for round creation, followed
by the exhaustive two-tournament roster, pairing, scoring, scorecard,
leaderboard, and event-stream audit. A future database-level coupling of legacy
entrant rows to account memberships requires an explicit orphan/backfill policy
and is not hidden inside this HTTP-boundary release.
