# Latest explanation

## The plan is a queue, not a history

`docs/PLANS.md` had accumulated a long release recap and a second copy of durable
product decisions. That made it harder to see whether work was actually active
and created multiple places that could disagree.

The repository now assigns one purpose to each document:

- `PLANS.md` contains one active step, one next candidate, and a short later queue.
- `ARCHITECTURE.md` contains durable boundaries, data ownership, invariants, and
  the implemented API inventory.
- `Documentation.md` describes current behavior and operator-facing contracts.
- `LatestExplanation.md` records the rationale and evidence for the latest step.
- `deployment_guide.md` is reserved for the future production runbook.

A plan step stays compact and testable:

```markdown
## Active step: short name

- Goal: the outcome
- Scope: owned files or modules
- Behavior: the exact change
- Validation: commands and evidence
- Invariants: facts that must remain true
- Stop condition: the measurable boundary
```

When the step is complete, that section is removed. Git retains chronology; the
durable documents retain the current truth.

## Durable product facts remain explicit

The cleanup does not discard the latest behavior. Architecture and Documentation
now explicitly retain both relevant contracts: final holes 10–18 are controlled
by an exact-admin toggle with no time dependency, and manual course setup creates
one immutable round-specific course/tee revision. Round opening calculates and
freezes Course and Playing Handicap, and net scoring allocates received strokes
through the preserved hole stroke indexes. A reusable multi-tee course library
remains a separate decision.

## Agent guidance is responsibility-based

The root `AGENTS.md` owns repository-wide scope, invariants, completion, and
publication policy. `docs/AGENT_WORKFLOW.md` owns role routing, the execution
loop, documentation ownership, and validation commands. Nested instructions own
backend, frontend, and migration rules, while `.codex/agents/` defines the
permissions and handoff contract for each specialist.

This removes repeated framework and milestone wording without weakening the plan
gate, sequential-writer rule, read-only review, validation requirements, or golf
domain invariants. The current frontend ladder includes its existing Vitest
suite, and database examples apply the disposable target explicitly to tests,
migrations, and seed commands.

## Validation

This documentation-only iteration required no runtime rebuild. All six repository
TOML files parsed, the five configured role descriptions matched their role
definitions, and every local Markdown link across the eight repository and docs
files resolved. Searches found no stale planned-flow, blackout, phase-specific
agent, or obsolete test-harness wording, and `git diff --check` passed.

Two read-only discovery audits confirmed that the decisions removed from the
plan have durable homes. A final reviewer found one stale README seed-format
description; it was corrected to identify rounds one and two as scramble and
round four as foursomes. The reviewer reported no remaining findings.
