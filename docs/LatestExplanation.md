# Password recovery without email

Tournament administrators can now help ordinary players recover their shared
account. In the tournament's player list, open **Hjelp med glemt passord**, confirm
your own password and create a private link. Copy it to your existing contact
channel after verifying the player's identity. The receipt includes manual-copy
fallback and expiry; outstanding links can be revoked even after the receipt is
closed. Sign-in explains how to contact an organizer or the site operator.

The approved authority boundary applies across all tournaments: an administrator
may recover a linked active ordinary participant in their own tournament. An
account with any tournament administrator membership or global admin role needs
the site operator. Self-recovery and unlinked accounts use that same operator
fallback. The packaged CLI requires exact account identity, an audit reason and
owner database access; it writes the link to a new owner-readable file, without
printing it or accepting a replacement password.

For example, an ordinary player contacts their organizer, receives a link and
chooses a new password. The link expires after 30 minutes and works once. Merely
opening it does not consume it. A replacement link invalidates previous ones;
successful reset invalidates all of that player's old sessions and returns them
to normal sign-in. If the organizer happens to open the player's reset link in
an already signed-in browser, the organizer's unrelated session remains intact.

Recovery preserves account/player IDs, memberships, teams, scores and handicap
snapshots. Tokens are independent 256-bit capabilities stored as hashes. Account,
session, membership and grant locks serialize recovery with password changes,
login and authority changes. An append-only authority ledger prevents a grant
from becoming valid again after removal/readdition, promotion/demotion or
unlinking/relinking. Audit and terminal grant identity are retained. Operator
provenance requires actual database owner authority, not a caller-controlled flag.

The reset page captures the fragment token in memory and removes it from browser
history, then sends it only in POST bodies. Recovery responses are private and
non-cacheable. Secret-bearing UI state is discarded on departure, permission
refresh or identity change. Auth response guards prevent a late read from
replacing a newly signed-in account. Production PostgreSQL suppresses bind
values and error details while retaining query timings and basic error events.

## Deployment

Apply forward migration 0024 with owner authority, refresh runtime permissions,
and deploy the matching API/frontend. Add `RESET_PASSWORD_ORIGIN` to the private
production configuration before using the updated Compose file; it must be the
exact public HTTPS origin without a trailing slash. Recreate PostgreSQL to adopt
the new logging command settings. The API runtime role must not own or inherit
the recovery-table owner. See `deployment_guide.md` for upgrade, rollback and the
private operator command procedure. No production deployment or real account
recovery was performed during this step.

## Validation

- Backend formatting, all-targets Clippy with warnings denied, and the ordinary
  workspace/all-targets test run passed: **121 tests**.
- The complete PostgreSQL 17 workspace/all-targets database-feature ladder passed
  **383 tests**, including those 121. All **20 focused recovery tests** also passed
  after the final changes. Migrate and development seed succeeded on disposable
  PostgreSQL; an explicit schema-23 upgrade case preserved accounts/sessions and
  checked seed idempotence.
- Recovery coverage includes exact-tournament authorization, privileged/unlinked/
  inactive/self targets, CSRF, malformed/oversized requests, throttling, expiry,
  replacement/revocation, authority changes away and back, parallel redemption,
  old-session/stale-login rejection, and restricted-runtime provenance/audit guards.
  Additional contention cases exercise expiry during a lock wait, a real profile
  password update, queued relinking and an administrator membership moving into
  the target account. Order-sensitive cases observe actual PostgreSQL lock waits.
- Frontend: **418 tests in 72 files**, application and browser-suite TypeScript,
  lint and production build passed. The existing 500-kB bundle advisory remains;
  the main chunk is 625.30 kB minified (180.71 kB gzip).
- **13 Chrome scenarios passed**: recovery (2), profile (3) and return-to-app (8).
  Recovery's two scenarios passed again after final spacing changes. Checks cover
  320×600, 390×844 and 1280×900, long names, overflow, 44px controls, reachable
  actions, clipboard success/denial, loading/retry/missing/expired states and
  signed-out forms. The real API flow verifies issue/copy/reset, target-session
  rejection, preservation of an unrelated session, new-password login and
  revocation without a retained receipt. Console/network assertions passed and
  masked mobile/desktop screenshots were inspected.
- The actual production backend Dockerfile built successfully with Podman and
  Rust 1.88. Its packaged operator command issued to a mode-0600 private file,
  kept the secret out of stdout/stderr, passed public preview and revoked the
  grant on disposable PostgreSQL. Compose configuration validation passed.
- A synthetic PostgreSQL extended-bind probe reproduced sensitive-value logging
  with default settings and verified that the three new settings remove those
  values while retaining query/error events. No real secrets were used.
- Read-only authorization, concurrency and frontend review is resolved. Review
  corrections include fresh permission gating, late auth identity protection,
  logging configuration and deterministic contention ordering. Browser validation
  also caught and fixed reopening the same fragment after history removal.

The full production Caddy/Docker Compose deployment was not launched: the local
container validation used Podman, while browser flows used Vite and the local
API. Physical-device Safari/iOS testing was not available in this Chrome harness.
These environment checks remain operator acceptance work; the existing bundle
advisory remains recorded. No queued feature was started.

**Verdict: READY WITH KNOWN LIMITATIONS.** Implementation, affected local checks,
review and documentation are complete with the environment limits above.
