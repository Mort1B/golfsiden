# Latest explanation

## Mobile pairing roster editor

Tournament admins can now configure one round's complete pairing roster from the
management workspace. Each round stays mounted so a collapsed draft is retained,
but only the expanded round fetches its private aggregate. Draft rounds expose
semantic add, remove, move, rename, and ordering controls; open, completed, and
locked rounds show the same authoritative data read-only.

Scramble teams remain round-specific shared-score owners. Flights independently
own grouping, member order, starting hole, and tee time. Individual rounds expose
no team editor. The form may save an incomplete roster, while clearly listing
unassigned entrants and teams that do not yet contain exactly two players;
backend opening readiness remains the authority.

## Full replacement without silent loss

The typed API boundary strictly decodes IDs, finite statuses/formats, timestamps,
nullable schedules, entrants, groups, and ordered members. A save sends complete
desired arrays and the version read from the server:

```json
{
  "expected_round_updated_at": "2026-08-24T10:00:00Z",
  "teams": [{ "id": "uuid", "name": "Lag 1", "members": [], "schedule_flight_id": null }],
  "flights": [{ "id": "uuid", "name": "Flight 1", "starting_hole": 1, "tee_time": "08:30:00", "members": [] }],
  "legacy_conversions": []
}
```

Existing identities and member order are retained, while new groups use browser-
generated UUIDs. Duplicate submission is blocked. On success the returned
aggregate becomes the exact pairing cache and the round detail/list timestamps
are refetched. The local draft records a fingerprint of the complete aggregate,
not only the round timestamp, so entrant-only changes are detected too. A newer
aggregate replaces a clean draft, but a dirty draft is preserved behind an
explicit discard-and-reload conflict state.

Stored inactive members remain visible by name and offer removal-only cleanup;
they cannot be reassigned. Exact server tee-time seconds and fractions survive
unrelated edits even though the time input edits minute precision.

## Explicit legacy normalization

Individual legacy grouping teams block ordinary editing until a separate first
save copies every group exactly into a new flight and submits every required
conversion mapping. For a retained scramble team with an old schedule, the admin
must explicitly select a flight containing the exact same members. That action
copies the team's exact starting hole and lossless tee time; equality never makes
the association automatically.

Changing or clearing a transfer restores the previous target flight's complete
pre-transfer schedule before another target is updated. This prevents an
abandoned selection from leaving an orphaned or duplicated schedule in the full-
replacement request.

## Validation

Review found and resolved schedule-transfer mismatch, tee-time truncation,
inactive-member cleanup, same-timestamp entrant refresh, duplicate heading IDs,
and abandoned-transfer schedule retention. The final review was clean.

All 141 tests across 24 frontend files passed, as did strict TypeScript, ESLint,
the production build, and `git diff --check`. The PostgreSQL pairing API
regression suite passed all 13 tests.

Real Chrome at 375x812 exercised the seeded individual and scramble editors,
independent team/flight assignment, add/remove/rename/reorder, optional schedule
entry and clearing, double-submit prevention, a real successful PUT, authoritative
refetch, and cleanup restoration. The double action emitted exactly one PUT and
the UI returned to its synchronized state. Phone and 1440x900 desktop layouts had
no page/editor overflow, measured action targets were at least 44px, and the final
run had no unexpected HTTP failures, console errors, or uncaught exceptions.
Programmatic focus could not conclusively trigger Chrome's `:focus-visible`
heuristic; the controls remain semantic and their CSS defines a visible keyboard
outline. Stale/non-draft, legacy, scheduled-transfer, and inactive fixtures were
covered by frontend/PostgreSQL automation rather than separate browser fixtures.

## Next boundary

Extend score-access listing and transactional score mutation authorization so
every authenticated exact flight member receives every eligible score owner in
that flight. This editor does not itself grant any scoring permission.
