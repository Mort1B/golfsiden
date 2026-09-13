# Plans

`PLANS.md` contains only current and queued work. Completed behavior belongs in
`Documentation.md`; durable technical decisions belong in `ARCHITECTURE.md`.

## Active step: define public result-sharing scope

The user requested the next roadmap step after tournament tie-breaks. This step
is planning-only until the public audience and projection below are selected.
The pending scope question proposes live overall gross/net standings with existing
player display names, protected hidden finals, revocable links and a 30-day expiry.
Alternatives are completed-tournament results only or removing sharing from scope.
No private data is made public by this planning step.

**Goal:** Define a concrete, reviewable first public-sharing feature without
weakening the existing membership-private workspace.

### Proposed implementation contract, pending scope selection

- An exact tournament admin can create, replace or revoke one tournament-specific
  read-only result link. Creating/replacing requires CSRF and current authority;
  replacement revokes the prior link atomically. List metadata includes expiry
  and revocation state, never an existing secret. New secrets are shown only in
  the successful creation response. Creation is deliberate, never automatic.
- Anyone holding a valid link can view the tournament name, overall gross/net
  standings and existing player display names without an account. Include only
  displayed positions/ties, selected totals, qualification/provisional progress
  and final-round tie explanations needed to understand those standings. Use a
  dedicated public DTO/page: omit account identifiers, usernames, handicaps,
  membership/roster details, team assignments, contribution history, hole scores,
  private resource links and mutation controls. No search or public directory.
- Public visibility always follows the ordinary non-admin projection, even when
  the visitor is signed in as an admin. Hidden completed finals are excluded;
  a hidden open final includes only the permitted front nine. Totals, eligibility,
  places and tie metadata are assembled after projection, not redacted afterward.
  Re-hide must remove previously visible final facts on the next refresh.
- Reuse pure scoring, best-N and tie-break rules through an explicit public
  projection context. Do not call the current unrestricted repository helper:
  its absent-user branch intentionally bypasses member projection. Keep existing
  protected handlers and their membership locks intact.
- Links expire 30 days after creation and can be revoked earlier by an exact
  tournament admin. Revocation/rotation and public authorization serialize on the
  grant through assembly in one consistent transaction. Check expiry after lock
  waits using wall-clock time. Previously delivered information cannot be recalled;
  every subsequent read must reauthorize. Deleting the tournament invalidates its
  link. Grants belong to the tournament and survive their issuing admin losing
  that role; current exact admins retain revocation authority. Every management
  action reauthorizes its caller. Tournament completion/archive does not extend
  or silently recreate a link.
- Store only a hash of an independent random 256-bit secret; the link carries a
  non-secret grant identifier plus secret in its URL fragment. Send the secret in
  a typed request body, never a URL path/query, logs, analytics or query-cache key.
  Define fragment/reload handling explicitly during implementation so links stay
  reusable without introducing persistent private browser storage. Return uniform
  unavailable responses for invalid, expired and revoked grants; throttle public
  requests using existing trusted client-identity rules.
- The public page uses its own cache scope and never consumes a signed-in private
  standings cache. Responses are no-store and the page uses no-referrer/noindex
  behavior. Refresh current results every 15 seconds while visible and on return,
  with an explicit refresh control and last-success time. An already-open page
  can show its last authorized snapshot until the next refresh (up to 15 seconds
  plus request latency); revocation does not recall already delivered results.
  Clear the projection on tab return before awaiting renewed authorization. No access to private
  SSE is granted. Hide prior content when link validity cannot be re-established;
  stop refresh after terminal expiry/revocation, and isolate late responses when
  switching links. Polling and expiry timers must clean up on departure.
- Admin controls explain the names/results made visible and the expiry before
  issuance; provide copy, replace and revoke states with duplicate-submit guards
  and session/tournament-safe handling. No link is sent to anyone automatically.

### Invariants and validation for the proposed implementation

Preserve score ownership, administrator-managed teams, historical handicap
snapshots, locked-score rules, qualification and gross/net separation. Public
capability authority grants no membership, score-entry or account-recovery rights.
Keep domain projection, repository authorization/transactions, API transport and
frontend state in their established layers; sources remain below 400 substantive
lines. Use a forward migration after schema 25, with deliberate foreign-key/delete
behavior, indexed grant lookup and auditable issue/replace/revoke metadata.

Implementation acceptance must cover clean/upgrade PostgreSQL migration, defaults
without automatic links, exact-admin/CSRF denial, token hashing, expiry after waits,
concurrent rotation/revocation/read ordering, invalid-token response parity,
throttling and unchanged private endpoint access. Assert public JSON field
allowlists and hidden-score noninterference, including metric-specific tie-breaks
and admin-cookie visitors. Browser checks at 320/390/1280px must exercise issuance,
copy/replace/revoke, anonymous use, live refresh, tab return, late responses,
loading/error/empty/long names and expiry/revocation with no private cache reuse.
Run all affected workflow ladders and read-only authorization/privacy review;
update product, architecture, deployment and latest-iteration documentation before
publication. Production links and deployments are outside local validation.

**Planning validation:** Trace current leaderboard/visibility, token, routing and
cache boundaries; review the proposed public contract and run `git diff --check`.
Application/database/browser ladders do not apply to a plan-only change.

**Stop condition:** The proposed contract is recorded and reviewed, and the public
scope question is answered before implementation. If answered, update this bounded
step to the selected exact contract; do not start another queued feature.

## Later, as separate bounded steps

1. **Offline scoring:** Define durable queued-write ownership, explicit unsynced
   status, retries/idempotency and score-conflict resolution. Reauthorize every
   replay and reject locked rounds or revoked access; clear or isolate private
   data on account changes. Current return-to-app refresh is not an offline queue.
2. **One open round per tournament:** Decide the product rule before adding a
   PostgreSQL constraint. Current reads deterministically select the highest-
   numbered open round. Any enforcement needs an existing-data preflight and
   concurrency validation without silently closing historical rounds.
3. **Additional formats:** After roadmap completion, performance work and security
   review, define four-ball, Stableford and match play as separate contracts.
