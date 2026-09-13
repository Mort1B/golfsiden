# Plans

`PLANS.md` contains only current and queued work. Completed behavior belongs in
`Documentation.md`; durable technical decisions belong in `ARCHITECTURE.md`.

## Active step

None. The next definition or implementation step requires user approval.

## Next candidate

### Define the match-play contract

Specify the supported individual/team variants, opponent setup, handicap
treatment, hole and match outcomes, concessions, early finishes, ties and
whether/how matches contribute to overall tournament standings. Check
authoritative rules and concrete examples, preserve the root invariants and
resolve product choices before preparing a bounded implementation step. This
is a definition step; do not implement it yet.

## Later, as separate bounded steps

1. **Implement approved format contracts:** One bounded step at a time, with each
   step's ordinary review, tests and validation completed before publication.
   First candidates are the pure domain foundations described in the
   [four-ball contract](ARCHITECTURE.md#planned-four-ball-stroke-play-contract) and
   [Stableford contract](ARCHITECTURE.md#planned-individual-stableford-contract).
   Keep each format unavailable until its complete persistence, lifecycle, result,
   offline and UI paths are ready. Scope subsequent slices explicitly after the
   definitions, including the shared no-score and result-kind boundaries.
2. **Code review:** Review the resulting application across the completed formats.
3. **Performance work:** Measure representative workloads and scope changes from
   the findings.
4. **Security review:** Review the resulting application and address findings in
   separately bounded steps.
