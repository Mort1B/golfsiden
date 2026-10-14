# Fantasy presets, optional round awards and albatross planning

This iteration is documentation-only. [PLANS.md](PLANS.md) records one next
candidate and two later candidates for the requested changes. No implementation
step is active, and current application behavior is unchanged.

The existing Fantasy API already accepts a saved four and captain for each
future round while its selection window is open. The first candidate makes this
easier to discover and use: review all rounds, copy a lineup into open rounds,
and retain independent overrides. Each save keeps its existing receipt and
recovery rules. For example, a manager can prepare the same four for three rounds
and change only round three's captain. A failed round-three save must not hide
acknowledged saves for rounds one and two.

The user clarified that closest to the pin and longest drive are independently
optional −1 awards to each winner's official net golf result only. Admins may
record neither, either or both after a round; completion and results never
require an award. The proposed combination rule allows one player to win both
for −2 total. Each award has its own identity, audit history and correction or
reversal, so retrying one cannot duplicate it or affect the other. Recorded hole
strokes, gross results and Fantasy points remain unchanged, including Fantasy
placement points. The plan proposes a personal adjustment in team
rounds without changing the partner's result. Stableford's points/equivalent
presentation must be resolved when activating that candidate; match play is
excluded because it has no net stroke result.

Net albatross or better will award 5 Fantasy points, while an actual hole-in-one
keeps 10 with single-category ace precedence. The user chose to apply this only
to games whose selections have not locked yet. Current storage, result metadata
and frontend decoding support only rules version 1, so the plan includes a new
version and coordinated compatibility work. Frozen version-1 games retain their
existing scoring for every round. An already-expired deadline counts as closed
even when its lock has not yet been materialized by a read.

The plan was reviewed against current selection, scoring, versioning and result
boundaries. Local Markdown links/anchors and `git diff --check` were checked.
Runtime, PostgreSQL and browser tests were not run because this planning step
changes only documentation; each candidate lists its implementation validation.
Current-behavior guides remain unchanged until the corresponding implementation.
Hosting remains user-operated.
