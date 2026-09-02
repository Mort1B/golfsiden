# Repository instructions

These instructions apply to the whole repository. More specific `AGENTS.md`
files under `backend/`, `frontend/`, and `migrations/` add layer-specific rules
and take precedence when they are stricter.

## Product invariants

- A tournament player is independent of team membership.
- A team belongs to exactly one round, and its composition may change each round.
- A player may appear on at most one team in a round.
- Historical results use preserved tournament or round handicap snapshots, not a
  player's current handicap.
- A score belongs to either one player or one team, never both or neither.
- Locked rounds reject ordinary score mutations. Corrections require an explicit,
  auditable administrator path.
- Individual tournament standings aggregate player results across rounds even
  when those results came from different teams.
- Gross and net results are separate views of the same preserved score data.

These invariants apply to handlers, repositories, migrations, seed data, tests,
and frontend assumptions.

## Task intake and scope

- Read `docs/PLANS.md`, `git status --short --branch`, this file, and every
  applicable nested `AGENTS.md` before meaningful work.
- `docs/PLANS.md` contains the single active implementation step and a short work
  queue. A concrete user implementation request authorizes the primary agent to
  write that bounded step; an ambiguous request remains planning-only.
- Work on exactly one implementation step at a time. The step must define its
  goal, scope, behavior, validation, invariants, and stop condition.
- Preserve unrelated worktree changes. Report newly discovered work without
  silently expanding the active step.
- Stop after the approved step; do not begin queued work without approval.

The role-routing and execution loop are defined in
`docs/AGENT_WORKFLOW.md`.

## Engineering standards

- Preserve existing behavior unless the active step explicitly changes it.
- Keep business rules, transport mapping, persistence, and UI state in their
  established layers. Consult `docs/ARCHITECTURE.md` before changing a boundary.
- No production source file may exceed 400 lines, excluding comments, blank
  lines, and tests. Split earlier when a file owns unrelated responsibilities.
- Add abstractions only when they enforce a boundary, remove demonstrated
  duplication, or isolate an approved extension point.
- Keep API types, runtime decoding, request/response handling, user-facing states,
  and tests aligned when a contract changes.
- Use typed parsers, serializers, and parameter binding. Do not assemble SQL,
  JSON, dates, or URLs through unsafe string manipulation.
- Never log credentials, session secrets, database URLs, personal data, or
  sensitive score-mutation payloads.

## Agent collaboration

- The primary agent owns scope, plan state, integration, documentation, and Git
  publication.
- Use the repository roles in `.codex/agents/` only when their bounded specialty
  materially helps. Small or tightly coupled work stays with the primary agent.
- Parallel work is read-only only. Never run two write-capable agents at once.
- Give each writer explicit, non-overlapping ownership. Subagents do not edit
  `docs/PLANS.md`, expand scope, commit, or push.
- Review and validation agents report evidence and failures; they do not repair
  source files.

## Validation and completion

- Run focused checks while iterating and the complete affected validation ladder
  from `docs/AGENT_WORKFLOW.md` before completion.
- Validate migrations against PostgreSQL. Validate user-facing changes in a real
  browser at mobile and desktop widths, including the relevant loading, error,
  empty, populated, and long-content states.
- Never claim a check passed unless it ran. Record skipped checks and their exact
  blocker.
- A step is complete only after implementation, tests, validation, review, and
  documentation are resolved or explicitly recorded.

## Documentation and publication

- `docs/PLANS.md` owns only current and queued work.
- `docs/ARCHITECTURE.md` owns durable boundaries and technical decisions.
- `docs/Documentation.md` owns current behavior, contracts, and operator flows.
- `docs/LatestExplanation.md` explains the latest completed iteration.
- `docs/deployment_guide.md` will own deployment and recovery procedures.
- Update every affected document in the same step; do not use the plan as a
  completed-history log.
- The primary agent stages only approved files. Completed validated work is
  committed to `main` and pushed to `origin/main` by default unless the user says
  otherwise.
- Never force-push, rewrite published history, or discard unrelated changes.
  Before reporting publication, verify a clean worktree and
  `HEAD == origin/main`.
