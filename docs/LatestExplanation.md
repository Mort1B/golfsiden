# Plan a separate all-round Fantasy competition

This iteration is documentation-only. Fantasy is not implemented; application
behavior, schema 34, existing player claims and deployment remain unchanged.

The user-approved design is attached to a tournament but independent of its
sporting results: four freely selected golfers each round, one captain doubling
placement and hole points, no budgets or transfer penalties, net scoring and the
same shared team result for both partners. A separate overall leaderboard sums
every round, with round standings and per-golfer explanations. The initial event
has nine two-person teams; the rules also cover smaller/larger fields.

The latest scoring clarification is explicit: actual ace or net albatross-or-better
+10 (never stacked), eagle +3, birdie +1, par 0, bogey -1, double -2, triple -3,
quad-or-worse and pickup -5. Non-finishers retain recorded hole points without
invented penalties for unplayed holes. Placement follows net rank with
10/8/6/5/4/3/2/1 points for positions 1-8 and zero thereafter. Captain multiplication
includes negative points.

[PLANS.md](PLANS.md) sequences contract closure, pure scoring, persistence and
selection APIs, result projections, the mobile-first UI, and release acceptance.
Each implementation step has ownership, validation and a stop condition. The
[planned architecture contract](ARCHITECTURE.md#fantasy-competition-design-planned-not-implemented)
records rules, proposed defaults, data and authorization boundaries, source seams
and concrete acceptance examples separately from implemented behavior.

Remaining decisions are explicitly gated: match outcome/net/concession treatment,
authoritative non-finish settlement and invalid/missed lineup handling. “Every
round” cannot be satisfied by silently excluding match play. Existing match
reports are not always physical strokes, and current sporting withdrawal is not
a live-round DNF action. These facts must not be papered over with invented scores.

Inspection verified the current net/round ownership, Stableford/four-ball score
states, match provenance, round-opening lock, visibility and frontend invalidation
boundaries. The plan calls for original/frozen handicap use, exactly four unique
picks plus a selected captain, server-enforced locks, pre-deadline secrecy, atomic
receipts and hidden-result noninterference in totals and rank.

Validation passed for document/source consistency, arithmetic examples, six added
local links/anchors, four-document scope and `git diff --check`. Independent
read-only review reported no actionable findings. No Rust,
frontend, database, browser or deployment checks are claimed: no runtime files
changed. Implementation and the complete affected validation ladders remain
future work requiring the next bounded user instruction.
