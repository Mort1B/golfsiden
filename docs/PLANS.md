# Plans

`PLANS.md` contains only current and queued work. Completed behavior belongs in
`Documentation.md`; durable technical decisions belong in `ARCHITECTURE.md`.

## Active step

None. The next implementation step requires user approval.

## Next candidate

### Define the four-ball contract

**Goal:** Define four-ball before implementation and before the later code review,
performance work and security review.

**Scope:** Settle the supported variant, round-specific team setup, score ownership,
per-hole result calculation, gross/net handicap treatment, incomplete holes,
confirmation and ties. Define how results contribute to individual overall
standings, including tournaments that mix formats.

**Behavior and invariants:** This is a definition step, not an implementation.
Keep teams administrator-managed and preserve the root player, round, score-owner
and historical-handicap invariants. Resolve product choices explicitly rather
than assuming every variant shares the existing stroke-play aggregation.

**Validation:** Check the proposed rules against authoritative golf and handicap
sources and work through concrete gross/net, incomplete-card and tied-result
examples. Identify implications for best-N, tournament tie-breaks, score entry,
offline conflicts, confirmation, history and public standings.

**Stop condition:** A reviewed contract records the supported behavior, exclusions,
resolved product decisions and acceptance examples, with one bounded implementation
step ready to approve. Do not implement another format as part of this definition.

## Later, as separate bounded steps

1. **Define Stableford:** Specify the points system, gross/net handicap treatment,
   uncompleted-hole representation, completion, ties and compatibility with mixed-
   format overall standings. Apply the same contract validation and invariants.
2. **Define match play:** Specify the supported individual/team variants, opponent
   setup, handicap treatment, hole and match outcomes, concessions, early finishes,
   ties and whether/how matches contribute to overall tournament standings. Apply
   the same contract validation and invariants.
3. **Implement approved format contracts:** One bounded step at a time, with each
   step's ordinary review, tests and validation completed before publication.
4. **Code review:** Review the resulting application across the completed formats.
5. **Performance work:** Measure representative workloads and scope changes from
   the findings.
6. **Security review:** Review the resulting application and address findings in
   separately bounded steps.
