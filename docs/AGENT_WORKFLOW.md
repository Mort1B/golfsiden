# Agent workflow

## Responsibilities

The primary agent is the orchestrator and final owner of intake, plan state,
delegation, integration, documentation, and publication. Repository-scoped roles
live in `.codex/agents/`:

| Role | Access | Responsibility |
| --- | --- | --- |
| `project_explorer` | Read-only | Trace relevant code and define a bounded step |
| `backend_engineer` | Workspace write | Implement one approved backend or persistence slice |
| `frontend_engineer` | Workspace write | Implement one approved frontend slice |
| `reviewer` | Read-only | Find correctness, security, regression, and test gaps |
| `validator` | Build-artifact write only | Run checks and report exact evidence without source edits |

Use a specialist when separation improves speed or review quality. The primary
agent handles small, documentation-only, or tightly coupled changes directly.

## Execution loop

1. **Inspect:** Read the current plan, instructions, worktree, architecture, and
   relevant implementation.
2. **Bound:** Record one active step in `docs/PLANS.md` with goal, scope, exact
   behavior, validation, invariants, and stop condition.
3. **Review the step:** Check ownership boundaries, compatibility, file size,
   migration/API implications, risk, and testability.
4. **Implement:** Assign at most one write-capable specialist at a time, or work
   locally. Keep the diff inside the approved scope.
5. **Review:** Use read-only review for cross-layer, authorization, scoring,
   handicap, lifecycle, migration, concurrency, or cache-synchronization work.
6. **Validate:** Run focused checks, then every applicable full ladder below.
   Validation failures return to the owning implementation layer.
7. **Document:** Update the durable documents and rewrite
   `docs/LatestExplanation.md` for the completed iteration.
8. **Close:** Remove the completed active step from `docs/PLANS.md`; retain only a
   concise next candidate and later queue.
9. **Publish:** Review and stage only scoped files, commit to `main`, push, and
   verify a clean branch aligned with `origin/main` unless publication was
   explicitly excluded.
10. **Stop:** Wait for approval before starting the next queued item.

## Delegation rules

- Parallelize only independent, read-only discovery or review.
- Give one writer explicit file ownership and expected output. Backend and
  frontend writers run sequentially even when their file scopes do not overlap.
- Subagents never edit the plan, broaden the task, commit, or push.
- Re-read shared files after delegated work because every agent shares the same
  worktree.
- Stop delegating when the primary agent has enough evidence to decide.

## Documentation ownership

| File | Content |
| --- | --- |
| `PLANS.md` | One active step, one next candidate, and a short later queue |
| `ARCHITECTURE.md` | Durable boundaries, invariants, data ownership, and API inventory |
| `Documentation.md` | Current product behavior, contracts, setup, and operator workflows |
| `LatestExplanation.md` | Rationale, decisions, validation, and a compact example for the latest iteration |
| `deployment_guide.md` | Deployment, migration, rollback, backup, and recovery procedures |

Completed history does not remain in `PLANS.md`. Git history preserves the
chronology; the durable documents describe the current truth.

## Acceptance gates

Apply only the gates affected by the step, but resolve each applicable one:

- **Scope:** the diff matches the active step and preserves unrelated work.
- **Structure:** responsibilities remain cohesive and production files stay below
  the repository limit.
- **Domain:** the root product invariants remain true.
- **Backend:** errors, transactions, authorization, ordering, and concurrency are
  deliberate and covered.
- **Frontend:** strict types, runtime decoding, query ownership, accessibility,
  mobile layout, and async states are covered.
- **Database:** forward migrations and seed behavior are exercised against
  PostgreSQL where applicable.
- **Documentation:** current behavior and the latest explanation match the code.
- **Publication:** only approved files are committed and the branch is aligned.

## Validation ladders

Use focused checks during implementation, then run the affected full ladder.

Backend:

```bash
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

PostgreSQL, with a disposable approved database:

```bash
DATABASE_URL=postgres://golf:golf@localhost:5432/golf \
  cargo test --workspace --all-targets --features database-tests
DATABASE_URL=postgres://golf:golf@localhost:5432/golf \
  cargo run -p golf-api --bin migrate
DATABASE_URL=postgres://golf:golf@localhost:5432/golf \
  cargo run -p golf-api --bin seed
```

Frontend:

```bash
cd frontend
npm run test
npm run typecheck
npm run lint
npm run build
```

Browser validation supplements these commands for user-facing changes; it does
not replace automated type, lint, unit, integration, or build checks.
