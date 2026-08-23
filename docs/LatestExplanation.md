# Latest explanation

## Atomic draft-round course configuration

Tournament administrators can now configure a draft round through
`PUT /api/rounds/{round_id}/course-configuration`. Both manual entry and a
curated GolfCourseAPI selection end in the same immutable local course, selected
tee, and ordered hole revision.

Every request echoes the round's current `updated_at`. The backend first checks
the authenticated user's exact tournament-admin membership and confirms that
the round is still draft. It then decodes the strict JSON selection. Manual
requests provide the complete facts directly. Provider requests provide only a
curated course ID and tee category/name; the backend refreshes all other facts,
requires one exact trimmed tee match, and passes them through the shared
validator. No database transaction remains open during that provider request.

The final transaction locks the round, revalidates the active session and admin
membership, and repeats both draft-state and timestamp checks. It then creates
the finalized immutable revision and attaches the course/tee IDs, copied names,
and hole count to the round. Commit happens before the single payload-free round
event. A stale or losing request therefore cannot leave an orphan revision.

Requests must use `application/json` and are limited to 32 KiB. Success and
endpoint-shaped errors are `private, no-store`; authentication rejections retain
the shared `no-store` policy. Stable conflicts distinguish a changed round,
non-draft lifecycle state, incomplete catalog row, and a provider tee that
disappeared after selection.

## Compact example

The manual payload uses array order as the hole number and keeps distance
optional:

```json
{
  "expected_round_updated_at": "2026-08-23T10:15:30.123456Z",
  "selection": {
    "source": "manual",
    "course_name": "Example Golf",
    "location": "Oslo, Norway",
    "tee": {
      "category": "male",
      "name": "White",
      "course_rating": 72.4,
      "slope_rating": 128,
      "holes": [
        { "par": 4, "stroke_index": 1, "distance": null }
      ]
    }
  }
}
```

## Invariants

- Only an active admin membership for the round's tournament may configure it.
- Only draft rounds accept configuration, and optimistic concurrency prevents
  silent replacement by a simultaneous save.
- Provider facts are server-fetched; no provider tee ID is invented.
- Manual and provider paths create the same complete immutable revision graph.
- No database transaction spans provider I/O.
- Failed authorization, validation, provider work, attachment, or lifecycle
  races create no orphan revision and publish no configuration event.
- Handicap snapshots, score ownership, teams, lifecycle transitions, and
  leaderboard behavior are unchanged.

## Validation

- Provider adapter tests cover exact/ambiguous tee selection, normalized mapping,
  and fail-closed numeric conversion.
- Ten PostgreSQL tests cover manual and provider-seam facts, nullable distance,
  authentication/CSRF/scope, authorization loss, strict JSON and size bounds,
  catalog gating, stale saves, rollback, simultaneous saves, both configuration/
  opening lock orders, exact SSE counts, and private caching.
- `cargo fmt --all -- --check` and Clippy with warnings denied passed.
- The standard workspace suite passed 62 tests; the database-enabled workspace
  suite passed 166 tests, including all 10 round-configuration cases.
- `git diff --check` passed. Frontend, browser, migration, seed, and API smoke
  checks were not required for this backend-only, migration-free step.

## Next boundary

Build the mobile course and tee picker with the manual-entry fallback on the
tournament management workspace. A full provider-success HTTP test remains
queued until a genuinely complete catalog row is reverified as `usable`; the
production gate is not weakened for test convenience.
