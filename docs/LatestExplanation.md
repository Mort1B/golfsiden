# A broader golf tournament platform

The README now describes Guttas Golf as a mobile-first golf tournament platform.
Its introduction covers single-round and multi-round competitions, independent
tournaments, and shared player accounts. The project name remains Guttas Golf.

A concise feature overview explains tournament management, supported individual
and team formats, administrator-managed teams/flights, courses and preserved
handicaps, mobile scoring, live standings/history, private access and account
recovery. A reader organizing a tournament can now see the project's capabilities
without the previous annual-trip framing.

The former long limitations section mixed implementation history with outdated
claims. It is replaced with current boundaries: supported formats, connected
scoring, membership-private results, unavailable locked-score correction UI and
single-instance deployment. Detailed contracts remain in `Documentation.md` and
operator procedures in `deployment_guide.md`. The README also identifies the
operator recovery command and trusted reset-origin configuration, and aligns the
Clippy example with the repository workflow.

## Validation

Feature claims were checked against current product documentation, implementation
and the work queue. Relative README links and Markdown fences were checked;
`git diff --check` passed. Review confirmed the diff contains only documentation
and preserves the work queue.

Backend, PostgreSQL, frontend and browser validation ladders were not run because
this iteration changes prose only; application behavior, dependencies, schema,
scoring rules, authorization and team management are unchanged.

**Verdict: READY.** Documentation-only scope is complete.
