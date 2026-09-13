# Plans

`PLANS.md` contains only the active implementation step and a short work queue.
Completed behavior belongs in `Documentation.md`; durable technical decisions
belong in `ARCHITECTURE.md`.

## Active step

The user requested the next step. Recovery implementation is pending the authority
choice below; repository discovery and the concrete implementation outline are
prepared. No recovery endpoints, schema, or UI have been activated.

## Next candidate: administrator-assisted password recovery without email

**Goal and scope:** Let an authorized organizer create a reset link for a specific
existing player account and share it privately using their own SMS/chat channel.
The app has no email or messaging integration. Current password change requires
the old password; it does not provide forgotten-password recovery.

**Proposed flow:**

1. A player contacts the organizer, who verifies their identity through a known
   contact channel. Under tournament player management, the organizer chooses
   “Lag lenke for nytt passord” for that player and confirms their own password.
2. Show the single-use link once, with “Kopier lenke,” expiry, and an instruction
   to share privately. Proposed lifetime: 30 minutes. Allow revocation; generating
   a replacement invalidates older links for the account. Issuing a link alone
   does not change the password or sign the player out.
3. The player opens a public reset page, enters and repeats a new password, and
   submits. Merely opening or previewing the link does not consume it. Apply the
   same password policy as signup/profile and show useful invalid/expired/used
   link states. Successful reset invalidates existing sessions and outstanding
   reset links, then directs the player to normal sign-in.
4. Add “Glemt passord?” guidance on sign-in explaining how to contact an organizer;
   do not expose a public account directory or assume profile email is identity.

**Authority decision before activation:** Accounts are shared across tournaments.
Anyone who can generate a usable reset link can take over that entire account,
not just its participation in one tournament. Proposed initial policy: exact
tournament administrators may recover linked ordinary participants in their own
tournament; deny self-reset and targets with administrator authority globally or
in any tournament. Check stored account/player/membership links on the server,
not a typed username or client-supplied target account. Confirm this trust boundary
when activating recovery. Define an audited operator-assisted fallback for admin
accounts, the sole organizer, and players without an eligible organizer; do not
silently broaden tournament-admin powers or leave those cases undocumented.

**Implementation boundary and invariants:**

- Add an append-only migration for hashed cryptographically random reset tokens
  and recovery audit records (issuer, target, tournament context, times, outcome).
  Preserve account/player IDs, memberships, scores, and handicap history.
- Bind grants to the account's credential generation. Atomically recheck expiry,
  issuer authority, target eligibility, and generation when consuming a grant;
  serialize issue/replace/revoke/redeem and password/login races. Password hashing
  stays outside long database transactions and uses the existing bounded service.
  Reuse credential-generation session invalidation and the login race protections.
- Protect administrator actions with session authorization, CSRF, current-password
  confirmation, and throttling. Rate-limit public redemption and retain generic
  invalid-link responses. Keep secret tokens/passwords out of logs, analytics,
  persisted browser storage, referrers, and shared caches; select a trusted HTTPS
  reset origin and a token transport that also avoids proxy URL logging.
- Revalidate issuer/target membership and privilege changes through transaction
  boundaries; revocation must win over later redemption. Keep token access separate
  from ordinary login sessions and tournament invitation acceptance.

Security reference: the token lifecycle, normal sign-in after reset, and session
invalidation follow the [OWASP Forgot Password Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Forgot_Password_Cheat_Sheet.html).
Manual delivery and the proposed administrator authority are product decisions.

**Concrete implementation outline (pending authority choice):**

- Add a dedicated recovery module in domain, repository, and API layers, plus
  migration `0024_password_recovery.sql`. Keep recovery grants independent of
  invitation tokens and ordinary login sessions. Store grant UUID, SHA-256 token
  hash, target account, nullable target player for operator grants, credential
  generation, issuer kind, optional
  issuer/tournament context, expiry, and terminal outcome. Retain append-only
  issue/replace/revoke/redeem audit events without token or password values.
- Under the proposed tournament-admin policy, use the selected tournament player
  as the request target. The server resolves the linked account and checks exact
  issuer admin membership, target enrollment and membership, active player link,
  no self-reset, and no target admin role globally or in any tournament. Deny
  ineligible targets generically without revealing their other memberships.
- Administrator endpoints live under
  `/api/tournaments/{tournament_id}/players/{player_id}/password-recovery`.
  `POST` with the issuer's current password creates/replaces a 30-minute grant
  and returns its ID, expiry, and one-time link. A separate `POST /revoke` with
  current-password confirmation invalidates outstanding grants in this context;
  an already revoked grant is harmless. Neither operation changes the password.
- The public `/reset-password/{grant_id}#token=...` page captures the fragment in
  memory and immediately removes it from browser history. Preview and redemption
  use POST bodies under `/api/auth/password-recovery/{grant_id}`; preview does
  not consume the token and exposes no account directory. Redemption accepts the
  token and repeated new password, with generic invalid/expired/used feedback.
  Keep secret-bearing mutation data out of persisted caches and remove inactive
  mutation state. Public reset works even when another account is signed in.
- Reuse bounded Argon2 and the existing 12–128 UTF-8-byte password contract.
  Schema 23 already invalidates sessions when the password hash changes; login
  already rechecks verified credentials under a user lock before session creation.
  Recovery must preserve those protections and invalidate all outstanding grants
  on redemption or intervening password changes. Username-only changes do not
  advance credential generation and do not independently revoke grants.
- Serialize target-account mutations and grant consumption, and lock the target's
  existing memberships before the no-admin check. Protect absent/new memberships
  too: checking `NOT EXISTS` alone does not prevent concurrent promotion or insert.
  Use deterministic account-lock ordering and recheck session expiry, issuer
  authority, player/account links, eligibility, and grant generation after waits.
  Validate the exact lock order against profile, login, and membership races.
- Configure a trusted reset origin explicitly: `CORS_ALLOWED_ORIGIN` is optional
  today, so it is not an unconditional recovery-origin source. Production links
  require HTTPS; never derive the origin from Host headers. Tokens stay out of
  URI paths/query strings because the API trace middleware records request URIs.
- Add a server-only recovery CLI for administrator/sole-organizer/unlinked cases.
  Require deployment-operator access, an exact account identifier, and an audit
  reason after identity verification through a known channel. Issue/revoke the
  same short-lived grants without setting a password or granting a web role.
  Operator provenance is an explicit grant kind, never inferred from a missing
  issuer account. Package the CLI in the production backend image and document
  its deployment privileges and private-output workflow.
  Keep secrets out of command arguments, environment values, and ordinary logs;
  expose a newly issued link only through an explicitly selected private output.
  No operator recovery command exists in the current checkout.
- Keep frontend work in a focused roster recovery disclosure, one-time link view,
  public reset page, typed recovery API module, and sign-in guidance. Existing
  handicap editing and invitation acceptance retain their own flows. If the
  operator-only policy is selected, omit roster issuance and adjust guidance.

**Validation:** Require read-only authorization/concurrency review, clean and
upgrade-path PostgreSQL migration checks, and full backend/PostgreSQL/frontend
ladders. Cover wrong tournament, unlinked player, privileged target, self-reset,
expired/revoked/rotated/used tokens, issuer removal, target privilege/link changes,
parallel redemption (only one succeeds), password-change/login races, CSRF,
throttling, token leakage, and old-session rejection. Browser-check issue/copy/
revoke/redeem/login, clipboard failure, logged-in and logged-out recipients,
mobile/desktop layouts, and relevant loading/error/long-content states.

**Stop condition:** The agreed authority and fallback policy is implemented or
explicitly bounded, ordinary-player recovery works end to end without email,
security checks and review are resolved, and product/architecture/operator/latest
iteration documents are updated and published. No additional account features.

## Shared completion requirements for future implementation

Each active step must satisfy `docs/AGENT_WORKFLOW.md`, including applicable
read-only specialist review for contract, handicap, scoring, or synchronization
changes. Browser checks cover relevant loading, error, empty, populated, and
long-content states. Record exact blockers for skipped checks. Update
`Documentation.md` and `LatestExplanation.md` when behavior changes, and
`ARCHITECTURE.md` when a durable boundary/contract changes. Validate any necessary
migration against PostgreSQL after reading `migrations/AGENTS.md`.

## Later

- Configurable tie-breaks, public share links, and offline scoring.
- Decide whether PostgreSQL should enforce at most one open round per tournament;
  reads currently select the highest-numbered open round deterministically.
- After roadmap completion, performance work, and security review, decide the
  contracts for four-ball, Stableford, and match play separately.
