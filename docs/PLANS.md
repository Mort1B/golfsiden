# Active plans

This file is the source of truth for implementation work. Keep only one active
step. Move durable completed behavior to `Documentation.md` or `ARCHITECTURE.md`
instead of accumulating a historical log here.

## Planning status

The tournament-first architecture below is proposed and awaiting approval. No
implementation step is approved by this planning request.

## Active step

### Phase 1: Tournament-scoped identity and authority foundation

- **Goal:** Replace global administration assumptions with tournament-scoped
  membership and authorization before adding self-service tournament creation.
- **Identity boundary:** Keep `users` as the global credential identity and keep
  a reusable global player profile for the same person across trips.
  `tournament_players` remains the competition identity inside one tournament,
  including that tournament's current handicap and participation state. Add
  tournament-scoped handicap history, and make round opening snapshot the
  tournament handicap rather than the reusable profile default. Team membership
  remains independent and round-specific.
- **Authority model:** Add `tournament_memberships` keyed by tournament and user,
  with `admin`, `scorer`, `player`, and `viewer` roles. Administration and
  participation remain separate: the creator receives an admin membership and
  an active tournament-player entry in the same transaction. New creators never
  receive global admin authority.
- **Migration:** Add a forward-only migration and do not rewrite migrations
  `0001`-`0005`. Explicitly backfill current global admins/scorers onto existing
  tournaments to preserve their existing authority, and backfill linked players
  only for tournaments they entered. Retain `users.role` temporarily for
  compatibility, but stop using it to authorize tournament resources.
- **Backend scope:** Add one tournament-authorization repository/service that
  resolves tournament ownership directly or through a round/team ID. Revalidate
  the active session and tournament membership inside each mutation transaction.
  Protect entrant, round, team, and lifecycle writes with this policy. Global
  account/profile operations remain self-service or platform-only and are not
  granted to tournament admins.
- **Read model:** Add `GET /api/me/tournaments` with the signed-in user's
  tournament role and player linkage. Tournament capability data is
  backend-authoritative; the frontend must not derive permissions from a global
  session role.
- **Privacy:** Replace the eventual global tournament/player directory with
  membership-scoped reads. Public access will require a later explicit share
  token rather than exposing all private trips by default.
- **Tests:** Cover migration backfill, creator-as-admin-and-player independence,
  cross-tournament IDOR attempts through tournament/round/team routes, session
  expiry/revocation races, scorer/player/viewer denial, and unchanged teammate
  score permissions.
- **Validation:** Validate clean and upgraded PostgreSQL databases, Rust format,
  unit/database/API tests, Clippy with warnings denied, and migration/seed
  idempotence.
- **Invariants:** Tournament roles never grant authority in another tournament;
  account email is not a player lookup key; withdrawn players may retain admin
  authority; tournament handicaps and round snapshots remain historical;
  flights and team membership remain separate round concepts.
- **Stop condition:** Tournament membership is the sole authority for tournament
  mutations, migrated data retains intentional access, cross-tournament writes
  fail closed, and all backend validation passes. Stop before creator signup or
  invite redemption.

## Upcoming work

### Phase 2: Atomic creator onboarding

- Add a public, rate-limit-ready `POST /api/onboarding/tournaments` contract with
  nested creator account/player data, tournament basics, and a nonempty round
  list. Do not accept roles, actor IDs, status, round count, or tournament
  scoring mode from the client.
- Hash the password off the async executor, then create the user, player,
  tournament, admin membership, tournament-player entry, initial handicap
  history, every draft round, initial invitation, and session in one database
  transaction. Publish events and set the session cookie only after commit.
- Add the invitation record and token-issuance primitives here: store only a
  hashed 256-bit token with expiry, revocation, and an optional maximum-use
  policy. Return the raw initial link once after commit; redemption follows in
  Phase 3.
- Require contiguous round numbers and one supported `scoring_format` per round.
  Derive `number_of_rounds` and the tournament summary mode from the round list.
  Courses and tees may remain unset while draft, but readiness must still block
  opening until they are valid.
- Replace `/` with a private-trip start screen and add a mobile setup wizard:
  tournament details, round definitions/formats, then creator account/player
  details. One final submission prevents orphan accounts or partial tournaments.
  After success, enter the tournament workspace and show the invite link.

### Phase 3: Reusable invitation onboarding

- Complete the invitation lifecycle with an append-only redemption table. Lock
  the hashed invitation record during redemption; expiry, revocation, optional
  maximum uses, and unique tournament membership make retries and concurrent
  joins safe.
- Use a non-secret invitation ID plus a URL-fragment token so secrets do not
  enter server request logs or referrer headers. Submit the token only in JSON
  bodies and remove it from browser history after capture.
- Provide minimal preview, new-account registration, authenticated acceptance,
  rotation, and revocation endpoints. Existing emails must sign in; never attach
  a player identity by matching submitted email.
- Add `/join/:invitationId` for preview and registration/acceptance. A successful
  join atomically creates or reuses the global identity, creates the
  tournament-player entry and player membership, records redemption, and starts
  a session when required.
- Test expiry, revocation, exhaustion, replay, concurrent final use, duplicate
  membership, duplicate email/account takeover, rollback, and token non-leakage.

### Phase 4: Tournament setup workspace

- Add contextual management routes under
  `/manage/tournaments/:tournamentId` for invitations, entrants, rounds, course/
  tee selection, pairings, and lifecycle actions. Do not restore a global
  `/admin` authority model or add another permanent bottom-navigation item.
- Add draft-only tournament and round update contracts plus course/tee/hole read
  APIs so an administrator can finish readiness after the initial round plan.
- Keep all player and round operations inside the selected tournament in the UI
  and in hierarchical TanStack Query keys. Show loading, empty, validation,
  conflict, unauthorized, expired-session, and long-content states.

### Later product work

- Model round flights independently from teams and extend score access so one
  player in a flight may enter scores for both teams in that flight. Never infer
  flights from tee times or starting holes.
- Add pairing generation/validation, explicit locked-score correction, public
  share links, tie-break configuration, offline-tolerant scoring, account
  recovery/email verification, signup rate limiting, deployment, backups, and
  production database roles.
