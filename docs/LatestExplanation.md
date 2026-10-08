# Clarify Fantasy early finishes and golfer standings

This iteration updates the Fantasy plan only. Fantasy is not implemented;
application behavior, schema 34, player claims and deployment remain unchanged.

A match that legitimately finishes early retains the points earned when it ends.
Unplayed remaining holes add no points or penalties and do not stay pending or
block settlement once the match facts are final. This is normal match completion,
not DNF. For example, seven earned hole points after a match ends on hole 14 stay
seven; holes 15-18 contribute nothing. Match placement, net allocation and
concession scoring remain separate decisions before implementation.

The plan now includes a golfer points leaderboard alongside Fantasy manager
standings. It shows every tournament golfer's base Fantasy points by round and
across all rounds, including golfers nobody selected. Placement and hole-point
breakdowns explain each total. Captain multipliers apply only to managers:
a golfer with 12 points still shows 12 even when a manager receives 24 for
captaining them. A second round of -2 makes that golfer's overall total 10.
Team partners retain the same shared round result, with separate golfer rows.
The existing privacy, provisional-result and historical-retention rules apply
to both leaderboards.

The [planned contract](ARCHITECTURE.md#fantasy-competition-design-planned-not-implemented)
records these rules and acceptance examples. [PLANS.md](PLANS.md) includes golfer
projections, UI views and release checks in the existing bounded sequence.
The early-finish decision is resolved; match placement/net/concession treatment,
authoritative DNF settlement and invalid/missed lineups remain contract gates.

Validation passed: two added local links/anchors, three arithmetic examples,
four-document scope and `git diff --check`. Independent read-only review found
no actionable issues. No runtime checks are required or claimed because no
application or migration files changed.
