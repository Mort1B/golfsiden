# Plans

`PLANS.md` contains only the active implementation step and a short work queue.
Completed behavior belongs in `Documentation.md`; durable technical decisions
belong in `ARCHITECTURE.md`.

## Active step

No active implementation step. The next candidate requires a new implementation
request; account recovery remains queued after it.

## Next candidate: per-hole handicap strokes in Score → Oppsummering

**Goal and scope:** Show each selected player's or team's allocated strokes beside
the hole metadata in the existing summary, for individual stroke play, scramble,
and foursomes. Keep the first UI change within the shared summary component;
expanding the individual hole-entry screen is separate work.

**Behavior:**

- Place a compact `+1`, `+2`, or higher badge beside `Par … · Index …`, separate
  from gross/net scores so it cannot be mistaken for strokes over par. Include
  a short explanation, “Ekstra slag fra spillehandicap,” and an accessible label
  such as “2 ekstra slag på hull 1.” Show allocations before any score is entered.
- Omit zero badges. For a negative playing handicap, show a signed `−1` (or lower)
  with wording explaining strokes given back, rather than calling them extra.
- Extend scoring and read scorecard hole contracts with a required signed integer
  `handicap_strokes`, calculated in the backend from the existing
  `handicap_strokes_for_hole` domain function. Use the same owner playing handicap
  used for net scoring and the full configured hole count; do not derive the
  value from gross minus net or duplicate the formula in React.
- Preserve the field through the read projection only for visible holes. Update
  TypeScript contracts, runtime decoders, response fixtures, and API coverage
  together. No schema change is expected.

**Invariants:** Use the preserved round/player or team handicap treatment already
used by the scorecard. Never use today's profile handicap or allocate team strokes
to members separately. Do not change scoring, snapshots, totals, team membership,
locked-round behavior, or reveal embargoed holes. Allocation uses the actual
round length even when a read projection displays only nine holes.

**Validation:** Cover zero, negative, and multiple strokes per hole; 9/18-hole
rounds; reordered stroke indexes; individual and both team formats; unscored and
completed cards; owner switching; historical profile-handicap changes; and
restricted read projections. Example: playing handicap 20 over 18 holes gives
`+2` on indexes 1-2 and `+1` elsewhere. Run the backend/PostgreSQL and frontend
ladders applicable to the scorecard contract, read-only scoring/handicap review,
and mobile/desktop browser checks for readability, long names, accessibility,
and relevant async states.

**Stop condition:** Summary badges agree with net-score allocation for all covered
owners and visibility states; review, validation, documentation, and publication
are resolved. Do not begin recovery work.

## Queued candidate: administrator-assisted password recovery without email

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
