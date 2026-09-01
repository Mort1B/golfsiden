# Latest explanation

## Global player discovery is retired

The web product no longer exposes a global player, profile, or handicap-history
directory. Every former method under `/api/players` is unrouted, the `/players`
page and navigation item are gone, and the frontend no longer contains a global
player client or DTO. Preserved `players` and handicap-history rows remain in
PostgreSQL because accounts, invitations, tournament registrations, snapshots,
and historical results still reference them.

Tournament discovery did not become global as a replacement. The remaining
collection route keeps its authenticated membership filter:

```rust
let tournaments = tournaments::list_for_member(
    &state.pool,
    authenticated.principal.user_id,
).await?;
Ok(([(CACHE_CONTROL, "private, no-store")], Json(tournaments)))
```

The legacy `POST /api/tournaments` route and its platform-admin extractor and
repository path are also removed. Creator onboarding is the product-facing
tournament creation boundary and atomically grants only the new tournament's
admin membership. A global account role grants no tournament listing or
administration access, and it cannot use a separate legacy creation path.

## Frontend and compatibility boundaries

The application now has three primary navigation items. Mobile and desktop CSS
explicitly lays out that three-item navigation, and an old `/players` bookmark
falls through to the established router error page without issuing an API
request. Tournament management continues to use the identity- and tournament-
scoped private roster query; roster loading, correction, and invalidation
semantics were not changed.

No migration was needed. This release removes transport surfaces rather than
stored identity or history, and existing tournament/player composite keys remain
the database isolation boundary.

## Validation and remaining audit

Regression coverage checks every retired player method for anonymous, member,
and global-admin sessions, proves legacy tournament creation cannot mutate the
database or emit SSE, and confirms authenticated tournament lists and rosters
remain membership-scoped and `private, no-store`. Rust formatting, default and
PostgreSQL tests, strict Clippy, frontend tests, typecheck, lint, build, and Chrome
checks at 375px and 1440px cover the release.

The next isolation slice must address the remaining scorecard/live-event read
policy, arbitrary global player IDs in the admin roster mutation, authorization
ordering in round creation, and the complete two-tournament read/mutation audit.
