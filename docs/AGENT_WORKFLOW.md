# Agent workflow

## Roles

The primary agent is the orchestrator. It owns intake, plan state, delegation,
integration, documentation, and Git publication. Project-scoped subagents live in
`.codex/agents/`:

| Agent | Access | Use |
| --- | --- | --- |
| `project_explorer` | Read-only | Trace code paths and produce one bounded plan step |
| `backend_engineer` | Workspace write | Implement one approved Rust/Axum/SQLx step |
| `frontend_engineer` | Workspace write | Implement one approved React/TypeScript step |
| `reviewer` | Read-only | Review correctness, security, invariants, regressions, and tests |
| `validator` | Workspace write for build artifacts | Run commands and browser checks without source edits |

The primary agent performs small, tightly coupled changes directly when
delegation would add overhead. Agent definitions are tools, not mandatory
ceremony.

## Strict execution loop

1. **Intake**: Read `docs/PLANS.md`, the dirty worktree, applicable instructions,
   architecture, and relevant code. Confirm whether the request is concrete.
2. **Plan**: If there is no precise active step, use `project_explorer` or plan
   locally. Record the goal, files, exact change, validation, invariants, and stop
   condition in `docs/PLANS.md`.
3. **Review the plan**: The primary agent checks scope size, ownership boundaries,
   product invariants, API/schema implications, file sizes, and testability.
4. **Implement once**: Route the step to one write-capable specialist or implement
   locally. Never run backend and frontend writers concurrently.
5. **Review**: Use `reviewer` for risky, cross-layer, security-sensitive, scoring,
   handicap, locking, migration, or state-synchronization work. Resolve findings
   within the same approved scope.
6. **Validate**: Use `validator` or run the targeted ladder locally. Record exact
   commands and results. Validators report failures without repairing them.
7. **Explain and document**: Rewrite `docs/LatestExplanation.md`. Update
   `Documentation.md`, `ARCHITECTURE.md`, README, and API types when their durable
   contracts changed.
8. **Close the plan**: Remove the completed active step. Keep only bounded upcoming
   work in `docs/PLANS.md`.
9. **Publish**: Stage scoped files, commit on `main`, push `origin/main`, and verify
   a clean aligned branch unless the user asked not to publish.
10. **Stop**: Do not begin another upcoming step without explicit approval.

## Delegation rules

- Parallelize independent read-only discovery such as schema inspection, frontend
  code-path tracing, and test-gap review.
- Keep writes sequential. Give every writer an explicit file scope and expected
  result.
- The primary agent re-reads shared files after a subagent returns because all
  agents share the worktree.
- A subagent that finds out-of-scope work reports it; it does not edit the plan or
  implement the extra work.
- Do not delegate simple formatting, one-file documentation, or a tightly coupled
  correction that the primary can complete faster and more safely.
- Stop delegation when evidence is sufficient. More agents are not a substitute
  for a clear decision.

## Step acceptance gates

Every implementation step must satisfy all applicable gates:

- **Scope**: The diff matches the named files/modules and behavior.
- **Structure**: Production files remain below 400 lines and responsibilities are
  cohesive.
- **Domain**: Tournament identity, round teams, handicap snapshots, score
  ownership, locking, audit, and standings invariants remain intact.
- **Backend**: Errors, transactions, async behavior, ordering, and constraints are
  deliberate and tested.
- **Frontend**: Strict types, query ownership, effects, accessibility, mobile
  layout, and complete async states are verified.
- **Database**: Forward migrations and seeds run against PostgreSQL; critical
  invariants have integration coverage.
- **Documentation**: Current behavior and the latest explanation match the code.
- **Publication**: Only approved files are committed, with no unrelated changes.

## Validation ladders

Use focused checks while iterating, then the affected full ladder.

Backend:

```bash
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

PostgreSQL:

```bash
DATABASE_URL=postgres://golf:golf@localhost:5432/golf \
  cargo test --workspace --all-targets --features database-tests
cargo run -p golf-api --bin migrate
cargo run -p golf-api --bin seed
```

Frontend:

```bash
cd frontend
npm ci
npm run typecheck
npm run lint
npm run build
```

Browser validation supplements these commands for any user-facing change. It
does not replace type, lint, unit, integration, or build validation.
