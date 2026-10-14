# A3 — identity-changing response ordering

Base revision: `3bc1e23cf8eb2bdf986afb61861cb608dfac42f7`.
Scope: the frontend's existing cookie-changing request boundary, session
publication/read coordination, affected route continuations, tests and docs.
No backend, API JSON, schema, dependency, sporting rule or deployment change.
The [assessment](../application-review-2026-10-14/README.md#a3--p2-late-account-creation-replaces-newer-authentication)
records the original real-cookie takeover reproduction at 390 and 1280px.

## Behavior and ownership

`api/identityQueue.ts` serializes all six cookie writers in this document:
login, logout, anonymous onboarding, invitation registration, player-claim
registration and own profile password changes. A generation starts when the
intent is queued. Waiting for response consumption prevents an earlier cookie
response from arriving after a later request has completed. The queue survives
route unmounts but does not persist credentials or coordinate other tabs/reloads.

Ordinary credentialed HTTP waits for final settlement. It captures the last
settled identity generation, so an old-owner request started during a transition
is rejected rather than dispatched under the final cookie. Headers and JSON body
arrival are fenced too. The settled generation advances with canonical session
publication, including slow reconciliation. The idle path dispatches immediately,
preserving existing signal cancellation. Explicit `credentials: 'omit'` capability
reads and public token previews bypass this identity boundary.

`AuthProvider` publishes the latest known successful writer's session/null;
failed/ambiguous requests reconcile with a raw cookie-backed session read. That
internal read bypasses the queue it is settling. A failed read clears private
state and exposes read-only authentication retry. No mutation retry is introduced.
Queued logout resolves the actual current cookie session, even if its original
canonical state was anonymous. Password changes compare the cookie's CSRF against
the original request authority and reject a replacement session before dispatch.

Forms stay mounted while their requests are pending. An accepted continuation
gets an event-loop turn to retain its receipt before central reconciliation can
change the route branch. Onboarding, invitations and login additionally fence
cache/state/navigation effects by intent, canonical identity and mounted lifetime.
Claims retain expected prepared player and username checks, exact lost-response
recovery and deliberate retries. Profile/recovery session reads share the same
publication fencing. Invitation acknowledgement wins over stale preview errors.
A failed newer login remains visible even when reconciliation finds the earlier
creator cookie; correct credentials can be deliberately submitted afterward.

## Regression evidence

Before production edits, four new API-boundary tests ran against original source:
**three failed, one passed**. The [retained summary](baseline-summary.txt) names
the failures: overlapping login dispatch, stale queued logout CSRF, and a password
change dispatched under renewed authority. Credential-bearing mock diffs are
omitted from the retained artifact. Original browser evidence remains in the
linked assessment; no production account was used.

Focused tests cover all writer families, held responses, failure release,
private reads begun before/during a transition, a slow failed-write reconciliation
gate, late body arrivals, reverse session-read order, null-to-null private-cache
clearing, departed form continuations, renewal, expected claim recovery and
accepted receipt precedence. Retained claim/profile/recovery behavior tests pass.

Read-only review found two races during implementation: prematurely marking an
identity settled while its reconciliation GET was pending, and a preview refetch
hiding an acknowledged invitation receipt. Both were repaired and gained retained
regressions. Final source review found no remaining actionable issue.

The first full frontend run caught an idle-path scheduling regression in retained
match cancellation tests. Removing the unnecessary idle wait restored the exact
signal/dispatch behavior; those tests were not weakened. A new sign-in test's
first fixture used the wrong default destination (`/score`); its explicit
`returnTo` was corrected. The new browser helpers initially imported a Vite-only
HTTP module in Node and used exact labels that include help text; both harness
issues were corrected. The API-driver logout probe also gained an explicit
intent-admission barrier before releasing its first response.

## Validation

| Check | Result |
| --- | --- |
| Original-source API regression | 3 failed, 1 passed as described above |
| Writer's focused frontend set | 100 passed in 12 files |
| Post-integration cancellation/login/queue/provider set | 44 passed in 5 files |
| Complete frontend unit/integration ladder | 1,133 passed in 147 files |
| TypeScript, ESLint and production build | Passed |
| Real Chrome identity, claim, profile and recovery matrix | 18 passed in 1.3 minutes; [captured result](browser-results.txt) |
| Independent read-only source review | No remaining actionable findings |
| Disposable backend migration/seed and readiness | Passed, unchanged schema 39 |

The full frontend total includes the focused tests. Backend source and schema
are unchanged, so their full test/Clippy/PostgreSQL ladders were not repeated.
The previously built API was migrated/seeded only for browser fixtures in rootless
PostgreSQL 17 container `golf-a3-20261014-pg`, port 55449, with synthetic data.
No hosted environment was used. Production files remain below 400 nonblank lines;
the largest changed production file has 247 nonblank/noncomment lines.

## Browser coverage and reproduction

Ten new scenarios use real API/PostgreSQL state:

- Hold the real committed onboarding or invitation response, navigate away in
  the SPA, and submit a newer login at 390 and 1280px. The newer POST must not
  dispatch until release. Then assert the cookie-backed account, displayed
  account, current tournament list, absence of departed receipt/navigation and
  exactly one original mutation.
- Keep ordinary onboarding's one-time invitation receipt, and invitation
  error/retry/long-content/success flows usable at 320, 390 and 1280px.
- Deliver an initial anonymous session read after a newer login and preserve
  the newer canonical/cookie identity.
- Probe queued logout from an anonymous canonical state, and two same-user
  renewals followed by a real CSRF-protected tournament creation. These two
  API-driver probes import the running page's actual API module in Chrome to
  exercise intents not simultaneously exposed by ordinary UI controls.
- Fail the newer login, reconcile the actual earlier creator cookie, retain the
  login error and complete a deliberate correct-credential retry.

Retained suites add three player-claim, three profile and two password-recovery
scenarios. Together they cover expected identity, lost-response recovery,
all-device password logout, unrelated-account preservation, token/cookie
independence, loading/error/empty/long-content states and normal account flows.
The suites check overflow, primary touch targets, page/console errors and relevant
request failures. Secret fields are masked in retained screenshots; no token or
password is included in the captured result summaries.

The first combined browser run passed 17 scenarios; recovery issuance stopped on
503 because the local API lacked `RESET_PASSWORD_ORIGIN`. Restarting the same
disposable API with the existing setting enabled the real recovery flow. A later
logout-probe attempt released its held response before its injected module had
queued logout; waiting for explicit probe admission fixed that harness race.
The final configured run passed all 18 scenarios without further changes.
The [mobile onboarding](onboarding-mobile.png),
[desktop onboarding](onboarding-desktop.png) and
[mobile invitation](invitation-mobile.png) screenshots were visually inspected.
The one-time invitation secret is masked in the onboarding captures.

From `frontend/`, with Vite on 5173 and the API on 3000 against the disposable
seed, set the API's existing `RESET_PASSWORD_ORIGIN=http://127.0.0.1:5173`, then run:

```bash
npm run test
npm run typecheck
npm run lint
npm run build
GOLF_IDENTITY_BROWSER=1 GOLF_PLAYER_CLAIMS_BROWSER=1 GOLF_PROFILE_BROWSER=1 GOLF_RECOVERY_BROWSER=1 npx playwright test --config playwright.lifecycle.config.ts identityOrdering.browser.ts identityIntent.browser.ts playerClaims.browser.ts profile.browser.ts passwordRecovery.browser.ts
```

Local logs: `/tmp/golf-a3-{before,focused,integration,frontend,types-final,lint-final,build,browser-final,browser-complete,browser-verified,migrate,seed}.log`.
This is same-document coordination, with no multi-tab, reload, physical-device,
public-host or production-load claim. Ordinary requests remain concurrent when
identity is stable; identity changes deliberately serialize. Latency was not
measured and no general optimization claim is made.
