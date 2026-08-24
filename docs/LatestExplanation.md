# Latest explanation

## Two-player foursomes

The application now supports `two_player_foursomes` end to end as an exact two-
player, team-owned alternate-shot format. One team owns one shared hole score,
and every authenticated member of its flight can keep score for every eligible
card in that flight.

Onboarding, round persistence, pairings, opening readiness, score access,
scorecards, confirmation, completion, round and tournament leaderboards, seed
data, strict frontend decoders, and Norwegian labels all recognize the format.
Individual stroke play and two-player scramble retain their existing behavior.

## Preserved WHS handicap

Foursomes requires a 50% allowance. Opening calculates each partner's unrounded
Course Handicap from the fixed tournament index and immutable tee facts, sums
the two values, applies 50% once, and rounds only the final team result. Exact
half values follow WHS allowance rounding, including moving stored-negative plus
handicaps toward zero.

```rust
let combined = first_unrounded.checked_add(second_unrounded)?;
let allowed = combined.checked_mul(50)?;
let playing_handicap = whs_allowance_round(allowed, 1_130 * 100)?;
```

The result is inserted into `round_team_handicap_snapshots` in the same opening
transaction as the member snapshots and before the round status changes. The
table permits capture only through that exact foursomes opening context, requires
two snapshotted members, and is immutable afterward. Scorecards and leaderboards
read this preserved value rather than recalculating historical results.

## Database and format boundaries

Forward migration `0013` appends the enum value, enforces allowance 50, adds the
team snapshot table, and replaces score, confirmation, completion, and lifecycle
functions so foursomes is explicitly team-owned. Existing migrations are
unchanged, and an upgrade regression starts from schema 0012 and exercises an
existing scramble scorecard through the replaced functions.

The closed Rust and TypeScript format policies map all three formats explicitly.
Foursomes uses exact two-player teams wholly contained in one flight. Completion
requires a complete, confirmed shared card; tournament standings attribute that
round result to both preserved members. Gross and net keep independent
competition-position ties with no hidden cross-metric tie-break.

Scorecard handicap loading and pairing-draft validation/serialization were split
into focused modules before adding the new behavior. Query-key ownership, SSE
invalidation, mutation synchronization, locks, audit actors, and error precedence
remain unchanged.

## Review and validation

Owner-level review found four low-severity test or robustness gaps. Checked
handicap arithmetic, stable seed timestamps, separate scramble/foursomes parity
coverage, and post-upgrade trigger coverage resolve them.

The final ladder passed 75 standard Rust tests, strict all-feature Clippy and
formatting, and 210 combined unit/PostgreSQL tests. A fresh database passed
migrate, seed, a second idempotent seed, and migrate again. The frontend passed
148 tests, strict TypeScript, ESLint, and the production build.

Real Chrome at 375px and 1440px validated onboarding, populated pairings,
opening, four preserved team snapshots, flight-wide team selection, synchronized
hole scoring, locked controls, and gross/net leaderboard labels. There were no
failed requests, uncaught exceptions, unexpected console errors, focus failures,
or layout overflow. Artificially forced generic error/empty states were skipped
because those shared components and contracts were unchanged.

## Roadmap order

The next implementation boundary is the phone score-selector optimization.
Additional play modes such as four-ball, Stableford, and match play are explicitly
deferred until the remaining roadmap, performance optimization, and a dedicated
security review are complete.
