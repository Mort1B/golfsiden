# Latest iteration — production integration and private proxy logs

DEPLOY-3 completes iteration three of the six-step Sunday readiness path. The
actual production Compose assembly runs with fresh schema 39, owner-only
migrations, real restricted runtime grants, secure browser sessions and native
SSE through Caddy. Docker Compose CLI 5.6.0 uses a private temporary rootless
Podman 5.8.7 API on amd64 because local Docker socket access is denied. No host
socket, group, daemon or deployed service changed.

The final live audit found a P2 deployment defect: Caddy reverse-proxy error
records contain the attached shared proxy secret when native streams close.
The default logger now drops whole request and response-header objects, keeping
status/duration/error diagnostics. This also avoids recording cookie, CSRF and
capability-bearing request metadata. The real web Dockerfile rebuilt with only
that reviewed Caddy overlay; API/PostgreSQL inputs and all 65 built web assets
remain unchanged. Actual error/access records from the rebuilt container omit
the secret and private canaries, with operational fields retained.

Thirty-nine named checks pass. Actual API startup rejects unexpected identity,
owner/excess authority, schema creation and migration-history writes. Missing,
pending, dirty, unknown and changed-checksum histories reject startup and return
generic readiness 503 while liveness remains 200. Database outage/reconnect
restores readiness. Runtime authority is checked at startup; readiness checks
database reachability and exact schema, not continuing role grants.

Seven real-Chrome checks cover distinct organizer/player UI login, Secure/
HttpOnly/SameSite=Lax host-only `/api` cookies, CSRF rejection, an acknowledged
persisted score and a native same-origin event updating the player's gross total
from 5 to 9. The player's final-round view preserves nine visible holes. Mobile
320/390 and desktop 1280 layouts, login errors, no-score/populated rows and long
names pass. Forged forwarding/internal headers cannot evade login quota.

Read-only review also caught inherited environment variables overriding the
private fixture. The runner now clears interpolation keys, binds generated
values, validates rendered destinations before service startup and passes
poisoned database/provider/port controls. Browser harness assumptions about
unscored links and player-visible holes were corrected; successful checks were
repeated against the rebuilt web. An early log check correctly failed before
native streams closed; complete post-browser logs pass. Fresh post-fix image
selection/rendering has a separate offline pre-start control.

Cleanup removes the three owned containers, three volumes, two networks and
private credentials/logs/CA/socket tree. Browsers close; four exact baseline/
repaired images remain. The [report](validation/production-rehearsal-2026-10-15/README.md)
retains receipts, immutable IDs, review, screenshot evidence and limits. Unchanged
Rust/frontend unit/type/lint/Clippy/domain ladders were not repeated; the affected
image/configuration/migration/grant/proxy/browser/log ladder was exercised.

DEPLOY-4 is next: critical organizer/player/Fantasy journeys using distinct
accounts. Current-application restore and final freeze/advisory/launch handoff
remain after it. Native Docker Engine, public DNS/ACME/certificate trust,
standard-port redirect following, the actual playing phone and off-host recovery
remain operator/release gates. This local run does not establish host readiness,
capacity or deployment sign-off. No feature or speculative optimization is added.
