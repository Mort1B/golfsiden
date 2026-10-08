# Prepared players, personal account claims and withdrawal

Date: 2026-10-08. Local PostgreSQL 17, Rust API and Vite frontend; Chrome via
`playwright.lifecycle.config.ts`. Task-owned database/container, not production.

## Behavior exercised

- Exact-admin player creation, one-time link copy and manual-copy fallback.
- Recipient claims the same player ID and preserved handicap with a new account;
  replay, expiry, revoked/replaced grants and username collisions fail safely.
- Concurrent claims and claim/withdrawal serialize without duplicate identity.
- Expiry after actual database lock/username waits rolls back account creation.
- Withdrawals reject draft assignments, participating open/completed rounds,
  self/admin targets and closed tournaments. Locked numeric scores, score audits,
  snapshots and handicap history remain byte-for-byte unchanged.
- Accounts, tournament membership and scorer role survive participation withdrawal.
- A held target account or membership lock returns a prompt retryable conflict
  with no partial entrant, audit or grant mutation, avoiding recovery lock cycles.
- Prepared active entrants can claim after completion or archival while ordinary
  membership creation stays forbidden. Reissue/revoke works after closure.
- Canonical UUID rate quotas, no-store/CSRF/authority handling, secret lifetime,
  held-response logout/login replacement, navigation/unmount and unknown-create
  recovery are covered by focused API/component tests.

## Commands and results

- `cargo fmt --all -- --check`: passed.
- `cargo test --workspace --all-targets`: 215 passed before final equivalent
  Clippy cleanup; full final database ladder also exercises these unit tests.
- `cargo test --workspace --all-targets --features database-tests --no-fail-fast`:
  **641 passed, 0 failed, 3 intentionally ignored measurements**, including
  12 new claim/withdrawal tests and the existing unit suites.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
  Existing vendored SQLx dependency warnings remain outside the application lint gate.
- `npm --prefix frontend run test`: 128 files / 942 tests passed.
- Frontend typecheck, lint and production build: passed.
- Focused claim API/components: 23 passed.
- `GOLF_PLAYER_CLAIMS_BROWSER=1 npm --prefix frontend run test:browser:lifecycle -- playerClaims.browser.ts`:
  3 passed against the real local API/database.
- Schema 33 was migrated and seeded before implementation. Upgrade to 34 and
  repeat seed succeeded, retaining all eight seeded players and entrants.
  New PostgreSQL tests also exercise fresh migrations.

The first sandboxed Rust run could not bind loopback HTTP test servers; rerunning
with local-network permission passed. Chrome similarly required permission for
its local socket. The first complete database run found two obsolete 405
expectations for the formerly retired POST route. They now assert 401/400 for
unauthenticated/legacy payloads and retain unchanged-row/event checks. A Clippy
nested-if finding and a reviewed withdrawal/recovery lock cycle were repaired.

## Browser evidence

All three scenarios check 320, 390 and 1280 px widths, overflow and 44px controls.
Real create/claim/reissue/revoke/withdraw flows are distinct from explicitly
intercepted loading/503/empty-list scenarios. Console/page errors were checked;
HTTP/network failures matched the deliberate rejection/interception scenarios.
Secrets/password fields are masked and traces disabled. Representative captures:

- [Admin mobile](admin-longname-receipt-320.png), [admin desktop](admin-longname-receipt-1280.png).
- [Claim mobile](claim-ready-320.png), [claim desktop](claim-ready-1280.png).
- [Withdrawn participant](withdrawn-390.png).

## Review and limits

Independent read-only reviews covered the plan, schema/locks, account claiming,
frontend decoding and secret/session lifetimes. Confirmed canonical-rate,
withdrawal/recovery lock-order and delayed-sign-out findings were fixed and
regression-tested; final source review found no outstanding actionable issue.

The three existing match-authorization performance measurements are intentionally
ignored: they require exclusive PostgreSQL with `pg_stat_statements` and are not
part of this feature. No physical device, native zoom or hosted DNS/TLS check was
performed. Browser tests do not establish every real-device/network race. General
in-flight authentication cookie races remain outside this step. Links are shared
manually; no messaging delivery, existing-account merge, access revocation,
restoration of withdrawn entrants or live-round withdrawal is included.

Ready for user-operated deployment with the limits above.

Deploy schema 34 with matching API/frontend and refreshed runtime grants using
the deployment guide. Hosting remains user-owned.
