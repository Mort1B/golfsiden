# Latest explanation

## Mobile draft-round course configuration

Tournament administrators can now configure course facts directly from the
Courses section of the management workspace. Every round keeps a compact summary;
only draft rounds expose an editor, and only one editor is expanded at a time.
Non-draft rounds remain visibly locked.

The provider path searches the local tournament-scoped catalog before any
external request. Search keys include the authenticated user, tournament, and
normalized query. A previous result may remain visible while a new query loads,
but it is labelled stale and cannot be selected. Missing and incomplete entries
show their catalog reason and lead naturally to manual entry. A future usable
entry loads its complete tees and displays rating, slope, optional length, par,
and hole completeness; the save still sends only the provider ID and exact tee
category/name, leaving provider facts under backend ownership.

The manual path accepts 1–36 holes and preserves inactive row values while the
admin changes that count. Course and tee names, tee category, course rating,
slope, every par, and a complete unique stroke-index permutation are required.
Location and distance in yards are optional. Norwegian comma and dot ratings are
normalized to one JSON number, while UTF-8 byte limits and backend ranges are
checked before mutation. Errors are attached only to the affected field and
focus moves to the first invalid control.

## Compact example

The frontend sends blank optional distances as `null` and uses array order as
the hole number:

```ts
await courseApi.configure(round.id, round.updated_at, {
  source: 'manual',
  course_name: 'Drøbak GK',
  location: null,
  tee: {
    category: 'male',
    name: 'Gul',
    course_rating: 71.2,
    slope_rating: 125,
    holes: [{ par: 4, stroke_index: 1, distance: null }],
  },
}, csrfToken)
```

## Coordination and failure behavior

The save coordinator prevents duplicate submissions and always uses the visible
round `updated_at`. Success updates both precise round caches, refetches their
authoritative values, collapses the editor, restores keyboard focus to its
toggle, and announces the preserved course/tee. Stale-round and non-draft
conflicts refresh the round without overwriting entered values. A stale provider
tee refreshes detail, clears only the rejected tee, and preserves the selected
course. Changing a choice clears obsolete mutation messages.

Course catalog and provider-detail queries are private and rooted by user ID.
They are excluded from generic payload-free SSE invalidation, preventing score or
round events from repeatedly consuming provider quota. The authoritative round
queries still refresh after a configuration event.

## Invariants

- Backend membership and CSRF checks remain authoritative; UI gating is only
  presentation.
- Only draft rounds mutate, and optimistic conflicts never auto-overwrite.
- Provider facts stay server-owned; manual and provider paths converge on the
  existing immutable revision transaction.
- Missing yard distance remains `null` and is never synthesized.
- Course configuration does not alter players, teams, handicap snapshots, score
  ownership, lifecycle transitions, or standings.

## Validation

- Focused frontend tests passed 18/18; the full suite passed 128/128 across 22
  files.
- Strict TypeScript, ESLint, the Vite production build, and `git diff --check`
  passed. The build processed 1,807 modules.
- PostgreSQL provider and round-configuration regressions passed 1/1 and 10/10.
- Chrome 151 passed at 375x812 and 1440x900. The consolidated run covered manual
  validation and focus, duplicate stroke indexes, a real one-request manual save
  with 18 null distances, success receipt/collapse, long-content layout,
  provider loading/detail/facts, stale/tee-stale/non-draft conflicts, catalog
  error/retry/empty, SSE exclusion, and keyboard-visible focus. There was no
  horizontal overflow, runtime exception, unexpected failed request, or console
  error; intercepted 409 and 503 responses were the deliberate conflict/error
  cases.
- Browser validation initially exposed a stale local PostgreSQL schema. Applying
  the repository migrations restored the expected revision columns; the same
  real manual save then returned 200 and the consolidated acceptance run passed.

## Next boundary

Phase 5 begins with normalized round flights, one designated scorekeeper per
flight, and draft-only pairing integrity. It must preserve the distinction
between tournament players, round-specific teams, and flight membership.
