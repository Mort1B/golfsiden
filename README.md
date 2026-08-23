# Guttas Golf

Mobile-first tournament software for a private annual golf trip. The current
milestone includes atomic self-service tournament creation, revocable sessions,
round-specific teams, audited individual/scramble scorecards, deterministic
round lifecycle operations, and live gross/net leaderboards.

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

The app is available at `http://localhost:5173`. Vite proxies `/api` to the
backend, so cookies remain same-origin from the browser's perspective. For a
separate API origin, set `VITE_API_URL` before building the frontend and set the
backend `CORS_ALLOWED_ORIGIN` to the exact frontend origin.

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
with 18 holes, a five-round tournament, and different two-player teams for
rounds one and two. Development credentials are:

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

### Course data note

The missing shortlist courses are expected to be added directly to
GolfCourseAPI later using a Pro plan. Until a provider course has complete tee
and hole data, the planned round-configuration flow must also support manual
entry by the tournament admin. The admin chooses or names the tee and supplies
its course rating and slope, then enters each hole's par and unique stroke index.
Hole distance in yards is optional. The backend now has one validated transactional
storage boundary for either source. It records the selected tee and complete
hole facts as an immutable local course/tee/hole revision so later round
configuration cannot drift when the provider is edited or unavailable.

The management workspace now exposes one mobile-first editor at a time for each
draft round. Admins can search the local catalog and, once an entry is verified
as usable, select a provider tee; every currently unavailable catalog entry
shows its reason and points to the manual fallback. Manual entry validates the
tee facts and complete hole/stroke-index permutation before saving. Open,
completed, and locked rounds remain read-only.

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
npm run test
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
- `frontend/src/features`: focused mobile feature components and utilities
- `frontend/src/api`: typed API client and resource types
- `migrations`: PostgreSQL schema and integrity triggers
- `docs`: architecture decisions and phased plan

See [Architecture](docs/ARCHITECTURE.md), [Project documentation](docs/Documentation.md), [Active plans](docs/PLANS.md), and [Agent workflow](docs/AGENT_WORKFLOW.md) for the API inventory, domain decisions, current behavior, and next milestone.

## Current limitations

Tournament-scoped membership authorization is enforced for entrant, handicap,
round, team, lifecycle, score-save, score-confirmation, and score-access
operations. Global player/profile mutations and legacy tournament creation are
platform-admin-only. Public read routes remain on the pre-onboarding viewer model
until the private frontend cutover.

Tournament creation now issues a reusable invitation secret, and admins can
rotate or revoke links, but recovery from a lost one-time plaintext response
still requires rotation. Request throttling is required before public
deployment. The remaining tournament settings, pairing, and lifecycle editors,
flights, offline scoring, and locked-round score corrections remain deferred.
Flights will be modelled explicitly rather than inferred from matching tee times
or starting holes.

Migration `0009` removes account email after deterministically deriving usernames
for existing accounts. Back up production data and retain the generated username
mapping before applying it; player-profile contact email is a separate optional
field and is not used for authentication.

`npm audit` currently reports two high-severity dependency records for one React Router advisory in its server/RSC action mode. This project uses only client-side routing and no React Server Components or router actions, so the affected path is not exposed; upgrade when the registry publishes a patched compatible version.
