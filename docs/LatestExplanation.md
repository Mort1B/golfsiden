# Latest explanation

## Mobile live leaderboards

The result tab now displays backend-calculated gross and net standings for either
a selected round or the whole tournament. Tournament, scope, round, and metric
live in canonical URL parameters, so refreshes and browser history preserve the
view without introducing a client store. A requested round is checked against
the selected tournament before its query can run.

Round rows show live score to par, the selected stroke total, holes played,
confirmation state, and exact scramble membership. Tournament rows show the
selected accumulated total, completed-round count, withdrawn state, and the
current open-round team returned by the backend. Unstarted owners remain visible
and unranked instead of displaying zero as a played result.

The new leaderboard endpoints cross a strict runtime decoder before responses
enter TanStack Query. Malformed identifiers, tagged owners, finite states,
nullability, numeric fields, or response identity fail into a query-owned retry
state. The existing single EventSource remains invalidation-only: score and round
events trigger an authoritative refetch rather than merging event data or
recalculating handicap results in React.

## Compact example

The round query cannot start until URL selection has resolved to a round owned by
the selected tournament:

```tsx
const selectedRound = rounds.find((round) => round.id === requestedRoundId)
  ?? preferredRound(rounds)

useQuery({
  queryKey: leaderboardKeys.round(selectedRound?.id ?? '', metric),
  queryFn: () => api.roundLeaderboard(selectedRound?.id ?? '', metric),
  enabled: scope === 'round' && selectedRound !== undefined,
})
```

## Validation

- `npm ci`, four Vitest tests, strict typecheck, ESLint, production build, and
  diff checks pass.
- Chrome checks pass at 320x720, 390x844, and 1280x900 with no console errors,
  failed requests, horizontal overflow, undersized touch targets, or navigation
  overlap.
- Browser coverage includes canonical defaults, keyboard focus and activation,
  Back navigation, round/tournament and gross/net switching, a real partial
  score, unstarted owners, and SSE-driven refetch.
- Intercepted browser cases cover loading, no tournaments, malformed-response
  retry, cached tournament-refresh failure, and long names at 320px.
- `npm audit` still reports two high-severity records for the documented React
  Router server/RSC advisory; this client-only app does not use the affected
  server action path.
