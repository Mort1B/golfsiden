# Guttas Golf

Mobile-first tournament software for a private annual golf trip. The current milestone provides the database foundation, Axum REST API, deterministic development data, read-only React views, atomic round opening and completion, round locking, and audited individual/scramble scorecard APIs.

## Prerequisites

- Rust 1.85 or newer
- Node.js 20 or newer and npm
- PostgreSQL 15 or newer
- Docker Compose (optional, for the supplied local database)

## Local setup

From the repository root:

```bash
cp .env.example .env
docker compose up -d postgres
cargo run -p golf-api --bin migrate
cargo run -p golf-api --bin seed
```

Start the backend:

```bash
cargo run -p golf-api --bin golf-api
```

The API listens at `http://localhost:3000`. Set `RUN_MIGRATIONS=true` to apply pending migrations automatically during backend startup.

In a second terminal, start the frontend:

```bash
cd frontend
npm install
npm run dev
```

The app is available at `http://localhost:5173`. Vite proxies `/api` to the backend. For a separate deployed API, set `VITE_API_URL` before building the frontend.

## Database commands

Apply migrations:

```bash
cargo run -p golf-api --bin migrate
```

Load or refresh the idempotent development seed:

```bash
cargo run -p golf-api --bin seed
```

The seed creates one admin identity, eight players, one course with 18 holes, a five-round tournament, and different two-player teams for rounds one and two.

## Verification

Run Rust formatting, unit tests, and Clippy:

```bash
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
```

Run PostgreSQL integration tests (the configured role must be allowed to create test databases):

```bash
DATABASE_URL=postgres://golf:golf@localhost:5432/golf \
  cargo test --workspace --all-targets --features database-tests
```

Run frontend checks:

```bash
cd frontend
npm run typecheck
npm run lint
npm run build
```

## Repository map

- `AGENTS.md`: repository-wide engineering and agent operating contract
- `.codex/agents`: project-scoped explorer, implementation, review, and validation agents
- `backend/src/api`: routes, handlers, request validation, SSE
- `backend/src/domain`: API/domain models and pure scoring/handicap services
- `backend/src/repositories`: SQLx database access
- `backend/src/bin`: migration and seed commands
- `backend/tests`: PostgreSQL integrity tests
- `frontend/src/pages`: milestone viewer pages
- `frontend/src/api`: typed API client and resource types
- `migrations`: PostgreSQL schema and integrity triggers
- `docs`: architecture decisions and phased plan

See [Architecture](docs/ARCHITECTURE.md), [Project documentation](docs/Documentation.md), [Active plans](docs/PLANS.md), and [Agent workflow](docs/AGENT_WORKFLOW.md) for the API inventory, domain decisions, current behavior, and next milestone.

## Current limitations

Authentication and authorization, the mobile scoring UI, live leaderboards, course administration, and admin forms are intentionally deferred to later milestone 2 or 3 slices. The user/role, score, audit, locking, and handicap snapshot schema is already present so those features can be added without remodelling the core data.

`npm audit` currently reports a high-severity React Router advisory in its server/RSC action mode. This project uses only client-side routing and no React Server Components or router actions, so the affected path is not exposed; upgrade when the registry publishes a patched version that resolves the overlapping advisories.
