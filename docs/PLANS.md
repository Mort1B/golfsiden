# Plans

`PLANS.md` contains only current and queued work. Completed behavior belongs in
`Documentation.md`; durable technical decisions belong in `ARCHITECTURE.md`.

## Active step

None. The next definition or implementation step requires user approval.

## Next candidate

### Define the Stableford contract

Specify the points system, gross/net handicap treatment, uncompleted-hole
representation, completion, ties and compatibility with mixed-format overall
standings. Check authoritative rules and concrete scoring examples, preserve the
root invariants, and resolve product choices before preparing a bounded
implementation step. This is a definition step; do not implement it yet.

## Later, as separate bounded steps

1. **Define match play:** Specify the supported individual/team variants, opponent
   setup, handicap treatment, hole and match outcomes, concessions, early finishes,
   ties and whether/how matches contribute to overall tournament standings. Apply
   the same contract validation and invariants.
2. **Implement approved format contracts:** One bounded step at a time, with each
   step's ordinary review, tests and validation completed before publication.
   Four-ball's first candidate is the pure per-player allowance and side-hole
   aggregation foundation described in
   [Architecture](ARCHITECTURE.md#planned-four-ball-stroke-play-contract); keep the
   format unavailable until its complete persistence, lifecycle, result and UI
   paths are ready. Scope subsequent slices explicitly after the definitions.
3. **Code review:** Review the resulting application across the completed formats.
4. **Performance work:** Measure representative workloads and scope changes from
   the findings.
5. **Security review:** Review the resulting application and address findings in
   separately bounded steps.
