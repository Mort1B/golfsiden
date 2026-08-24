# Latest explanation

## Exhaustive round scoring-format policy

The remaining lifecycle and scorecard paths now obtain format semantics from one
closed domain policy. It maps every current `ScoringFormat` to either a player-
owned card with its snapshot-handicap rule or a team-owned card with an exact
team size, snapshot rule, and approved team formula.

This is preparation for a future format, not a new format. PostgreSQL enums and
triggers, API DTOs and errors, frontend contracts, seeds, and visible behavior
remain unchanged.

## Preserved calculations and ownership

The exhaustive mapping is explicit:

```rust
match format {
    ScoringFormat::IndividualStrokePlay => PlayerOwned {
        snapshot_handicap: UncappedIndividualRoundAllowance,
    },
    ScoringFormat::TeamScramble => TeamOwned {
        exact_team_size: 2,
        snapshot_handicap: IndexCappedCourseHandicap { maximum_index_tenths: 360 },
        team_playing_handicap: Scramble35And15,
    },
}
```

Individual play still applies the configured allowance to the unrounded course-
handicap ratio. Scramble still caps each registered index at `36.0` before tee
conversion and calculates the team allowance from 35% of the lower and 15% of
the higher course handicap. Handicap-disabled rounds report zero only after the
stored team has the required members and snapshots, so malformed ownership does
not become a writable scorecard.

Lifecycle readiness, pairing load and replacement, completion-owner discovery,
score authorization, and scorecard validation now consume the same owner/team-
size policy. Every member of a stored flight retains authority to score every
eligible card in that flight; tournament admins and scorers retain their existing
override.

## Pairing write boundary

The former 376-line replacement module is split into a small transaction
orchestrator, validation, and writes. The same transaction still performs exact
admin authorization, draft and optimistic-version checks, identity and roster
validation, legacy conversion and schedule-transfer validation, referenced-team
protection, atomic replacement, authoritative reload, and commit.

Membership ordering, timestamps, legacy facts, update behavior, and error
precedence are preserved. No event behavior changed.

## Validation

Focused policy, handicap, lifecycle, pairing, completion, authorization, and
scorecard coverage passed. That includes 68 standard backend tests and 198
combined unit/PostgreSQL checks; the focused PostgreSQL suites passed with 13
pairing, 12 lifecycle, six completion, two authorization, and eight scorecard
tests. Strict all-feature Clippy, Rust formatting, and `git diff --check` also
passed. Owner-level review found no correctness, security, scoring, locking,
ordering, transaction, migration, API-contract, or frontend issue after its one
test-strength finding was resolved.

No browser or frontend run is required because no client contract, rendered
state, or interaction changed.

## Next boundary

Two-player foursomes remains a decision-gated implementation. Its handicap
formula must be approved before the Rust and PostgreSQL scoring-format enums,
round configuration, snapshots, scoring, leaderboards, seed data, and UI are
expanded together.
