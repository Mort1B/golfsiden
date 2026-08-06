# Project documentation

## Current product state

Milestone 1 provides a Rust/Axum API, PostgreSQL schema and migrations, an
idempotent development seed, and a strict TypeScript React viewer application.
Users can currently browse tournaments, tournament players, rounds, players, and
round-specific teams. Administrative forms, score entry, leaderboards, and
authentication are planned but not implemented.

## Repository structure

- `backend/src/api/`: Axum routes, validation, response mapping, and SSE.
- `backend/src/domain/`: models and pure scoring/handicap behavior.
- `backend/src/repositories/`: SQLx queries and persistence operations.
- `backend/tests/`: PostgreSQL integration tests.
- `frontend/src/api/`: typed frontend API boundary.
- `frontend/src/pages/`: route-level mobile-first views.
- `frontend/src/ui/`: reusable application UI.
- `migrations/`: forward PostgreSQL schema changes.
- `.codex/agents/`: project-specific specialist agent definitions.
- `docs/PLANS.md`: the only active implementation plan.

## Preserved domain behavior

- Tournament players retain identity and accumulated results across changing
  round teams.
- Tournament registration stores an initial handicap, and the schema supports an
  immutable handicap snapshot per player and round.
- Team membership is unique per player and round.
- Scores have exclusive player/team ownership.
- Locked-round score mutations require an explicit admin correction setting.
- Score mutations are auditable in PostgreSQL.
- The initial two-player scramble formula is isolated in the domain layer and
  uses 35% of the lower plus 15% of the higher course handicap.
- SSE messages invalidate client queries; clients refetch authoritative data.

## Development workflow

Follow `README.md` for setup and commands. Agents and contributors must also read
the root and applicable nested `AGENTS.md` files. Meaningful implementation work
is plan-gated through `docs/PLANS.md` and follows the loop in
`docs/AGENT_WORKFLOW.md`.

## Known limitations

- No authentication or authorization enforcement.
- No score mutation or scorecard API.
- No gross/net round or tournament leaderboard API.
- No automatic round handicap snapshot capture yet.
- No admin UI for tournaments, players, courses, rounds, or teams.
- No offline score queue or public leaderboard link.
