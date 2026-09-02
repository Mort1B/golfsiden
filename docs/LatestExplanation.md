# Latest explanation

## Results now have stable read-only destinations

Phase 7B2b turns each visible result into a protected URL instead of transient
leaderboard text. A tournament row opens the selected player's gross or net
history. That history is the existing authoritative tournament leaderboard entry,
so its completed qualification and metric-specific counted selection cannot drift
from the ranking that linked to it.

Every visible contribution keeps the exact owner that produced it. An individual
round therefore links to its player card, while a scramble or foursomes result
links both attributed players to the same preserved round-team card.

```ts
scorecardUrl(
  tournamentId,
  contribution.round_id,
  contribution.owner,
  metric,
)
```

The link deliberately uses `contribution.owner`, never `current_team`, because a
player's team may change each round.

## A drilldown never becomes a scoring surface

The direct-card route composes its target in three stages. It first loads the
tournament's decoded rounds and proves the requested round belongs there. It then
loads the role-projected round leaderboard and requires the exact tagged owner.
Only after those checks does it enable the actor-free member scorecard read.

The route never calls score access, completion validation, `/scoring`, confirmation,
or score mutation endpoints. It still receives the Phase 7B1 visibility projection,
so a non-admin final card contains only visible front-nine facts and refetches at
the trusted release deadline. SSE remains an authoritative invalidation signal.

## Canonical keys and URLs keep identities coherent

History and cards reuse the same session-owned canonical query keys as the main
leaderboard and read-card surfaces. Explicit save/confirmation invalidation, SSE,
logout, and identity replacement therefore target the same facts rather than
leaving route-specific copies behind.

Metric, summary/hole view, and visible hole are canonical URL state. Complete
route identities remount independently, invalid owner types and cross-tournament
rounds fail closed, and initial failures cannot render stale predecessor data.
Backend integration coverage proves both player- and team-owned contribution tags
open cards with matching preserved gross/net totals. Frontend tests cover route
canonicalization, historical owner selection, mandatory/provisional wording,
cache invalidation, and target mismatch behavior. The full backend, serialized
PostgreSQL, and frontend ladders pass. Real Chrome at phone and desktop widths
proved the embargoed individual path, canonical refresh and Back behavior, exact
shared-team URL/card equality for Anders and Henrik after cleared-session logins,
same-user cross-tournament isolation, and fail-closed access for a non-member.
Observed drilldown traffic contained only private/non-cacheable read endpoints;
no scoring-authority, confirmation, or mutation endpoint was requested.
