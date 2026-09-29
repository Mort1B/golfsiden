# Return to a prepared scorecard during a coverage gap

The Score page now offers **Tilbake til åpnet scorekort** when disconnected or
when its reads fail. It returns to the current session's last visited writable
card and hole, using existing in-memory data. Stroke/team, four-ball and Stableford
are supported. Ordinary online resume still fetches fresh data and finds the first
missing hole.

The returned card is labelled pending server verification. Golfers can continue
entering holes into the existing device queue; confirmation and card switching
wait for fresh access/status/card reads. Original request IDs and expected revisions
are preserved. Reconnecting delivers queued entries through existing conflict
checks and resumes the ordinary verified workspace.

Only actual visited writable cards are prepared. The marker contains IDs/hole,
not a second cache. Known denial, noneditable rounds, missing caches, session expiry
and account/session changes invalidate it. Revocation is sticky even when a later
SSE disconnect clears the query error. A route parameter, prefetched neighbor or
pending score alone cannot reopen a card. An unavailable returned card offers a
fresh online retry without deleting pending edits.

Example: load the card before losing coverage, score holes 1 and 2, then move to
hole 3. After leaving the page, choose Score and explicitly return to the opened
card. Hole 3 remains selected; its entry joins the existing queue. Reconnection
checks and delivers all three exact values.

The controlled baseline reproduces the missing return action. Eight real Chrome/
API/PostgreSQL scenarios pass, including all four formats, preserved request heads,
exact server values, a real locked round, controlled denial/expiry and changed
accounts. An additional offline results-navigation case and all 16 production
route-loading cases pass, alongside 919 unit/component tests, TypeScript, ESLint
and the production build. Phone and desktop layouts were checked. Results and limits
are recorded in the [validation report](validation/offline-return-2026-09-29/README.md).
Read-only review found no remaining actionable issue after the transport-error
persistence repair.

**READY WITH KNOWN LIMITATIONS:** deploy the frontend update; no new migration is
required. The app must remain open and required data must remain cached. Offline
reload/closed-app launch, pinned/persisted private cards and background sync are
not included. Hosting remains user-owned; further implementation should follow
specific testing feedback or a separately scoped offline-preparation request.
