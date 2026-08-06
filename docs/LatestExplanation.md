# Latest explanation

## One-hole mobile scoring

The Score tab now opens the newest eligible round and resolves a stable player
or team owner from round completion validation. Tournament, round, tagged owner,
hole, and hole/summary view are canonical URL parameters, so explicit selections
work with browser history while adjacent-hole movement replaces the current
entry. Draft rounds are excluded and locked cards remain readable.

Score input is deliberately separate from cached server state. One coordinator
owns an exact round/owner/hole, writes the first change immediately, coalesces
rapid taps without concurrent requests, and keeps the latest desired gross score
visible. It reports synchronization only after a decoded scorecard refetch agrees
with that value. Failures retain Retry and Discard actions, and route/unload
guards keep unresolved intent from being silently abandoned.

Net strokes, totals, and playing handicaps always come from the backend.
Complete cards can be confirmed, confirmed cards require explicit correction
mode, changed scores remove confirmation, and re-confirmation restores the
correction gate. Completed rounds remain editable and locked rounds are strictly
read-only. Until session authentication is implemented,
`VITE_SCORER_USER_ID` supplies development attribution only and missing or
malformed configuration disables mutations.

## Compact example

The coordinator verifies the authoritative scorecard before publishing the final
sync state and starts one coalesced follow-up only when the desired value moved:

```ts
await dependencies.save(submitted)
const verified = await dependencies.verify()

if (snapshot.desiredValue !== verified) {
  void persist()
} else {
  update({ serverValue: verified, phase: 'synced' })
}
```

## Invariants

- Completion validation, not leaderboard order, defines eligible score owners.
- Client code never calculates handicap or net results.
- Writes are serialized per exact tagged owner and hole.
- Confirmation and correction callbacks retain immutable owner scope.
- SSE remains an invalidation signal; clients refetch authoritative state.

## Validation

- Rust format, 28 unit tests, Clippy with warnings denied, and 35 PostgreSQL/API
  integration tests pass.
- `npm ci`, 19 Vitest tests, strict typecheck, ESLint, and production build pass.
- Real Chrome checks cover 320px, 360px, and desktop layouts; individual and
  scramble owners; rapid coalescing; injected failure/retry; blocked NavLink and
  Back navigation; confirmation, correction, re-confirmation, locking, SSE
  refetch, long content, keyboard focus, and bottom-navigation clearance.
- Browser validation produced no console errors, failed requests, unexpected
  HTTP responses, or horizontal overflow.
- `npm audit` still reports the documented two high-severity React Router
  server/RSC advisory records; the application uses client-side routes only.
