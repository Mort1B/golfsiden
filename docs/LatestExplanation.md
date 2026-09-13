# Public live result-sharing

Tournament admins can now open **Turneringsstyring → Innstillinger → Del resultater
offentlig** and deliberately create a reusable link to overall gross/net standings.
The page shows existing player display names and permitted summary results without
requiring an account. A link lasts 30 days; admins can replace or revoke it earlier.
The creation receipt supports copy and manual fallback and is the only place the
secret is shown. Nothing is sent to another person automatically.

The public page refreshes every 15 seconds while visible, on return to the tab,
and on explicit refresh. It shows positions/ties, selected totals, qualification,
provisional progress and applicable final-round tie explanations. It omits account
and player identifiers, handicaps, teams, contribution history, hole scores,
private scorecard links and score-entry controls.

## Visibility and capability boundaries

Public results always use ordinary non-admin visibility, even when an organizer
opens the link while signed in. For example, an open final with 73 gross strokes
but hidden back nine can show only its permitted 37 front-nine strokes. Changing
a hidden back-nine score does not change public JSON. Once a hidden final is
completed it is excluded altogether until release. Best-N selection and tournament
tie-breaks consume only that permitted projection.

Migration 0026 stores only hashes of independent random 256-bit secrets and derives
immutable issue/replace/revoke audits. Grant identity and 30-day expiry cannot be
edited through ordinary writes. Grant ownership belongs to the tournament and
survives issuer demotion; each management action checks current exact-admin
session, membership and credential generation. Parent tournament deletion
intentionally cascades capabilities and their audits.

A public read holds its grant lock through consistent projected result assembly,
checking wall-clock expiry after lock waits and after loading. Rotation/revocation
serialize with reads. Invalid, expired and revoked capabilities share a generic
unavailable response. Existing private handlers retain membership authorization;
the public boundary never calls the unrestricted internal read helper.

Frontend public DTOs allow only the selected summary fields. Each link visit owns
a separate QueryClient, and changing even only the secret for the same grant
replaces that cache. Refreshes and metric changes remove earlier snapshots;
failed reads hide rows, terminal unavailability stops polling and bounded expiry
timers avoid the JavaScript maximum-delay overflow. Delayed responses cannot
restore a previous visit. Private HTTP requests still include cookies by default;
public result requests explicitly omit them and use a 12-second timeout.

The reusable secret stays in the link fragment and transient page memory, never
local/session storage or cache keys. HTTP requests carry it in the body. The API
and updated Caddy routes set no-store, no-referrer and noindex/nofollow. Previously
delivered data cannot be recalled: an open page can retain its last authorized
snapshot until the next refresh, up to 15 seconds plus request latency.

## Validation

- Standard backend workspace/all-target suite: **131 passed**.
- Complete PostgreSQL workspace/all-target suite with `database-tests`: **414
  passed**, including 14 focused sharing integration tests and the token unit
  regression. Clean migration, schema-25 upgrade with actual history,
  development seed, exact-admin/CSRF/stale behavior, immutable audits, parent
  deletion and independent gross/net hidden-score noninterference passed.
- Concurrency tests observe real lock waits: reads before revoke/rotation, queued
  reads after revoke, simultaneous replacements, and expiry during both grant
  waits and fact loading. Formatting and all-target/all-feature Clippy with
  warnings denied passed.
- Frontend full suite: **466 passed in 78 files**. The final nonce adjustment also
  passed all 9 focused public lifecycle regressions; final typecheck (including
  browser TypeScript), warning-free lint and production build passed.
- Chrome sharing: **2 scenarios passed** at 320×600, 390×844 and 1280×900. The
  real API flow creates a fresh tournament/course/flight, issues/copies a link,
  displays anonymous live score changes, releases/re-hides final results, compares
  admin-cookie visibility, completes the hidden final, replaces and revokes the
  link. It also covers metadata loading/error/retry, delayed issuance and a real
  competing-admin stale write. Controlled states cover long names, gross/net
  ties, loading/error/empty, visible polling, tab return/offline, wrong/removed
  fragments, Back/Forward, late old responses and local expiry.
- Existing Chrome return-to-page regressions: **8 passed**; tournament tie-break
  browser regressions: **2 passed**. There were **12 passing browser scenarios**
  across the affected suites.
- Caddy 2.10.2 served shared HTML with all three privacy headers and retained the
  ordinary-page referrer policy. Shared public/admin API error paths also retained
  those headers with the upstream deliberately unavailable. Independent API
  integration tests assert headers on application responses and errors.
- Mobile/desktop screenshots were inspected, with capability fields masked and
  tracing/automatic failure screenshots disabled. Checks include horizontal
  overflow, 44px primary controls, trial-click reachability, no private API/SSE
  calls from public views and expected-only console/network outcomes.
- Read-only backend/frontend and durable-document review has no open findings.
  Diff checks passed; the largest changed production source has 286 substantive
  lines.

Initial non-escalated backend tests could not bind their existing local HTTP mock
sockets; the rerun with local socket access passed. Review found and resolved
missing audit protections, a privacy fixture whose final did not affect the
selected score, and same-grant secret cache reuse. The initial browser fixture
needed its required flight before round opening; a later polling race required
the test to accept revocation already detected before its manual refresh. Final
focused runs passed without changing the intended production rules.

**READY WITH KNOWN LIMITATIONS:** the existing bundle advisory remains at 645.93 kB
minified JavaScript (186.83 kB gzip). Physical iOS/Safari, full production Compose
image integration and a production deployment were not exercised; validation
used local Chrome, the development API, disposable PostgreSQL 17 and the actual
Caddy configuration in a disposable container. Deployment requires migration
0026, refreshed runtime grants and matching API/frontend/Caddy builds. No production
link was created. Offline scoring and all other queued features remain deferred.
