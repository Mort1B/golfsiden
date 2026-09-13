# Guttas Golf

A mobile-first golf tournament platform for organizing competitions, managing
players and rounds, and following live scores and results.

From a single round to a multi-round competition, Guttas Golf brings tournament
setup, invitations, scoring, and results together. Players use one account across
tournaments, while organizers manage each event's courses, teams, flights, and
access independently.

## Features

- **Tournament management:** Create tournaments, plan rounds, choose how many
  results count toward the standings, optionally require a specific round, and
  choose shared places or a final-round comparison for equal tournament totals.
  Manage each tournament from draft through completion and archive.
- **Individual and team formats:** Play individual stroke play, two-player
  scramble, or two-player foursomes. Organizers assign teams and flights for each
  round, with team membership able to change between rounds.
- **Courses and handicaps:** Select a saved course layout or configure course,
  tee, and hole data. Preserve tournament and round handicap snapshots so later
  profile changes do not alter historical results.
- **Mobile scoring:** Enter and confirm scores for authorized cards, with per-hole
  handicap stroke indicators in the scorecard summary and automatic refresh when
  returning to the app.
- **Live standings and history:** Follow separate gross and net leaderboards,
  inspect a player's counting rounds, and open preserved individual or team
  scorecards from the results.
- **Private tournament access:** Invite players into membership-protected
  workspaces and control when an 18-hole final round's back-nine results become
  visible to members. Scoring permissions and read-only result access remain
  separate.
- **Share live results:** Organizers can create a revocable 30-day link to overall
  gross/net standings. Visitors see player names and permitted results without an
  account; hidden final results stay protected.
- **Player accounts:** Manage profiles and passwords across tournaments. Organizers
  can create private recovery links for ordinary players without email
  integration; administrator accounts are recovered by the site operator.

## Prerequisites

- Rust 1.88 or newer
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

The API listens at `http://localhost:3000`. `RUN_MIGRATIONS=true` is a
development convenience; production uses the explicit migration action and
refuses startup against an incompatible schema.

In a second terminal, start the frontend:

```bash
cd frontend
npm install
npm run dev
```

The app is available at `http://localhost:5173`. Vite proxies `/api` to the
backend, so cookies remain same-origin from the browser's perspective. For a
separate API origin, set `VITE_API_URL` before building the frontend and set the
backend `CORS_ALLOWED_ORIGIN` to the exact frontend origin.

## Production deployment

The repository includes a portable single-host production baseline in
`compose.production.yml`: Caddy terminates HTTPS, serves the actual Vite build,
and proxies same-origin `/api` and SSE traffic to the Rust release binary;
PostgreSQL remains private on an internal network with a persistent volume and a
separate least-privilege runtime role. Production secrets come from an ignored
runtime environment file, secure cookies and a trusted-proxy secret are
mandatory, migrations are explicit, and `/api/health` and `/api/ready` expose
separate liveness and readiness boundaries. Password-recovery links require an
explicitly configured HTTPS `RESET_PASSWORD_ORIGIN`.

Use [the production deployment and recovery guide](docs/deployment_guide.md) for
the exact build, migration, permission, backup, restore, rollback, and launch
procedure. Never reuse `.env.example` credentials or run the development seed in
production.

## Database commands

Apply migrations:

```bash
cargo run -p golf-api --bin migrate
```

Load or refresh the idempotent development seed:

```bash
cargo run -p golf-api --bin seed
```

The seed creates one admin identity, eight linked player accounts, one course
with 18 holes, and a five-round draft tournament whose final round is mandatory
within its best-three standings. It is ready to start. After
the admin starts the tournament in its management workspace, every seeded
round's pairings are ready to open.
Each round has two four-player flights starting on holes 1 and 10. Scramble
rounds one and two and foursomes round four have four two-player score-owner
teams; individual rounds three and five use flights only. The player rotations
change between rounds. Development credentials are:

- Admin username: `admin`
- Player usernames: `anders`, `bjarne`, `christian`, `daniel`, `eirik`,
  `fredrik`, `geir`, and `henrik`
- Shared local password: `golf-dev-2026`

The local `.env.example` explicitly disables the cookie `Secure` flag for HTTP
development. Keep `SESSION_COOKIE_SECURE=true` in HTTPS environments.

Tournament-admin course search uses the bundled local shortlist and consumes no
provider requests. Provider detail for catalog entries verified as usable is
enabled by setting the optional backend-only `GOLF_COURSE_API_KEY`; when absent,
that detail endpoint returns a deliberate unavailable response. Never place this
key in frontend environment variables or browser requests.
`GOLF_COURSE_API_DAILY_LIMIT` defaults to 50 and caps uncached provider calls
per UTC day in each backend process; use the provider plan's limit when changing
it. Multi-instance deployments require a shared quota before relying on this as
a global account-wide ceiling.

### Course configuration

For each draft round, organizers can choose a saved layout, select an available
course and tee from the catalog, or enter the course details manually. Manual
entry includes the tee's course rating and slope, each hole's par and unique
stroke index, and optional distance in yards. Unavailable catalog entries explain
why they cannot be used and offer manual entry as a fallback.

The selected course, tee, and hole facts are saved as an immutable local revision,
so later provider changes do not alter a configured round. Open, completed, and
locked rounds remain read-only for course configuration.

## Verification

Run Rust formatting, unit tests, and Clippy:

```bash
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Run PostgreSQL integration tests (the configured role must be allowed to create test databases):

```bash
DATABASE_URL=postgres://golf:golf@localhost:5432/golf \
  cargo test --workspace --all-targets --features database-tests
```

Run frontend checks:

```bash
cd frontend
npm run test
npm run typecheck
npm run lint
npm run build
```

## Repository map

- `AGENTS.md`: repository-wide engineering and agent operating contract
- `.codex/agents`: repository-local explorer, implementation, review, and validation roles
- `backend/src/api`: routes, handlers, request validation, SSE
- `backend/src/domain`: API/domain models and pure scoring/handicap services
- `backend/src/repositories`: SQLx database access
- `backend/src/bin`: migration, development seed, and operator password-recovery commands
- `backend/tests`: PostgreSQL integrity tests
- `frontend/src/pages`: route-level application pages
- `frontend/src/features`: focused mobile feature components and utilities
- `frontend/src/api`: typed API client and resource types
- `migrations`: PostgreSQL schema and integrity triggers
- `docs`: current behavior, architecture, active work, workflow, and deployment guidance

See [Architecture](docs/ARCHITECTURE.md), [Project documentation](docs/Documentation.md), [Plans](docs/PLANS.md), [Agent workflow](docs/AGENT_WORKFLOW.md), and the [deployment guide](docs/deployment_guide.md) for the API inventory, domain decisions, current behavior, queued work, and production operations guidance.

## Current limitations

- Supported formats are individual stroke play, two-player scramble, and
  two-player foursomes. Four-ball, Stableford, and match play are not implemented.
- Scoring requires a connection; there is no offline score queue or automatic
  replay of writes made while disconnected.
- Tournament workspaces and results require membership. Public leaderboard
  sharing is not available.
- Locked rounds reject ordinary score changes. An administrator interface for
  audited corrections to locked-round scores is not yet available.
- Production deployment targets a single API instance. Rate limits and course
  provider quotas need shared state before horizontal scaling.

See [Project documentation](docs/Documentation.md) for detailed behavior and
[Plans](docs/PLANS.md) for the current work queue.
