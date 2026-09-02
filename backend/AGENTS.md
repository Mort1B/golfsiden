# Backend Agent Rules

These rules apply to `backend/` in addition to the repository contract.

## Boundaries

- `api/` owns HTTP extraction, request validation, status codes, and response
  mapping. Handlers must not contain SQL or scoring formulas.
- `repositories/` owns SQLx queries and transaction mechanics. Repositories must
  not depend on Axum response types.
- `domain/` owns pure tournament, handicap, scoring, standings, and lifecycle
  rules. Domain services must not depend on HTTP or database connections.
- `config.rs` owns environment configuration, `error.rs` owns shared API error
  mapping, and `main.rs` only wires the application and shutdown behavior.
- Keep transport DTOs separate from persistent models when their contracts
  diverge. Do not expose database-only columns accidentally.

## Rust mandates

- Use idiomatic `Result` and `?` propagation with specific error types.
- Do not use `unwrap`, `expect`, indexing that can panic, `panic!`, `todo!`, or
  `unreachable!` in request, repository, scoring, or lifecycle paths. A startup
  assertion is allowed only for a true process invariant with a clear message.
- Minimize `clone`, `Arc`, and locks. Every shared mutable value needs a stated
  ownership and concurrency reason.
- Never hold a synchronous lock or database transaction across unrelated async
  work. Never use blocking filesystem, network, sleep, or CPU-heavy work on a
  Tokio request task.
- Make ordering deterministic. Leaderboard ties, team display order, and list
  endpoints require explicit stable ordering.
- Use newtypes or enums when they prevent invalid states. Do not replace domain
  enums with unconstrained strings inside core logic.
- Handicap and scoring formulas stay configurable behind focused domain
  interfaces. Do not hardcode formulas in handlers or SQL queries.
- Preserve timestamps and last-write-wins behavior deliberately. Score mutation
  code must also preserve auditability and locked-round enforcement.

## SQLx and API rules

- Use bind parameters for all values. Never interpolate request data into SQL.
- Multi-write operations that must succeed together use an explicit transaction.
- Map unique, foreign-key, and check failures to consistent API errors without
  leaking confidential database details.
- Validate at the HTTP boundary and enforce critical integrity again in the
  database.
- Collection endpoints must define ordering and avoid accidental N+1 queries.
  If an N+1 query is intentionally acceptable for a bounded small collection,
  document the bound and revisit before the collection can grow.
- API errors keep the established shape:
  `{ "error": { "code": "...", "message": "..." } }`.
- Mutations that affect visible data publish an SSE invalidation event only after
  successful commit.

## Tests and validation

- Pure scoring, handicap, tie, and lifecycle behavior uses fast unit tests.
- Database constraints, transactions, migrations, locking, and repository
  behavior use PostgreSQL integration tests in `backend/tests/`.
- API tests assert status, JSON shape, validation errors, and persisted effects.
- Every regression fix begins with or includes a test that fails for the original
  behavior.
- Run the backend and database ladders from `docs/AGENT_WORKFLOW.md`. Use
  `cargo fmt` to make formatting changes; do not hand-format around rustfmt.
