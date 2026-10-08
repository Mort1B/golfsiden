# Guttas Golf

A mobile-first golf tournament platform for organizing competitions, managing
players and rounds, and following live scores and results.

From a single round to a multi-round competition, Guttas Golf brings tournament
setup, invitations, scoring, and results together. Players use one account across
tournaments, while organizers manage each event's courses, teams, flights, and
access independently.

## Features

- **Tournament management:** Create tournaments, configure rounds, and edit a
  draft's name, description and date range. Choose how many results count, an
  optional mandatory round, and shared places or a final-round tie-break.
  Start the tournament, then open, complete and lock rounds separately before
  completing and archiving the event.
- **Six scoring formats:** Individual stroke play, two-player scramble,
  two-player foursomes, four-ball, individual Stableford and singles match play.
  Each uses its own scoring and confirmation flow; see the format table below.
- **Round-specific pairings:** Organizers assign teams, flights, starting holes
  and singles opponents manually. The pairing editor checks readiness before a
  round opens. Players belong to the tournament independently of teams, and
  team membership can change between rounds.
- **Courses and handicaps:** Select a saved course layout or configure course,
  tee, and hole data. Preserve tournament and round handicap snapshots so later
  profile changes do not alter historical results.
- **Mobile scoring:** Enter authorized player or team cards with per-hole handicap
  indicators, saved-on-device/server status, and explicit conflict resolution.
  Continue through coverage gaps and return to the last opened card and hole
  while the app and required cached data remain available.
- **Live standings and history:** Follow separate gross and net leaderboards,
  inspect a player's counting rounds, and open preserved individual or team
  scorecards from the results.
- **Tournament Fantasy:** Run an optional private game alongside the golf. Pick
  four golfers and a captain each round, then follow manager and golfer points
  for individual rounds and the whole tournament. See [Fantasy](#fantasy).
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

## Supported formats

| Format | Holes | Scores and results |
| --- | --- | --- |
| Individual stroke play | 9 or 18 | Player scores; separate gross and net standings |
| Two-player scramble | 9 or 18 | One team card; contributions follow the round's preserved team membership |
| Two-player foursomes | 9 or 18 | One team card; contributions follow the round's preserved team membership |
| Four-ball | 18 | Each partner enters scores or pickups; the server selects the team's best gross/net result per hole |
| Individual Stableford | 18 | Native gross/net points, including zero-point pickups; overall standings use labelled `36 − points` equivalents |
| Singles match play | 18 | Manually assigned opponents, gross or net mode, and a separate win/draw/loss table worth 1/½/0 points |

Stableford equivalents are distinct from actual stroke totals. Singles match
points do not contribute to overall gross/net standings. Team composition and
opponents are administrator-managed; there is no automatic team balancing or
bracket generation.

## Organizer and player workflow

1. **Create and invite.** Create a tournament and its rounds, then invite players
   to join with their own accounts.
2. **Prepare the draft.** Choose courses and tees, configure standings, and assign
   teams, flights or singles opponents. Under **Turneringsstyring → Innstillinger
   → Navn og datoer**, edit the tournament details if needed; its date range must
   contain every existing round date.
3. **Start and open.** Start the tournament, then open a ready round from
   **Rundestyring**. Opening freezes the round's handicap and team snapshots.
4. **Score and follow results.** Players use **Score**; members follow
   **Resultater**, gross/net views and private scorecards. Ordinary online return
   refreshes the card and resumes at the first missing hole.
5. **Confirm and finish.** Confirm completed scorecards online. Once the required
   cards are confirmed, the organizer can complete and lock the round, then
   complete and archive the tournament when eligible.

## Fantasy

Fantasy is a separate game within a tournament: it uses the recorded golf results
without changing scores, teams or sporting standings. Any tournament member can
manage a Fantasy entry, including a member who is not playing golf. It supports
all six formats and varying field sizes; nine two-person teams means eighteen
selectable golfers and nine teams competing for placement points.

### Join, select and follow

1. The tournament administrator opens **Fantasy · min firer og poengtavler** and
   selects **Aktiver Fantasy** while every round is still draft and no Fantasy
   deadline has passed. Each interested member selects **Meld meg på Fantasy**.
2. For each round, choose four distinct eligible golfers, select one as
   **Kaptein · doble poeng**, and save. Both partners can be selected, and different
   managers can choose the same golfers. There is no budget or transfer penalty.
3. Picks lock when the round opens or at an earlier administrator-set deadline.
   Without a new valid lineup, the previous eligible locked lineup and captain
   are reused automatically. With no complete eligible fallback, the entry gets
   zero for that round; the app does not choose replacements. Late entrants do
   not receive points for rounds whose selection windows have already closed.
4. Use **Fantasy-lag** for manager standings and **Spillerpoeng** for every
   golfer's base points, whether selected or not. Both boards offer a round view,
   **Sammenlagt · alle runder**, and expandable point breakdowns.

The captain doubles their entire contribution, including negative points. All
rounds count toward Fantasy totals, independently of the golf tournament's
best-round or tie-break settings. Equal totals share rank. Captain multipliers
never change a golfer's own points leaderboard.

### Points

Except in match play, each golfer receives net hole points plus placement points.
Net scores use preserved round handicap calculations, not the player's current
profile handicap.

| Hole outcome | Points |
| --- | ---: |
| Actual hole-in-one, or net albatross or better | +10 |
| Net eagle | +3 |
| Net birdie | +1 |
| Net par | 0 |
| Net bogey | −1 |
| Net double bogey | −2 |
| Net triple bogey | −3 |
| Net quadruple bogey or worse, or explicit pickup | −5 |

Only one category applies per hole. A physical ace earns +10; a net score of one
does not by itself count as an ace. Placement follows the format's net ranking:
positions 1–8 receive **10, 8, 6, 5, 4, 3, 2, 1** points; later positions receive
zero. Tied positions share points and skip the next position (1, 2, 2, 4).
Stableford uses native net points for placing and uncapped net strokes for hole
categories, so a zero-point Stableford hole can still incur a Fantasy penalty.

In scramble, foursomes and four-ball, both partners receive the same shared
hole and placement points. Each team is ranked once before its points are
attributed to the two golfers; changing partners next round preserves history.

**Match play awards only +3 for a win, +1 for a draw and −1 for a loss**, using
the accepted match outcome. There are no hole, ace or placement points, including
when a match finishes early. A captain therefore receives +6, +2 or −2.

Non-finishers in other formats keep recorded hole points, receive no placement
award, and incur no penalty for unplayed holes. An administrator must explicitly
record the non-finish; missing scores alone remain pending.

Selections stay private until lock, even from other administrators. Hidden final
results remain concealed in both boards. Fantasy selection and settlement writes
require connectivity and are not queued offline. See the
[playing and administrator guide](docs/Documentation.md#playing-and-administering-fantasy)
for deadlines, uncertain saves, result states and corrections, and the
[deployment checks](docs/deployment_guide.md#fantasy-setup-and-release-checks)
for enabling Fantasy after an upgrade.

## Scoring with poor connectivity

Open the intended scorecard while connected before entering a coverage gap.
Stroke/team, four-ball and Stableford entries use a durable device queue. Wait
for the saved-on-device state before leaving the card; reconnection delivers
pending entries, with explicit review if another score conflicts.

If you navigate away while the app remains open, **Score → Tilbake til åpnet
scorekort** returns to the same session's last visited writable card and hole
when disconnected or reads fail. The required card data must still be cached.
The returned card awaits server verification; confirmation and switching cards
require fresh online access checks.

Pending edits survive reload for later online delivery to the same account, but
the full scorecard is not stored for offline reopening. Reloading, closing the
app, browser cache eviction or session expiry can require connectivity before
you can continue. There is no offline app shell or background sync. Clearing
browser site data removes pending device edits.

Singles match play queues numeric notes only. Match reports, concessions,
confirmation and corrections require connectivity. See the
[scoring documentation](docs/Documentation.md#mobile-score-entry) for recovery
and conflict handling.

## Prerequisites

- Rust 1.88 or newer
- Node.js 22.12 or newer and npm (Node 20.19+ is also supported by the Vite toolchain)
- PostgreSQL 15 or newer
- Docker Compose (optional, for the supplied local database)

## Local setup

From the repository root:

```bash
cp .env.example .env
docker compose up -d --wait postgres
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
npm ci
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

The current application requires **schema 37** through `0037_fantasy_guards.sql`.
For an existing installation:

1. Record the current release, verify a backup and copy it off-host.
2. Build the chosen immutable release tag and stop API/web for the upgrade.
3. Run the owner migration action, then the runtime `permissions` action.
4. Start matching API/frontend builds and verify `/api/health` and `/api/ready`.
5. Complete the [Fantasy setup and release checks](docs/deployment_guide.md#fantasy-setup-and-release-checks)
   in a separate test tournament with two member accounts.

Use the [exact upgrade commands](docs/deployment_guide.md#upgrade-sequence).
Fantasy needs no additional service, scheduler, secret or environment variable.
Installing it does not enable games or enroll members automatically. On an
installation already at schema 37, the Fantasy UI and acceptance work adds no
further migration. An incompatible-schema rollback requires restoring a verified
backup into a fresh volume; see [Upgrade and rollback](docs/deployment_guide.md#upgrade-and-rollback).

The [local Fantasy acceptance report](docs/validation/fantasy-release-2026-10-08/README.md)
covers all six formats and nine teams over three rounds. The hosted deployment
at gg26.no is operator-managed; local validation does not establish which release
runs there.

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

Use a separate test tournament for hands-on testing: invite a player, prepare and
open a round, enter and edit scores, compare results from another session, then
confirm and lock. Revisit saved scores after reload and a normal service restart.
For coverage-gap testing, follow the offline flow above and verify the same scores
from another online session after reconnection.

Recorded checks and their limits are available for
[core workflows and restart persistence](docs/validation/test-ready-2026-09-29/README.md),
[draft tournament editing](docs/validation/tournament-details-2026-09-29/README.md),
and [offline scorecard return](docs/validation/offline-return-2026-09-29/README.md).
These include local PostgreSQL-backed browser checks at phone and desktop widths;
they do not replace testing the hosted site on physical phones.

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
- `vendor/sqlx-postgres`: local SQLx patch for safe cancellation during transaction startup
- `frontend/src/pages`: route-level application pages
- `frontend/src/features`: focused mobile feature components and utilities
- `frontend/src/api`: typed API client and resource types
- `frontend/e2e`: browser workflow and recovery checks
- `migrations`: PostgreSQL schema and integrity triggers
- `docs`: current behavior, architecture, active work, workflow, and deployment guidance

See [Architecture](docs/ARCHITECTURE.md), [Project documentation](docs/Documentation.md), [Plans](docs/PLANS.md), [Agent workflow](docs/AGENT_WORKFLOW.md), and the [deployment guide](docs/deployment_guide.md) for the API inventory, domain decisions, current behavior, queued work, and production operations guidance.

## Current limitations

- Tournament detail editing is limited to drafts. It does not move round dates,
  restructure the schedule or change started tournaments.
- Singles match play does not include team matches, extra holes, byes or brackets.
- Offline return depends on the same open app/session and retained card data.
  Offline reload, closed-app launch and background sync are not supported.
  Confirmation requires a connection.
- Tournament workspaces and scorecards require membership. Revocable public
  links expose only the limited live overall standings.
- Locked rounds reject ordinary score changes. Singles match play has an audited
  administrator correction and reconfirmation flow while locked; legacy locked
  stroke-score corrections do not yet have an administrator interface.
- Production deployment targets a single API instance. Rate limits and course
  provider quotas need shared state before horizontal scaling.

See [Project documentation](docs/Documentation.md) for detailed behavior and
[Plans](docs/PLANS.md) for the current work queue.
