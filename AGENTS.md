# Repository Agent Contract

This file governs the whole repository. More specific `AGENTS.md` files under
`backend/`, `frontend/`, and `migrations/` add rules for those trees and take
precedence when they are stricter.

## Product invariants

- A tournament player is independent of team membership.
- A team belongs to exactly one round. Team composition may change every round.
- A player may appear on at most one team in a round.
- Historical results use preserved tournament or round handicap snapshots, not
  a player's current handicap.
- A score belongs to either one player or one team, never both or neither.
- Locked rounds reject ordinary score mutations. Corrections require an explicit
  admin correction path and must remain auditable.
- Individual tournament standings aggregate player results across rounds even
  when those results came from different teams.
- Gross and net results are separate views of the same preserved score data.

Do not weaken these invariants in handlers, repositories, migrations, seed data,
tests, or frontend assumptions.

## Source of truth and task intake

- `docs/PLANS.md` is the source of truth for active and upcoming work.
- Read `docs/PLANS.md`, `git status --short --branch`, this file, and every
  applicable nested `AGENTS.md` before meaningful work.
- No implementation begins without one clearly defined active step in
  `docs/PLANS.md`. A direct, concrete user implementation request counts as
  approval to write that bounded step into the plan. Ambiguous requests remain
  planning-only until confirmed.
- Work exactly one implementation step at a time. Each step must state its goal,
  files or modules, exact behavior change, validation, invariants, and stop
  condition.
- Do not expand scope silently. Add newly discovered work to upcoming work and
  stop at the current step's boundary.
- Inspect the dirty worktree before editing. Preserve unrelated user changes.

The detailed role routing and execution loop live in
`docs/AGENT_WORKFLOW.md`.

## Code structure

- No production code file may exceed 400 lines, excluding comments, blank lines,
  and tests. Split before a change reaches the limit.
- Prefer 150-350 lines for complex ownership, async, data-access, or error logic.
- Prefer smaller responsibility-based modules for UI components, hooks, DTOs,
  repositories, and domain services.
- Split a file when it starts owning unrelated concerns, even if it is below the
  hard limit.
- For modules above roughly 150-200 lines of core logic, prefer a named module
  directory with focused sub-files over a growing mixed-purpose file.
- Do not put business logic, transport serialization, database access, and UI
  state management in the same module.
- Do not add abstractions unless they enforce a boundary, remove real
  duplication, or isolate a planned extension point.
- Keep tests out of normal production control flow. Rust private unit tests may
  use `#[cfg(test)] mod tests`; integration tests belong in `backend/tests/`.
  TypeScript tests use dedicated `*.test.ts` or `*.test.tsx` files.

## Change discipline

- Keep diffs milestone-based, cohesive, and reviewable. A step may cross backend,
  migrations, and frontend when the behavior genuinely requires it.
- Preserve behavior unless the approved step explicitly changes semantics.
- Update tests whenever behavior or an invariant changes.
- Update API types, request/response handling, and user-facing states together
  when an API contract changes.
- Prefer forward-compatible domain boundaries over generic frameworks. Implement
  the formats the product uses before generalizing.
- Use structured parsers and serializers. Do not build JSON, SQL, dates, or URLs
  through ad hoc string manipulation when typed APIs are available.
- Comments explain non-obvious constraints or tradeoffs, not syntax.
- Never log credentials, session secrets, database URLs, personal data, or score
  mutation payloads containing sensitive identity data.

## Multi-agent operating model

- The primary agent is the orchestrator and final owner of scope, integration,
  documentation, and Git publication.
- Use `project_explorer` for read-only architecture and code-path discovery.
- Use `backend_engineer` for approved Rust/Axum/SQLx steps.
- Use `frontend_engineer` for approved React/TypeScript steps.
- Use `reviewer` for correctness, security, regression, invariant, and test-gap
  review.
- Use `validator` to run checks and report exact outcomes without editing source.
- Run read-only exploration or review agents in parallel only when their work is
  independent and materially useful.
- Never run two write-capable agents concurrently. Backend and frontend workers
  run sequentially with explicit, non-overlapping file scopes.
- Subagents do not edit `docs/PLANS.md`, publish Git changes, or expand scope.
  They return concise findings, changed files, and exact validation results to
  the primary agent.
- Do not delegate trivial, single-file, or tightly coupled work when coordination
  costs more than it saves.

## Validation and definition of done

Run the smallest relevant checks during iteration and the complete affected
ladder before completion.

Backend baseline:

```bash
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Database rules, when PostgreSQL is available:

```bash
DATABASE_URL=postgres://golf:golf@localhost:5432/golf \
  cargo test --workspace --all-targets --features database-tests
```

Frontend baseline:

```bash
cd frontend
npm run typecheck
npm run lint
npm run build
```

- Validate migrations against PostgreSQL, not only by reading SQL.
- Validate user-facing frontend work in a real browser at a mobile viewport and
  a desktop viewport. Check loading, error, empty, populated, and long-content
  states relevant to the change.
- Do not claim a check passed unless it ran. Record unavailable checks and the
  exact blocker.
- A step is done only when implementation, focused tests, required validation,
  review findings, and documentation are resolved or explicitly recorded.

## Documentation and Git

- Keep `docs/PLANS.md` concise: one active step, bounded upcoming work, and no
  long completed history.
- After a meaningful implementation step, replace
  `docs/LatestExplanation.md` with the latest rationale, important decisions,
  invariants, validation, and one compact code example.
- Update `docs/Documentation.md` when behavior, setup, API contracts, or operator
  workflows change. Update `docs/ARCHITECTURE.md` when boundaries or durable
  design decisions change.
- The primary agent stages only files in the approved scope. Subagents never
  commit or push.
- Completed, validated iterations are committed to `main` and pushed to
  `origin/main` by default unless the user explicitly says not to commit or push.
- Never force-push, rewrite published history, or discard unrelated changes.
- Before reporting publication complete, verify a clean worktree and
  `HEAD == origin/main`.
