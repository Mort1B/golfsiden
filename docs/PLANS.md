# Active plans

This file is the source of truth for implementation work. Keep only one active
step. Move durable completed behavior to `Documentation.md` or `ARCHITECTURE.md`
instead of accumulating a historical log here.

## Planning status

Phase 2 is complete. No implementation step is currently approved. Invitation
redemption and the private-read workspace remain proposed later phases.

## Active step

No active implementation step. A direct approval is required before Phase 3 or
another bounded step begins.

## Upcoming work

### Phase 3: Reusable invitation onboarding

- Add append-only invitation redemptions with locked expiry, revocation, maximum
  use, duplicate-membership, and concurrency enforcement.
- Use a non-secret invitation ID plus a URL-fragment token. Provide minimal
  preview, new-account registration, authenticated acceptance, rotation, and
  revocation without matching identity by email.
- Add authenticated invitation reissue so an admin can recover from a lost
  one-time onboarding response without storing a recoverable plaintext secret.
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
