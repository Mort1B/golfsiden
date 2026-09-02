# Latest explanation

## Final visibility is an admin decision

Phase 7C replaces the 24-hour final-score clock with one tournament-owned
setting. A newly created tournament hides holes 10–18 by default. The exact
tournament admin can release or re-hide them from the management workspace even
after the final is completed or locked; confirmation and elapsed time no longer
affect the decision.

The focused API uses an absolute desired state and its own optimistic timestamp:

```json
{
  "back_nine_hidden": false,
  "expected_visibility_updated_at": "2026-09-02T12:00:00Z"
}
```

The repository revalidates the active session and exact tournament-admin
membership inside the write transaction. PostgreSQL independently guards the two
visibility columns and advances the timestamp. Changed commits publish a
dedicated identifier-free `visibility` event; idempotent requests do not.

## Every protected read uses the same policy

The pure visibility rule consumes the caller's exact tournament role, final-round
identity, round state, hole count, and persisted toggle. Exact admins and the
separately authorized scoring projection remain full. For every other role, a
hidden open final is recomputed from holes 1–9, and a hidden completed or locked
final is omitted before tournament best-N selection. Actor-free scorecards,
completion readiness, player history, and direct result cards receive the same
redaction.

Migration 0018 preserves already-released completed/locked finals during upgrade,
keeps other tournaments hidden, and removes the former deadline column and
confirmation triggers. Course revisions, round lifecycle, confirmations, scores,
and immutable handicap snapshots are unchanged.

## Tightening visibility fails closed in the browser

A normal query invalidation can continue rendering old data while refetching.
That is unsafe when an admin re-hides results, so the dedicated visibility event
cancels in-flight protected reads and synchronously clears role-projected
leaderboard, completion, history, drilldown, and actor-free scorecard query state.
An EventSource error performs the same transition immediately without refetching
while offline; reconnect `open` clears again and then loads authoritative state.
Writable `/scoring` queries stay separate and are never cleared by this policy.

The manual course path remains the existing round-specific immutable revision.
Admins provide tee rating, slope, hole pars, and stroke indexes; opening still
calculates and freezes Course/Playing Handicap in the backend and allocates
received strokes by the preserved handicap and hole index.
