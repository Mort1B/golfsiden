# Latest explanation

## The open round now participates provisionally in best N

Phase 7B2a adds the visible scored portion of the deterministic highest-numbered
open round to tournament gross and net selection. An unstarted card contributes
nothing. Once an owner has a visible score, the preserved player card or exact
round-team card becomes one provisional candidate, attributed once to each frozen
team member where applicable.

The displayed best N is selected independently for gross and net from completed
history plus that one provisional candidate. Mandatory-round reservation and
round-number/UUID cutoff ordering remain deterministic. Every loaded round is
validated in full before the exact current round is rebuilt through its own
role-aware visibility projection, so a different hidden final cannot redact a
non-final open round.

```rust
let qualification = completed_qualification(&candidates, required, mandatory, metric);
select_displayed(&mut candidates, required, mandatory, metric);
// Qualification stays completed-only; displayed selection may include one provisional.
```

## Live selection is not final qualification

`completed_rounds`, `counted_contributions`, and `eligible` remain derived only
from completed or locked contributions. A provisional result may enter the
displayed aggregate and position, but it never qualifies the player or satisfies
a mandatory result. Rows with a selected score rank first by completed
qualification count and then by the requested metric's displayed score-to-par.
Visible provisional hole progress only orders rows inside a sporting tie; it does
not break that tie.

Each contribution now states whether it is provisional and carries visible holes
scored plus the authoritative round hole count. Completed identities remain in
`included_round_ids`; a provisional identity must match `current_round_id`.
During a non-admin final-round blackout, its totals and progress derive only from
holes 1–9 with the existing full-round handicap denominator. Admins retain the
full projection.

## The client validates lifecycle and explains provisional state

The strict frontend boundary validates contribution lifecycle, owner, progress,
mandatory state, completed qualification, metric-specific selection, totals,
eligibility, ranking, ties, and final-nine bounds before caching. The UI separates
“fullførte tellende” from “foreløpig,” shows visible hole progress, and labels an
embargoed mandatory final as awaiting release without implying whether a player
completed it.

SSE still carries invalidation only: a named event includes the fixed
`invalidate` data marker required for browser dispatch, never identifiers or
mutable score state. Before every tournament standings fetch, the client
refetches or awaits the exact rounds query and then validates the new
leaderboard against that authoritative lifecycle state. This prevents an
open/completed transition from pairing fresh standings with stale rounds. The
extra request is a deliberate correctness cost to measure during the later
performance review.

Focused backend tests cover individual and team attribution, unstarted/partial/
full-open cards, unequal qualification, best-N displacement, mandatory open
rounds, independent current-round visibility, and final-nine redaction. Frontend
tests cover strict coherence, lifecycle transitions, release wording, and live
labels. The full backend and frontend ladders pass. In real Chrome at phone width,
a persisted fourth-hole score dispatched the named `score` event, refetched
rounds before standings, changed `3 av 18 hull` to `4 av 18 hull` without reload,
and preserved the final-nine embargo with no console or network errors.
