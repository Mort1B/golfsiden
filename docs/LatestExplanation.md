# Latest explanation

## Exhaustive round-leaderboard format boundary

The pure round leaderboard no longer mixes format selection with stored-fact
validation, score assembly, totals, confirmation checks, and ranking. The former
338-line builder is split into a 251-line format-neutral orchestrator and a
focused owner-policy module.

This is a behavior-preserving preparation step. It does not add foursomes,
change `ScoringFormat`, alter PostgreSQL, or change an API response. Its value is
that a later team-owned format cannot silently inherit scramble semantics merely
because it is not individual stroke play.

## Closed owner and handicap policy

One internal policy maps every current Rust format exhaustively:

```rust
match format {
    ScoringFormat::IndividualStrokePlay => IndividualSnapshots,
    ScoringFormat::TeamScramble => TwoPlayerTeam {
        exact_team_size: 2,
        handicap: Scramble35And15,
    },
}
```

Individual entries still come from immutable round snapshots and use the stored
playing handicap. Scramble entries still come from frozen exact-round teams and
members, then apply the existing 35%/15% formula and configured allowance.

Team identity, duplicate assignment, snapshot coverage, and exact team size are
validated before handicap disabling can replace the calculated value with zero.
Malformed stored ownership therefore remains an error even in a scratch round.
No generic framework or unapproved future formula was introduced.

## Format-neutral assembly remains authoritative

After owner seeds are built, the unchanged path validates holes, exclusive
player/team score ownership, duplicate score keys, and confirmations. It then
calculates gross and net totals, par played, completeness, score to par,
deterministic name/UUID ordering, and competition ties. Tournament aggregation
continues attributing a team result exactly once to each frozen round member and
does not branch on scoring format.

The frontend's visible labels are unchanged, but its prior binary fallback is
now an exhaustive `Record<ScoringFormat, string>`. Adding a new typed format will
therefore require an explicit Norwegian label instead of silently displaying it
as individual stroke play. The runtime API decoder continues rejecting unknown
format strings before caching.

## Validation

Focused domain coverage includes both existing formats and a new regression that
proves a one-member scramble team fails closed when handicaps are disabled.
Owner-level review found no correctness, ordering, serialization, or abstraction
issue. All 10 focused leaderboard domain tests and seven PostgreSQL leaderboard
API tests passed. The complete ladder also passed: 65 standard backend tests,
all 194 PostgreSQL-backed/unit checks, strict all-feature Clippy, Rust formatting,
142 tests across 25 frontend files, strict TypeScript, ESLint, the production
build, and `git diff --check`.

No browser run is required because the rendered labels, markup, styling, and
interaction are unchanged.

## Next boundary

Continue the Phase 6 preparation by isolating the lifecycle, pairing, completion,
authorization, and scorecard owner/handicap policies that still use scramble as
a proxy for team ownership. The PostgreSQL enum and trigger branches remain
unchanged until a complete foursomes implementation is approved.
