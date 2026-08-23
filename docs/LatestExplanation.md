# Latest explanation

## Backend course-provider discovery boundary

Tournament administrators can now search GolfCourseAPI and retrieve one
course's tee and hole detail without exposing the provider credential to the
browser. Both GET routes are scoped by tournament ID. The backend first confirms
the authenticated user's exact tournament `admin` membership in a short
transaction, commits it, and only then consults the in-memory cache or performs
external I/O. A global administrator receives no cross-tournament bypass.

The adapter follows the provider's official opaque course-ID and Bearer-auth
contracts but returns a deliberately smaller local shape. Search results include
stable provider identity, names, location, and female/male tee counts. Detail
flattens the provider's tee groups into category-labelled tees, derives one-based
hole numbers from provider order, and names `handicap` as `stroke_index`. It does
not invent a tee ID, persist a revision, or configure a round.

External work is bounded: search text is 2–80 bytes, results stop at 20, only two
cache misses may run concurrently, connections time out after two seconds,
requests after five seconds, and responses above 1 MiB fail closed. An aggregate
256-entry cache keeps searches for 10 minutes and details for 24 hours. Uncached
calls consume a per-process UTC daily allowance, default 50; an upstream `429`
exhausts that local day so later misses do not keep contacting the provider.

## Compact example

Authorization and provider I/O are intentionally separated:

```rust
require_tournament_admin_read(&state.pool, user_id, tournament_id).await?;
let courses = state.course_provider.search(query, fuzzy_match).await?;
```

The authorization helper commits before returning, so the second line never
runs while its PostgreSQL transaction or membership lock is held.

## Invariants

- `GOLF_COURSE_API_KEY` remains optional, backend-only, and redacted from debug
  output; the request header is marked sensitive.
- Provider IDs remain opaque and provider success payloads are validated before
  caching or returning normalized data.
- Missing configuration, saturation, timeout, exhaustion, malformed data,
  upstream failure, and missing courses use stable non-secret error envelopes.
- No migration, course persistence, round mutation, SSE event, or frontend
  behavior was added. Tournament players, round teams, handicap snapshots,
  score ownership, auditability, and locked-round protection are unchanged.
- The cache and UTC quota are per process. Multi-instance deployment requires a
  shared quota before it can enforce the provider account's global ceiling.

## Validation

- Seven focused provider tests pass against local mock HTTP servers, covering
  exact Bearer/query behavior, normalized detail, validation, caching, result
  bounds, timeout, saturation, response size, malformed JSON, status mapping,
  and daily exhaustion without any live provider key or quota use.
- The focused PostgreSQL route test passes and proves signed-out and unauthorized
  requests cause no provider I/O, validation uses the standard error envelope,
  successful data is private/non-cacheable, provider error bodies stay hidden,
  and `404`/`429` mappings remain stable.
- The backend suite passes 48 tests without database features and 132 tests with
  all features: 48 unit tests plus 84 PostgreSQL integration tests.
- Rust formatting, Clippy across all targets/features with warnings denied, and
  `git diff --check` pass.
- Independent review found three provider-boundary gaps: repeated calls after
  `429`, framework query-rejection bodies, and missing route-level error tests.
  All three were fixed and the follow-up review found no remaining findings.
- Frontend and browser checks were not run because this step adds no browser
  route, UI, or frontend contract.
