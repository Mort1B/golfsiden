# Guttas Golf

Mobile-first tournament software for a private annual golf trip. The current
milestone includes atomic self-service tournament creation, revocable sessions,
round-specific teams and flights, transactional draft pairing rosters, audited
individual/scramble scorecards, deterministic round lifecycle operations, and
live gross/net leaderboards with URL-backed player history and read-only result
scorecards. Final-round holes 10–18 default to hidden from every non-admin read
projection until the exact tournament admin releases them; the admin may re-hide
them without a time dependency. Authorized scoring views remain available only
for the exact writable card.

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
with 18 holes, and a five-round draft tournament whose final round is mandatory
within its best-three standings. It is ready to start. After
the admin starts the tournament in its management workspace, every seeded
round's pairings are ready to open.
Each round has two four-player flights starting on holes 1 and 10. Scramble
rounds one, two, and four have four two-player score-owner teams; individual
rounds three and five use flights only. The player rotations change between
rounds. Development credentials are:

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
operations. The global player/profile/handicap HTTP directory, its `/players`
page, and legacy platform-admin tournament creation have been retired. Players
are discovered through private tournament rosters, while tournament creation is
available through creator onboarding only. Scorecard reads and live invalidation
streams also require exact tournament membership. Direct arbitrary-player roster
registration is retired; creator onboarding and tournament invitations are the
only supported participation entry points. Direct round creation now resolves
exact tournament-admin authority before request or target validation. The
score-access read now returns `403` when the session lacks exact target
membership, while exact viewers remain authorized with no writable owners. A
two-tournament PostgreSQL fixture proves that a reused global player keeps
separate handicaps, snapshots, flights, player/team cards, results, mutations,
and identifier-free event scope. Each SSE frame carries only a fixed
`invalidate` marker so browser EventSource dispatches it. The frontend rejects
mismatched tournament, round,
player, team, result, invitation, and course-configuration identities before
caching or revealing them, and tournament-keyed workspaces discard drafts,
mutation receipts, pending state, and one-time invitation secrets during SPA
target changes. Tournament admins can atomically choose both best N and an
optional mandatory round before start. That round permanently consumes one of N
slots; a player who misses it cannot replace it with another result.

Tournament creation now issues a reusable invitation secret, and admins can
rotate or revoke links, but recovery from a lost one-time plaintext response
still requires rotation. Request throttling is required before public
deployment. The backend now exposes a private consolidated team/flight roster and
an atomic admin replacement endpoint for draft rounds. Flight-aware validation
and opening require complete assignments and keep scramble teams within one
flight; the development seed supplies representative assignments for all five
rounds, ready to open after the admin starts the tournament. Tournament admins
can edit one draft round at a time with accessible
add, move, remove, and ordering controls; saves replace the complete team/flight
roster atomically and preserve dirty input on conflicts. The remaining
tournament settings, offline scoring, and locked-round score corrections remain
deferred. Every linked tournament player can score all eligible individual or
team cards in their exact round flight; admins and scorers retain their full-
tournament override. Flights are explicit and are never inferred from matching
tee times or starting holes.

Final-round read confidentiality is enforced across round standings, tournament
standings, scorecards, and completion progress. Non-admin tournament standings
omit a completed or locked final round while it is hidden, and member scorecard
reads expose only front-nine-derived facts. The separate `/scoring` scorecard
projection returns a full card only after exact write authorization. The
frontend keeps read and scoring projections in distinct session-owned caches.
Dedicated visibility events, disconnects, and reconnects clear role-projected
facts before authoritative refetch, while writable scoring caches remain separate.

Tournament gross and net standings now include the visible scored portion of the
highest-numbered open round as an explicitly provisional best-N contribution.
Completed-only qualification and mandatory-round eligibility remain separate,
while the UI shows provisional hole progress and refetches authoritative rounds
before validating a live standings update.

Every visible tournament row opens that player's metric-specific contribution
history, and every scored round result or historical contribution opens the exact
preserved player/team card. These drilldowns use only membership-private read
projections; they never acquire scoring or confirmation authority.

Migration `0009` removes account email after deterministically deriving usernames
for existing accounts. Back up production data and retain the generated username
mapping before applying it; player-profile contact email is a separate optional
field and is not used for authentication.

`npm audit` currently reports two high-severity dependency records for one React Router advisory in its server/RSC action mode. This project uses only client-side routing and no React Server Components or router actions, so the affected path is not exposed; upgrade when the registry publishes a patched compatible version.
