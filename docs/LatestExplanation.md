# Latest explanation

## Fast flight-card switching

The score page now gives a flight member a phone-first horizontal rail for every
scorecard returned by the server's writable-owner list. Each semantic button
shows the player or team name plus holes scored, readiness, or confirmation, and
marks the selected card explicitly. The unchanged full owner selector remains
available for eligible cards that are read-only for the current session.

A quick switch keeps the current hole, returns to hole entry from the summary,
and replaces browser history. Repeated card changes therefore do not make the
Back button traverse each scoring tap. Long names are ellipsized within 52px
touch targets, the rail scrolls horizontally on narrow screens, and keyboard
focus retains the existing high-contrast outline.

```ts
export function quickOwnerSelection(
  selection: ScoreSelection,
  owner: ScoreOwner,
): ScoreSelection {
  return { ...selection, owner, view: 'hole' }
}
```

## Authoritative cache and navigation boundary

The route still owns canonical tournament, round, tagged owner, hole, and view
parameters. Completion validation supplies owner names and progress; score
access supplies the exact ordered writable set. Their tagged intersection is
rendered without copying server state into a client store.

The selected card uses its existing TanStack Query key. After selection the
route prefetches only the previous and next writable cards, while pointer or
keyboard intent can warm the one card being approached. Prefetch therefore
remains owner-scoped and bounded instead of eagerly loading an entire flight.
SSE continues to carry invalidation only and authoritative refetches update both
scorecards and completion progress.

The existing score coordinator remains the navigation authority. Saving,
verification, a failed mutation awaiting Retry or Discard, and confirmation all
disable both the full selectors and the new rail. Correction remains explicit,
the first changed stroke removes confirmation, audit behavior is unchanged, and
locked rounds retain their existing read-only edit boundary. No backend, API,
query-key, ownership, handicap, or scoring-format contract changed.

## Review and validation

Owner-level review found no correctness, locking, state-sync, accessibility,
strict-TypeScript, SSE, query-key, or file-boundary defect. Its one low-severity
test gap was resolved by extracting and testing the exact same-hole, tagged-owner
route selection.

The final frontend ladder passed 151 tests across 26 files, strict TypeScript,
ESLint, the production build, diff checks, and production file-size checks.

Real Chrome used an isolated database and a flight member with four writable and
four additional read-only cards. At 320px, 375px, 390px, and 1440px it preserved
hole 9 across four rapid switches, kept history length unchanged, avoided a
loading state, and fetched only the current/adjacent or explicitly focused
owner. Semantic 52px buttons, visible keyboard focus, horizontal rail scrolling,
long-name ellipsis, and absence of page overflow passed.

Saving and deliberately forced offline failure disabled the rail and selectors;
Retry synchronized and restored navigation. Confirmation disabled navigation,
the rail showed confirmed state, explicit correction removed confirmation, and
an external score mutation produced an SSE-driven completion refetch and updated
progress. There were no unexpected failed requests, console errors, or browser
exceptions. Locked-round rendering was fixture-limited because the isolated
database had no completed-and-confirmed round safe to lock; the existing locked
mutation/editability path was unchanged and had passed the preceding release's
browser validation.

## Roadmap order

Phase 6 is complete. The next bounded roadmap item is `counted_rounds` and its
draft-only configuration boundary in Phase 7. Additional play modes remain
deferred until the remaining roadmap, optimization, and security review are
finished.
