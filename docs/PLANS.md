# Plans

`PLANS.md` contains only current and queued work. Completed behavior belongs in
`Documentation.md`; durable technical decisions belong in `ARCHITECTURE.md`.

## Active step

No active implementation step.

## Next candidate

**Stableford playable integration:** Scope the approved 18-hole individual format
through snapshots, numeric/pickup storage, conditional offline delivery, confirmation,
native gross/net points, private history and mixed overall standings using 36 minus
points. Preserve typed score units, visibility and existing format contracts. Write
and review one bounded implementation step before beginning work.

## Later, as separate bounded steps

1. **Match-play playable integration:** Implement the approved singles format,
   draws after 18 holes and separate 1/½/0 match-points table without changing
   gross/net overall totals. Scope reporting, authority and confirmation explicitly.
2. **Code review:** Review the application across completed formats, including the
   generic stroke allocator's `i32::MIN` edge, which is unreachable through the new
   foundations' i16 snapshot boundaries.
3. **Performance work:** Measure representative workloads and scope changes from
   the findings, including the existing frontend bundle-size warning.
4. **Security review:** Review the resulting application and address findings in
   separately bounded steps.
