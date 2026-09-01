# Latest explanation

## Round creation authorizes before interpreting the request

`POST /api/tournaments/{tournament_id}/rounds` used to deserialize and validate
the requested round before checking the caller's exact tournament role. An
authenticated non-admin could therefore distinguish malformed fields, the
tournament's configured round limit, or stored course/tee matches before the
write transaction eventually returned `403`.

The handler receives the untouched request and performs a repeatable-read,
active-session exact-admin preflight before it inspects headers or polls the
body:

```rust
rounds::preflight_create(
    &state.pool,
    authenticated.principal.session_id,
    tournament_id,
).await?;
```

Only a successful preflight reaches content-type checking, JSON decoding with
unknown-field rejection, or semantic validation. The post-authorization body
read is explicitly limited to 32 KiB; supported content types remain
`application/json` and `application/*+json`. An existing target without exact
admin membership returns `403` without polling its body; a missing tournament
returns `404`. Authentication and CSRF extraction remain earlier still, so an
inactive session returns `401` and a missing or invalid CSRF token returns
`403`. Global account role is never a tournament-authority bypass.

## The write transaction remains authoritative

Preflight controls response ordering but does not grant durable write authority.
`create_authorized` opens a separate transaction, revalidates the active session
and exact admin membership under locks, inserts the round, and commits. A session
revocation or membership change between preflight and insertion therefore
prevents the write. The payload-free round invalidation is published only after
the successful commit.

Request-only failures, target validation failures, missing targets, and every
unauthorized caller leave both round rows and live events unchanged. Existing
round constraints, scoring-format rules, course/tee integrity, lifecycle facts,
participants, teams, flights, handicaps, scores, and historical results are not
changed by this authorization-order release.

## Validation and remaining scope

Focused PostgreSQL coverage includes anonymous, invalid and expired sessions,
missing and invalid CSRF, scorer/player/viewer memberships, outsiders,
cross-tournament admins, global-role admins, missing targets, malformed JSON,
unknown fields, target-dependent validation, oversized and failed body streams,
post-preflight session revocation and membership removal, and successful exact-
admin creation. It proves forbidden bodies are not polled, error precedence,
zero writes/events on every rejection, and exactly one persisted round plus one
matching post-commit event on success.

The next step remains the exhaustive two-tournament roster, pairing, scoring,
scorecard, leaderboard, and event-stream isolation audit. No frontend caller or
database migration changed in this release.
