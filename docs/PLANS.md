# Plans

`PLANS.md` contains only current and queued work. Completed behavior belongs in
`Documentation.md`; durable technical decisions belong in `ARCHITECTURE.md`.

## Active step

None. Select and bound the next implementation step before changing runtime code.

## Next candidate

**Four-ball pure domain foundation:** Implement the approved 18-hole allowance
and player-hole-to-side gross/net aggregation rules with focused acceptance tests.
Keep four-ball unavailable and preserve existing format calculations. Define the
exact files, validation ladder and stop condition when this candidate is started;
do not combine it with persistence, score entry or the other format foundations.
See the [four-ball contract](ARCHITECTURE.md#planned-four-ball-stroke-play-contract).

## Later, as separate bounded steps

1. **Implement approved format contracts:** One bounded step at a time, with each
   step's ordinary review, tests and validation completed before publication.
   First candidates are the pure domain foundations described in the
   [four-ball contract](ARCHITECTURE.md#planned-four-ball-stroke-play-contract) and
   [Stableford contract](ARCHITECTURE.md#planned-individual-stableford-contract),
   followed by the [match-play foundation](ARCHITECTURE.md#planned-singles-match-play-contract).
   Keep each format unavailable until its complete persistence, lifecycle, result,
   offline and UI paths are ready. Scope subsequent slices explicitly after the
   definitions, including the shared no-score and result-kind boundaries.
2. **Code review:** Review the resulting application across the completed formats.
3. **Performance work:** Measure representative workloads and scope changes from
   the findings.
4. **Security review:** Review the resulting application and address findings in
   separately bounded steps.
