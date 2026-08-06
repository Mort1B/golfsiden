# Active plans

This file is the source of truth for implementation work. Keep only one active
step. Move durable completed behavior to `Documentation.md` or `ARCHITECTURE.md`
instead of accumulating a historical log here.

## Active step

### Administration authorization boundary

- **Goal:** Require authenticated admin authority for every existing management
  and round-lifecycle mutation while preserving public viewer reads and the
  scoring authorization delivered in the previous step.
- **Scope:** Inventory and protect player creation/edit/deactivation, handicap
  updates, tournament creation, entrant registration, round creation/open/
  complete/lock, team creation, and membership assignment/removal. Add one
  reusable role guard instead of handler-local role comparisons. Keep locked
  score correction deferred until its explicit workflow exists.
- **Actor attribution:** Remove remaining client-supplied audit actors, including
  handicap `changed_by`, and derive them from the session. Reject legacy actor
  fields through strict request DTOs. Preserve existing database audit and
  lifecycle transaction contexts.
- **Frontend:** Keep viewer pages public. Protect `/admin`, add a compact mobile
  administration index for the operations already supported by the API, and
  expose actions only to admins. Hidden controls are not an authorization
  boundary; every mutation must fail with stable `401` or `403` responses.
- **Tests:** Add a route-by-route authorization matrix covering unauthenticated,
  viewer, scorer, player, and admin sessions; actor-spoof rejection; session
  expiry/revocation; exact error envelopes; lifecycle concurrency; and frontend
  route/action visibility. Retain all teammate scoring tests unchanged.
- **Validation:** Run Rust formatting, unit/database/API tests, Clippy with
  warnings denied, disposable migration/seed checks, frontend tests/typecheck/
  lint/build, real mobile and desktop browser flows, review, and publication.
- **Invariants:** Public reads and live leaderboards remain available; only
  admins manage tournament state; player identity never comes from email;
  score/team/flight semantics remain round-specific; files stay within the
  repository size limits.
- **Stop condition:** Every existing non-scoring mutation is protected by the
  shared admin policy, the first useful mobile admin workflows operate end to
  end, all validation passes, and the implementation is documented and pushed.

## Upcoming work

### Flights and tournament operations

- Model round flights independently from teams. Extend the existing score-access
  resolver so one player in a flight may enter scores for both teams in that
  flight. Never infer a flight from matching starting holes or tee times.
- Add pairing validation UI and a balanced team/flight generator that avoids
  repeat partners while retaining manual overrides.

### Product hardening

- Add shareable leaderboard links and configurable tie-break rules.
- Add offline-tolerant scoring, deployment, backups, monitoring, production
  database roles, login rate limiting, and an explicit locked-score correction
  workflow.
