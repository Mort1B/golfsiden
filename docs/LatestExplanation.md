# Latest explanation

## Agent operating model

The repository now separates durable engineering rules from task-specific agent
instructions. `AGENTS.md` defines shared invariants, plan gating, file limits,
validation, documentation, and publication rules. Nested instruction files make
the Rust, React/TypeScript, and PostgreSQL requirements stricter where they
apply.

Five project agents cover the useful specialization boundaries without allowing
parallel source edits. Exploration and review are read-only. Backend and frontend
engineers can write only one approved step at a time. The validator has workspace
write access solely because compilers and browser tools create artifacts; its
instructions prohibit source changes. The primary agent remains responsible for
integration, documentation, commits, and pushes.

This structure keeps strict TypeScript/React rules equal to the Rust rules:
frontend work cannot suppress type errors, bypass the typed API layer, duplicate
TanStack Query state, misuse effects, omit async states, or ship without mobile,
desktop, accessibility, type, lint, and build validation.

## Compact example

Project agents are registered in `.codex/config.toml` and load a focused TOML
instruction layer:

```toml
[agents.frontend_engineer]
description = "Strict TypeScript and mobile-first React implementation specialist."
config_file = "agents/frontend_engineer.toml"
```

No model is pinned in the project configuration. Each agent inherits the active
session model unless the caller explicitly selects another one.

## Validation

- Custom agent fields and config references are checked against the current Codex
  project-agent format.
- All instruction and documentation links are repository-relative.
- No production application behavior was changed by this iteration.
