# DEPLOY-3 — isolated production integration rehearsal

Iteration 3 of six for Sunday 18 October 2026. The supplied production Compose
assembly runs successfully with its actual API, web and PostgreSQL images,
production guards, owner migrations and restricted runtime grants. A reproduced
P2 proxy-secret logging defect is repaired and verified in a rebuilt web image.
This is local integration acceptance, not deployment sign-off.

## Findings and disposition

| Finding | Severity | Evidence and result |
| --- | --- | --- |
| Caddy reverse-proxy error records contain the shared proxy secret | P2 medium | Actual cancelled native SSE streams produced 12 original web records containing the synthetic secret. The default logger now deletes `request` and `resp_headers`; actual rebuilt runtime/error and access records retain operational fields without secret/header/URL canaries. Fixed. |
| Inherited shell variables can override the rehearsal's private Compose file | P1 high, fixture safety only | Review found a possible external migration destination in the newly written runner. All Compose interpolation keys are cleared, generated values are bound explicitly and rendered database destinations are checked before starting services. Poisoned database/provider/port environment controls pass. No external database action was observed; this is not a production application finding. Fixed. |

The logger deliberately retains levels, logger names, messages, status, sizes,
duration and upstream error diagnostics while omitting request/response metadata
objects. Filtering the global default logger covers both observed runtime and
access records; this was proved in adapted configuration and complete actual
container logs. It does not change proxy headers sent to the API. The repair
uses Caddy's documented [global logger](https://caddyserver.com/docs/caddyfile/options#log)
and [field filters](https://caddyserver.com/docs/caddyfile/directives/log#filter).

No sporting, session, authorization, migration, dependency or browser application
source changed. The remaining release gates are critical-flow acceptance,
current-application restore and final release/advisory/launch handoff.

## Environment and immutable artifacts

- Docker Compose CLI 5.6.0 used the real `compose.production.yml` through a
  private Unix API socket for rootless Podman 5.8.7 on amd64. Native local Docker
  access is denied by socket permissions; noninteractive sudo is unavailable.
  No socket permission, group, daemon or deployed-host changes were made.
- One randomly named owned project, three volumes and two networks. A private
  override selected exact local images with no build/pull during startup; only
  web ports were published, on loopback at random high ports. PostgreSQL's real
  network is internal; API/PostgreSQL have no published ports.
- New synthetic owner/runtime credentials and accounts, no production seed or
  retained database. Localhost Caddy internal CA, disposable Chrome contexts with
  explicit certificate exceptions, no global host/browser trust installation.
- API and PostgreSQL retain the exact [DEPLOY-2](../production-images-2026-10-15/README.md)
  images from source `b73d13e7f3aa8ee3508e306fdbb0b20584595937` and context
  `a32af6b17ec12f63abd5570d582c1ca49711a14a601de640aff0e1a8363ecea3`.
  All their build/runtime inputs remain unchanged.

| Service | Exact final image ID |
| --- | --- |
| PostgreSQL | `10318b2908d877b5d80b7460f7fc984ce11d08e31dfc8a7cc391562631cf76c7` |
| API | `1a6524c2727580acae040f98e978216de8a0ef9a86a7190bd463b18533a23bda` |
| Repaired web | `badbe24d6c7b796cb174a733e9124a792fbb666319fcbc9ee046637a2d6741e6` |

The actual frontend Dockerfile rebuilt from tracked source `22e90af` with only
the reviewed `deploy/Caddyfile` overlay, using cached bases with `--pull=never`.
[web-build.json](web-build.json) records the context, overlay SHA, tag and image
ID. All **65 web assets are byte-identical** to DEPLOY-2. Its old web image is
retained as the reproduced baseline; the repaired web is the integration
candidate. Future release builds must include the logging repair.

## Executed acceptance

[integration.json](integration.json) contains **39 passing named checks**.

- Fresh PostgreSQL initialization; all 39 compiled migrations applied by the
  owner, successful matching rerun, actual permissions job and no seeded users.
- Actual TCP runtime login/current identity, restricted cluster flags, no owner
  membership or public-object ownership. Real schema creation/alteration and
  migration-history update attempts fail with SQLSTATE `42501`. Application DML
  subsequently succeeds through onboarding, invitation, course, round and scores.
- Separate real API startup failures before listening for wrong identity,
  owner/excess authority, schema creation authority and history-write authority.
- Missing, pending, dirty, unknown and checksum-mismatched history each reject
  startup and return generic live readiness `503` while health remains `200`.
  Original history is restored exactly after each control. Readiness checks
  schema; runtime authority is checked at startup, not continuously by readiness.
- Owned database stop yields readiness `503` and health `200`; startup/reconnect
  returns readiness `200`. This is a short reconnect control, not recovery/soak
  or backup acceptance.
- Actual container image IDs, API UID10001 and production environment verified;
  correct runtime URL, secure cookies, migrations disabled and no CORS override.
  No automatic restarts were observed. Actual network membership, publication
  addresses, security headers and final health/readiness pass.
- HTTP redirects to HTTPS with the correct host/path. Caddy selects standard
  443; random host-port translation is not represented in the redirect. The
  browser uses its explicit HTTPS origin. Following that redirect at the random
  mapped port was not accepted or claimed; standard host 80/443 remains a host
  acceptance check.
- Real unfiltered SSE cancellation logs reproduce the proxy-secret leak. After
  rebuilding/replacing only web, complete fresh error/access logs contain no
  synthetic proxy secret or marked cookie/CSRF/URI canaries. Error/duration and
  access status remain present. The canary health request and native-stream
  closure are separate controls; a cancelled health request alone need not
  produce a reverse-proxy warning.

[browser.json](browser.json) records **seven browser checks** in real Chrome
155.0.8059.79 against the final rebuilt web, at 320/390/1280 CSS-pixel widths:

- Distinct organizer and player register through actual API flows, then log in
  through real UI forms in separate fresh browser contexts. Host-only session
  cookies are `Secure`, `HttpOnly`, `SameSite=Lax`, path `/api`.
- Missing CSRF rejects a score write with `403`, preserving zero scored holes.
  A legitimate score save is persisted and acknowledged in the actual UI.
- Native browser EventSource stays on the same Caddy HTTPS origin. The player
  receives a real `score` event and authoritative refresh from gross 5 to 9;
  reload preserves 9. The player's final-round projection remains nine visible
  holes, while the organizer scorecard retains 18.
- Login error, no-score rows, populated results, saved score and long names fit
  the tested widths. Screenshots were inspected. No unexpected console, request
  or response failures were observed in the successful case.
- Ten invalid logins with rotating forged forwarding/internal identity headers
  still reach the same resource quota; attempt 11 returns `429`/`Retry-After`.

The first two browser attempts exposed harness assumptions, not product defects:
unscored rows are not links, and a distinct player's final-round view counts nine
visible holes. The corrected case passed on the baseline web and was repeated
successfully after rebuilding web. The first post-rebuild log check ran before
the browser had closed native streams and correctly lacked the required warning;
the complete post-browser error/access check passes. These failed attempts are
not counted as passing runs.

## Harness, review and cleanup

`rehearsal.py` owns private configuration, Compose startup, actual SQL and owned
cleanup; `guards.py` exercises rejection and reconnect; `browser.cjs` uses the
installed frontend Playwright/Chrome; `proxy_logs.py` checks the real logging
regression; `build_web.py` builds the reviewed overlay; `audit.py` checks the
running candidate. No raw configurations, credentials, cookie values, capability
tokens, HARs or sensitive logs are retained here.

The recorded run started on DEPLOY-2, reproduced the defect, built/replaced only
web, and repeated browser/log/audit checks. A fresh **post-fix** setup now selects
the retained repaired web only when its exact reviewed overlay SHA and image
identity match. Skip baseline-only `proxy_logs.py before` and `build_web.py` in
that replay. It needs a new private rootless API control JSON with `pid`,
`directory` and Unix `socket`, or equivalent native-engine fixture adaptation;
the old private control file has been deleted. This harness is a validation
fixture, not an operator deployment tool. The supplied deployment guide remains
the host procedure.

[postconditions.json](postconditions.json) independently verifies retained IDs,
39/seven receipts, source scope, overlay hash, harness syntax and deleted private
tree. It also runs the fresh post-fix setup's actual image selection and Compose
rendering, stopping before the first service mutation; this last control is
explicitly offline, not another full integration run.

Read-only specialist review found the inherited-environment risk and the
post-fix replay gap; both were repaired and controlled. It also corrected the
CORS key and limited redirect claims. Production logging repair and final
closure evidence were reviewed. The primary agent owns implementation and
publication.

Cleanup removed exactly **three containers, three volumes and two networks**
owned by the rehearsal, then stopped the private API process and removed its
socket/private configuration, diagnostic, CA and fixture directory. Browser
contexts/browser were closed. The three DEPLOY-2 images and repaired web image
remain for subsequent isolated steps. Other services, images and host state were
not changed.

No Rust/frontend application, dependency or schema source changed, so unchanged
full unit/type/lint/Clippy/database-domain ladders were not repeated. The affected
deployment ladder was run: real web image build/config adaptation, actual fresh
migrations/grants, authority/schema controls, production Compose assembly,
browser/proxy/log regressions and final source/artifact/cleanup audit.

## Remaining limits

Native Docker Engine, the actual host architecture, public DNS/ACME/certificate
trust, standard-port redirect following, physical playing phones, production
capacity and off-host recovery are not established by this local run. Fresh
install versus existing-data upgrade remains an operator prerequisite. The
broader required organizer/player/Fantasy journeys are next (DEPLOY-4), current
backup/restore is DEPLOY-5 and final freeze/advisory/launch sign-off is DEPLOY-6.
No additional feature or speculative optimization is selected.

Screenshots: [login 320](login-320.png), [login desktop](login-1280.png),
[login error](login-error-390.png), [no score](leaderboard-empty-390.png),
[saved score 320](score-saved-320.png), [live mobile](leaderboard-live-390.png),
[live desktop](leaderboard-live-1280.png).
