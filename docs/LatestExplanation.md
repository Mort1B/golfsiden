# Latest explanation

## Runtime flight-member score authority

Every authenticated tournament player linked to an exact round flight member can
now score every eligible card in that flight. In individual stroke play those
owners are the flight's snapshot-backed players. In scramble they are the exact-
round two-player teams whose complete membership is contained in that flight, so
one flight may expose several shared team cards to every member.

Flights grant write authority only. Player and team score ownership, preserved
handicap snapshots, confirmation state, audit attribution, and locked-round rules
are unchanged. Starting hole, tee time, display order, name, or one matching team
member never implies access. A player without stored flight membership retains
the pre-flight fallback to their own individual card or direct round team.

## One policy for listing and mutation

Score-access listing, hole saves/corrections, and card confirmation now call the
same deterministic owner resolver. Tournament admins and scorers retain every
eligible round owner. Player access still requires an exact tournament `player`
membership and a currently linked player identity; viewers, unlinked accounts,
non-members, and other-flight players receive no authority.

The resolver returns each tagged owner once, ordered by case-insensitive player
or team name and then UUID. Scramble eligibility requires exactly two team
members and two opening snapshots, and the flight branch rejects a team unless
all of those members share the actor's exact stored flight. For example, a player
in Team A can receive both complete teams in Flight 1 without receiving Team C in
Flight 2:

```json
{
  "round_id": "round-uuid",
  "writable_owners": [
    { "type": "team", "id": "team-b-uuid" },
    { "type": "team", "id": "team-a-uuid" }
  ]
}
```

## Transaction and privacy boundary

`GET /api/rounds/{round_id}/score-access` re-locks the active session and user,
then holds the exact tournament membership through owner assembly in a
repeatable-read transaction. Successful responses are `Cache-Control: private,
no-store`; the browser continues using its user-rooted private query key and
unchanged tagged-owner decoder.

Save and confirmation already lock the round before revalidating the session and
tournament membership. They now check membership in the same resolved owner set
inside that transaction. A successful peer write still records the authenticated
account as `submitted_by` or `confirmed_by`; a valid owner in another flight is
`403`, while an owner from another round remains the existing
`409 score_owner_not_eligible` validation error.

## Validation

The focused PostgreSQL suite covers peer individual cards, two teams in one
scramble flight, a member of each team scoring the other team, stable ordering,
duplicate prevention, direct legacy fallback, a snapshot-backed team split
across flights, cross-flight save/confirm denial without mutation, cross-round
rejection, privileged roles, viewer/unlinked denial, and session/membership/link
revocation. Owner review found no correctness or security defect after those
cases were added.

All 193 PostgreSQL-backed tests passed, including the new authorization suite and
the seven existing scorecard regressions. The 64 standard backend tests, strict
all-feature Clippy, Rust formatting, 141 tests across 24 frontend files, strict
TypeScript, ESLint, the production build, and `git diff --check` also passed.

Real Chrome at 375x812 showed all four round cards while returning exactly the
two same-flight teams as writable with `Cache-Control: private, no-store`.
Anders switched to peer Lag 2, saved par, reached `Synkronisert`, received the
authoritative card refetch, and revisited the persisted score. At 1440x900 a
direct cross-flight Lag 3 card remained read-only with every score control
disabled. Both layouts had no horizontal overflow, visual defects, unexpected
request failures, console errors, or runtime exceptions.

## Next boundary

Phase 6 begins with splitting format-sensitive lifecycle, scorecard, and
leaderboard modules before adding two-player foursomes. Faster phone switching
among several writable flight cards belongs to that format-aware scoring work.
