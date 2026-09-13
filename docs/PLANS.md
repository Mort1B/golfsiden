# Plans

`PLANS.md` contains only current and queued work. Completed behavior belongs in
`Documentation.md`; durable technical decisions belong in `ARCHITECTURE.md`.

## Active step

No active implementation step.

## Next candidate

**Match-play playable integration:** Scope the approved 18-hole singles format,
draws after 18 holes and separate 1/½/0 match-points table without changing
gross/net overall totals. Define opponent setup, reporting/concession authority,
confirmation/finality, corrections, visibility and conditional offline numeric
notes explicitly. Write and review one bounded implementation step before work.

## Later, as separate bounded steps

1. **Code review:** Review the application across completed formats, including the
   generic stroke allocator's `i32::MIN` edge, which is unreachable through the new
   foundations' i16 snapshot boundaries.
2. **Performance work:** Measure representative workloads and scope changes from
   the findings, including the existing frontend bundle-size warning.
3. **Security review:** Review the resulting application and address findings in
   separately bounded steps.
