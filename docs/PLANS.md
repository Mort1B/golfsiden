# Active plans

This file is the source of truth for implementation work. Keep only one active
step. Move durable completed behavior to `Documentation.md` or `ARCHITECTURE.md`
instead of accumulating a historical log here.

## Active step

### Session-backed authentication foundation

- **Goal:** Replace the public scorer environment identity with a secure
  server-derived session and enforce the existing admin, scorer, player, and
  viewer role model at mutation boundaries.
- **Files/modules:** Add focused authentication configuration, password/session
  services, middleware/extractors, API handlers, repositories, and tests under
  `backend/src/`; add only the forward migration needed for revocable sessions;
  update the seed; add decoded session APIs, an auth provider, and compact mobile
  sign-in/sign-out states under `frontend/src/`; remove runtime dependence on
  `VITE_SCORER_USER_ID`; update setup and architecture documentation.
- **Backend behavior:** Add `POST /api/auth/login`, `POST /api/auth/logout`, and
  `GET /api/auth/session`. Verify password hashes with a maintained Rust password
  hashing library, issue opaque high-entropy session tokens, store only token
  hashes, use secure HTTP-only same-site cookies, enforce expiry and revocation,
  and return the signed-in user's ID, display name, and role. Configuration must
  make cookie security explicit for local HTTP versus deployed HTTPS.
- **Authorization:** Require admin for player, handicap, tournament, entrant,
  round, team, and lifecycle mutations; permit admin or scorer for score writes
  and scorecard confirmation; keep health, viewer reads, leaderboards, and live
  invalidations public for now. Ignore actor IDs supplied by clients and derive
  `submitted_by`, `confirmed_by`, and handicap `changed_by` from the session.
  Return stable `401 unauthenticated` and `403 forbidden` JSON errors without
  leaking credential or session details.
- **Frontend behavior:** Bootstrap one session query, show a focused sign-in page
  when a protected workflow is entered without a session, hide Admin navigation
  from non-admin roles, disable scorer mutations for viewer/player roles, and
  preserve the intended protected URL through successful sign-in. Keep session
  server state in TanStack Query and clear protected cached data on logout.
- **Seed/development:** Give the deterministic admin user a documented
  development password without committing a production credential or accepting
  plaintext storage. Keep seed execution idempotent. Production startup must not
  create or reset an administrator implicitly.
- **Tests:** Cover correct and incorrect login, generic unknown-user responses,
  cookie attributes, expiry, revocation/logout, token hashing, role enforcement
  for every mutation group, client actor spoof rejection, session decoding,
  protected URL restoration, navigation visibility, and logout cache cleanup.
  Add database/API integration tests for concurrent logout/use and score audit
  attribution from the authenticated user.
- **Validation:** Run Rust format, all unit/database tests, Clippy with warnings
  denied, frontend install/test/typecheck/lint/build, diff and line checks, and
  real-browser mobile/desktop sign-in, refresh persistence, role denial, logout,
  Back behavior, console, cookie, and failed-request checks.
- **Invariants:** Passwords and raw session tokens never reach logs or storage;
  authorization is enforced in Axum and not inferred from hidden frontend UI;
  database lifecycle/integrity guards remain authoritative; public read-only
  leaderboards stay accessible; no production module exceeds 400 lines.
- **Stop condition:** The seeded admin can sign in, refresh, score with server
  attribution, access authorized mutations, and sign out; scorer and viewer role
  boundaries are proven by API tests; the frontend no longer needs a build-time
  user ID; validation, review, documentation, and publication pass.

## Upcoming work

### Administration workflows

- Add mobile tournament/player/handicap management and entrant registration.
- Add course/hole, round, team assignment, readiness, lifecycle, and score
  correction administration.

### Tournament operations

- Add shareable read-only leaderboard links and configurable tie-break rules.
- Add pairing validation UI and a balanced team generator that avoids repeat
  partners.
- Add offline-tolerant score mutation queuing with visible reconciliation.
- Add deployment, backups, monitoring, and production security hardening.
