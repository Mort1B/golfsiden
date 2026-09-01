# Latest explanation

## Final-nine confidentiality is enforced at the read boundary

Phase 7B1 applies one backend-owned policy to round standings, tournament
standings, scorecards, and completion progress. It combines exact tournament
role, authoritative final-round identity, round state, hole count, the persisted
deadline, and one PostgreSQL observation timestamp. Exact admins receive full
reads. Other members receive only holes 1–9 while an 18-hole final is open or
while a completed or locked final has a null or future deadline. Equality
reveals a non-open round.

Stored facts are validated in full before projection. Restricted round
standings and scorecards then recompute totals and progress from visible holes,
using the full 18-hole denominator for handicap allocation. Completion progress
counts actual front-nine scores and exposes no authoritative completion,
confirmation, readiness, or hidden-derived issue state. A hidden completed or
locked final is omitted before tournament best-N selection and ranking.

```rust
let restricted = visibility.mode == VisibilityMode::FrontNine;
let visible = holes.iter().filter(|hole| !restricted || hole.hole_number <= 9);
// Totals, progress, positions, and ties are derived only after this boundary.
```

## Reading and scoring are separate capabilities

The existing member scorecard GET is now an actor-free read projection. The new
`/scoring` suffix returns the full mutation-oriented card only after exact admin,
scorer, or flight-owner authorization succeeds. It does not take an exclusive
round lock and rejects locked rounds. Corrupt hole counts, ordering, identities,
stroke-index permutations, or score ownership fail before either projection is
produced.

The React client mirrors that boundary with distinct session/round/owner query
keys. It waits for authoritative score access before selecting `/scoring`, uses
that projection for prefetch and write verification, invalidates both variants
after mutations, and removes full cached data after terminal authorization or
lock transitions. Restricted URLs are canonicalized to the visible hole prefix.

## Trusted expiry and validation

Visibility metadata contains `observed_at` and `hidden_until`. The browser uses
their interval only to schedule one cleaned-up refetch; it never grants access
locally. SSE remains payload-free and triggers the same authoritative reads.

Backend coverage includes admin/scorer/player/viewer roles, individual and team
owners, gross/net projection, open/completed/locked and null/future/equal/expired
deadlines, writable-owner isolation, actor omission, corrupt facts, completion
redaction, and private cache headers. Frontend coverage verifies strict runtime
coherence, cache separation, nullable hidden state, visible-progress labels,
timer behavior, and hidden-hole navigation. Focused backend checks, strict
Clippy, and 196 frontend tests plus typecheck, lint, and build passed. Final
integrated validation is run before publication.
