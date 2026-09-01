# Latest explanation

## Tournament targets are verified before frontend caching

User-owned query keys already separated sessions and resource IDs, but several
frontend decoders trusted the server's embedded tournament or round identity.
That meant a mismatched response could be cached under the requested target even
though backend authorization was correct. Tournament-local React state could also
survive an A-to-B route change because React Router reused the page component.

The API boundary now validates every relevant response against the requested
tournament, round, player, owner, metric, invitation predecessor, and course-
configuration target. Collections reject duplicate and internally incoherent
identities. The former mixed tournament API module was split so decoding remains
a focused responsibility, and the unchecked team assertion became a runtime
decoder:

```ts
const round = decodeExpectedRound(value, roundId)
if (round.tournament_id !== tournamentId) {
  invalidData('rundedata', 'round.tournament_id identity')
}
```

This applies to tournament detail and settings mutations, rosters, round lists
and detail, teams, pairings, scorecards, round/tournament leaderboards, course
configuration, and invitation administration. Invitation issue/rotation tokens
are returned only after exact tournament validation; rotations additionally bind
the returned predecessor to the requested invitation.

## Transient state belongs to one route target

Tournament, management, round, leaderboard, and invitation pages now use keyed
workspace components. Navigating within the SPA therefore remounts target-local
handicap and counted-round drafts, mutation errors/receipts, pending invitation
actions, and revealed one-time secrets. A request started in tournament A may
finish after navigation, but its token cannot render in tournament B or reappear
when returning to A.

Query ownership and authentication behavior remain unchanged: private keys are
still rooted by session user, same-user background refreshes stay mounted, and
SSE carries only invalidation signals for the exact live target. Client checks
reject invalid data but never grant authority; the backend remains authoritative.

## Validation

Focused and full frontend tests, strict typecheck, lint, production build, the
Rust baseline, strict Clippy, and the full PostgreSQL suite passed. Chrome at
375x812 and 1440x1000 used the same account independently registered in two
tournaments and verified distinct rosters, handicaps, rounds, teams, cards,
leaderboards, live targets, reset drafts, and delayed invitation completion.
Loading, empty, populated, deliberate error, long-content, and keyboard-focus
states were exercised without cross-target rendering or normal-run console and
network failures.

No backend, schema, or scoring behavior changed. The next bounded product step is
the optional mandatory counted round within best-N tournament standings.
