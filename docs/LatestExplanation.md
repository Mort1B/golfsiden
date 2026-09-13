# Automatic recovery when returning to Score

A native Chrome EventSource regression reproduced the reported class of loading
failure. The test opened a scorecard, ended its live stream, and returned HTTP
204 on the next connection attempt. Chrome stopped retrying. The app had cleared
protected completion/read data on disconnect, leaving Score waiting for an
`open` event that never arrived. Dispatching a persisted page-return event did
not recover the original implementation; the regression failed after five
seconds without finding the score input. This establishes a concrete defect,
not the exact cause on the user's unspecified original device/browser.

The shared subscription now handles visible-page return, persisted `pageshow`,
and `online`. It restarts stopped/retrying streams, retains healthy streams,
ignores superseded-source events, and cleans up return listeners. A deduplicated
return check refreshes the session before private HTTP reads and rechecks the
resulting user identity. Ordinary score events still target only score-dependent
queries and never refresh authentication. No polling or request-timeout rewrite
was needed for the reproduced cause.

Reconnection also exposed a related lifetime problem: clearing completion could
unmount the writable score input, losing failed intent and navigation protection.
The exact already-loaded, authorized writable card now keeps that component
mounted during temporary progress clearing and transient completion/access errors.
Its coordinator and confirmation observer survive. Cleared owner names and
progress are replaced with a generic label and recovery notice; writes, retries,
confirmation, and card navigation are disabled until recovery. Failed input stays
discardable. Protected read projections still clear synchronously, and terminal
authorization errors or authoritative lock/access changes retain existing guards.

For example, after a failed attempt to register 4 on hole 8, disconnecting and
returning keeps that failed 4 visible and guarded. Refreshing completion and access
does not issue another score write. Once connected, the player can deliberately
retry or discard it. A read-only card that previously showed hole 18 instead
hides during disconnect and returns to hole 9 if the refreshed response hides the
final nine.

Read-only review identified two additional recovery gaps, both resolved: transient
score-access errors must retain disabled unresolved input, and confirmation retry
must respect the same recovery/read-only/pending gates as the primary confirm
button. The final source review found no remaining material issues.

## Validation

- Frontend: `npm run test` passed all 397 tests in 66 files. Regressions cover
  shared stream lifecycle, return-event cleanup/coalescing, identity-first refresh,
  failed/queued edits, pending/failed confirmation, access denial, and existing
  resume/Back/Forward/first-gap behavior.
- `npm run typecheck`, `npm run lint`, `npm run build`, and strict browser-suite
  TypeScript compilation passed. Lint has no new warnings. The production build
  retains the existing greater-than-500-kB bundle advisory (613.72 kB main chunk).
- All 8 native Chrome browser tests passed using `npx playwright test --config
  playwright.lifecycle.config.ts returnLoading.browser.ts` from `frontend/`.
  The suite uses a real local HTTP event stream and controlled, runtime-decoded
  API fixtures; it needs the local Vite app but no seeded database. Mobile/desktop
  checks cover 320, 390, and 1280 pixels, screenshots, horizontal overflow,
  44px controls, click reachability, and console/network errors.
- Browser scenarios cover all three return events, repeated return with a healthy
  stream, delayed/error/empty/populated/long-content states, failed-save guards
  without duplicate writes, expired login, restricted read recovery, denied access,
  internal navigation, Back/Forward, and reload. Offline/frozen-page and lock-on-
  return checks are included in the same suite.
- Backend and PostgreSQL ladders were not applicable: no backend, persistence,
  migration, scoring formula, or API contract changed. This run did not exercise
  production Caddy, a real deployed API/database, or physical iPhone/Safari
  backgrounding/screen lock; those environments were not available in this harness.
  Persisted-page and visibility events are dispatched in Chrome, rather than
  claiming physical-device or native back-forward-cache restoration evidence.

**Verdict: READY WITH KNOWN LIMITATIONS.** The concrete stopped-stream regression
is fixed and input preservation is covered. The user's original device-specific
cause remains unconfirmed. Per-hole handicap badges and password recovery remain
separate queued work.
