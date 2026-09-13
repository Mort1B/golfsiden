# Plans

`PLANS.md` contains only current and queued work. Completed behavior belongs in
`Documentation.md`; durable technical decisions belong in `ARCHITECTURE.md`.

## Active step

None. Select and bound the next implementation step before changing runtime code.

## Next candidate

**Four-ball playable integration:** Connect the approved foundation through the
input/competition-owner policy, player numeric/no-score storage and revisions,
side-card APIs and confirmation, lifecycle guards, derived round/overall results,
private/public projections, offline delivery and mobile score entry. Preserve
existing formats and credit the derived side result once to each frozen partner.
Before implementation, define the exact ownership, contracts, migrations,
validation and release stop condition. Keep four-ball unavailable until every
required path is coherent; do not combine this with other new formats.
See the [four-ball contract](ARCHITECTURE.md#planned-four-ball-stroke-play-contract).

## Later, as separate bounded steps

1. **Stableford and match-play integration:** After four-ball, scope each approved
   format's remaining shared result boundaries, persistence, lifecycle, results,
   offline and UI paths separately. Keep formats unavailable until complete;
   finish the ordinary review/tests/validation for each step before publication.
2. **Code review:** Review the resulting application across the completed formats,
   including the existing generic stroke allocator's `i32::MIN` edge (unreachable
   through the new foundations' i16 snapshot boundaries).
3. **Performance work:** Measure representative workloads and scope changes from
   the findings.
4. **Security review:** Review the resulting application and address findings in
   separately bounded steps.
