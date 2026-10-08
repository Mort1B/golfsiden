# Finalize the Fantasy scoring and lineup contract

FANTASY-1 is complete as a documentation-only contract. No Fantasy runtime,
migration or UI has been implemented. The next bounded step is the pure scoring
foundation, FANTASY-2.

The user's final choice makes match play outcome-only: +3 for a win, +1 for a
draw and -1 for a loss. Captain doubling produces +6/+2/-2. Match holes, aces,
net calculations and placement do not add Fantasy points. Accepted sporting
results supply the outcome, including early finishes and concessions; confirmation
and visibility still govern whether the award is provisional, settled or withheld.
This supersedes the earlier proposal to keep match hole points and removes the
need for additional Fantasy match handicap inputs. Non-match scoring is unchanged.

Missing a new valid submission automatically reuses the previous eligible locked
four-player lineup and its captain. Current valid submissions take precedence.
The copied lineup receives this round's results and partners, with its source and
origin recorded. Reuse is atomic with deadline finalization and can continue
across rounds. If there is no previous lineup or it contains an ineligible golfer,
mark missed/invalid with zero; do not generate replacements or promote a captain.
For example, match outcomes win/draw/loss/win with the loser captained total
3 + 1 - 2 + 3 = 5. Golfer standings still show the base 3/1/-1/3.

The existing Fantasy-only non-finish disposition, preserved scoring inputs,
whole-card source generations, privacy and immutable selection boundaries remain.
The [resolved contract](ARCHITECTURE.md#fantasy-competition-design-planned-not-implemented)
contains the rules and acceptance examples; [PLANS.md](PLANS.md#next-candidate)
records the next step and remaining integration sequence.

Validation passed for four added links/anchors, captain/leaderboard arithmetic,
four-document scope and `git diff --check`. Read-only review identified a missing
match revision boundary for non-finish dispositions; the contract and acceptance
example now cover it, and follow-up review confirmed no remaining findings.
Runtime validation is not applicable because no application or migration files
changed.
