# Active plans

This file is the source of truth for implementation work. Keep only one active
step. Move durable completed behavior to `Documentation.md` or `ARCHITECTURE.md`
instead of accumulating a historical log here.

## Planning status

Phase 1 is complete. Phase 2 is proposed and awaits explicit implementation
approval.

## Active step

### Phase 2: Atomic creator onboarding

- **Goal:** Let a first-time visitor create a private tournament, their account
  and player identity, the complete draft round plan, an admin membership, an
  initial reusable invitation, and a session without partial records.
- **API contract:** Add public, rate-limit-ready
  `POST /api/onboarding/tournaments` with nested creator account/player data,
  tournament basics, and a nonempty round list. Reject client roles, actor IDs,
  status, round count, and summary scoring mode.
- **Transaction:** Hash the password off the async executor, then create the
  user, player, tournament, admin membership, tournament-player entry, initial
  tournament handicap history, all draft rounds, initial invitation, and session
  in one transaction. Set the cookie and publish events only after commit.
- **Round plan:** Require contiguous round numbers and one supported
  `scoring_format` per round. Derive `number_of_rounds` and tournament summary
  mode. Courses and tees may remain unset in draft, while readiness still blocks
  opening until configuration is valid.
- **Invitation issuance:** Store only a hashed random 256-bit token with expiry,
  revocation, and optional maximum uses. Return the initial raw link once after
  commit; redemption remains Phase 3.
- **Frontend:** Replace `/` with a private-trip start screen and add a mobile
  wizard for tournament details, rounds/formats, and creator account/player
  details. Submit once, cache the returned session/tournament, enter the
  tournament workspace, and expose the initial invite link.
- **Tests:** Cover full rollback at each constraint boundary, duplicate email,
  password hashing/session issuance, creator-as-admin-and-player linkage, exact
  derived round count/mode, unsupported or noncontiguous rounds, token hashing,
  and absence of post-failure events or cookies.
- **Validation:** Run the complete Rust and frontend ladders, clean/upgraded
  PostgreSQL migrations, repeatable seed, and real mobile/desktop browser flows.
- **Invariants:** A creator gets tournament admin authority only; participant and
  team identity stay independent; raw credentials/tokens never enter storage or
  logs; existing score and lifecycle behavior remains unchanged.
- **Stop condition:** The browser can atomically create a configured draft
  tournament and creator session, display the one-time-returned reusable invite
  link, and recover cleanly from every tested failure. Stop before invite
  redemption.

## Upcoming work

### Phase 3: Reusable invitation onboarding

- Add append-only invitation redemptions with locked expiry, revocation, maximum
  use, duplicate-membership, and concurrency enforcement.
- Use a non-secret invitation ID plus a URL-fragment token. Provide minimal
  preview, new-account registration, authenticated acceptance, rotation, and
  revocation without matching identity by email.
- Add `/join/:invitationId` and atomically create/reuse identity, entrant, player
  membership, redemption, and session.

### Phase 4: Private tournament workspace

- Move public tournament/player reads to membership-scoped resources and reserve
  later public access for explicit share tokens.
- Add contextual management routes under
  `/manage/tournaments/:tournamentId` for invitations, entrants, rounds, course/
  tee selection, pairings, handicap updates, and lifecycle actions.
- Add draft-only update contracts plus course/tee/hole reads. Keep hierarchical
  TanStack Query keys and complete mobile loading/error/empty/conflict states.

### Later product work

- Model round flights independently from teams and extend score access so one
  player in a flight may enter scores for both teams. Never infer flights from
  tee times or starting holes.
- Add pairing generation, locked-score correction, share links, tie-breaks,
  offline scoring, account recovery/email verification, rate limiting,
  deployment, backups, and production database roles.
