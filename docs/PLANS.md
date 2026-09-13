# Plans

`PLANS.md` contains only current and queued work. Completed behavior belongs in
`Documentation.md`; durable technical decisions belong in `ARCHITECTURE.md`.

## Active step

None. Select and bound the next implementation step before changing runtime code.

## Next candidate

**Match-play pure domain foundation:** Implement the approved relative handicap
allocation, typed ordered outcomes, early-finish/draw derivation and exact match
point arithmetic with focused acceptance tests. Keep match play unavailable and
preserve existing result pipelines. Bound the files, validation and stop condition
when starting; do not combine this with persistence, reporting authority or UI.
See the [match-play contract](ARCHITECTURE.md#planned-singles-match-play-contract).

## Later, as separate bounded steps

1. **Integrate approved format contracts:** After the match-play foundation,
   scope the shared result boundaries and each format's persistence, lifecycle,
   result, offline and UI integration explicitly, one bounded step at a time.
   Keep formats unavailable until their complete paths are ready; finish the
   ordinary review/tests/validation for each step before publication.
2. **Code review:** Review the resulting application across the completed formats,
   including the existing generic stroke allocator's `i32::MIN` edge (unreachable
   through the new foundations' i16 snapshot boundaries).
3. **Performance work:** Measure representative workloads and scope changes from
   the findings.
4. **Security review:** Review the resulting application and address findings in
   separately bounded steps.
