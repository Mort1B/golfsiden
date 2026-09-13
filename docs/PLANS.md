# Plans

`PLANS.md` contains only current and queued work. Completed behavior belongs in
`Documentation.md`; durable technical decisions belong in `ARCHITECTURE.md`.

## Active step

None. Select and bound the next implementation step before changing runtime code.

## Next candidate

**Stableford pure domain foundation:** Implement the approved numeric/no-score
points calculation, resolved progress and typed native points versus overall
comparison values, with focused acceptance tests. Preserve existing calculations
and keep Stableford unavailable. Bound the files, validation and stop condition
when starting; do not combine this with persistence, UI or match play.
See the [Stableford contract](ARCHITECTURE.md#planned-individual-stableford-contract).

## Later, as separate bounded steps

1. **Implement approved format contracts:** One bounded step at a time. After the
   Stableford foundation, the [match-play foundation](ARCHITECTURE.md#planned-singles-match-play-contract)
   is the next candidate. Scope later shared input/result boundaries and each
   format's persistence, lifecycle, result, offline and UI integration explicitly.
   Keep formats unavailable until their complete paths are ready; finish the
   ordinary review/tests/validation for each step before publication.
2. **Code review:** Review the resulting application across the completed formats,
   including the existing generic stroke allocator's `i32::MIN` edge (unreachable
   through the new four-ball i16 snapshot boundary).
3. **Performance work:** Measure representative workloads and scope changes from
   the findings.
4. **Security review:** Review the resulting application and address findings in
   separately bounded steps.
