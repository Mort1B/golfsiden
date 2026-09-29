# OFFLINE-RETURN-1 — 2026-09-29

**READY WITH KNOWN LIMITATIONS** for returning to the current session's exact
previously visited writable card and hole while the app remains open and its
required queries remain cached. Ordinary online resume is unchanged. No API,
schema, queue protocol, format or public-sharing changes are included.

## Reproduction and implementation

The controlled baseline test uses the previous committed ScorePage/workspace
implementations with the new regression scenario: visit hole 8, disconnect, queue
an entry, move to hole 9, depart and return through `/score`. It fails because no
explicit return action exists. The final implementation passes that same scenario,
returns to hole 9 and permits another device-persisted entry.

The prepared capability stores IDs and hole only. QueryClient remains the sole
owner of server cards and revisions. The new route marker alone grants nothing:
a matching visited target and current session/cache checks are required. Prefetched
cards, pending queue entries and another account cannot create that capability.
Observed terminal errors and lock/cache changes revoke it synchronously, surviving
later SSE clearing. Expiry and session replacement also revoke it. Fresh successful
workspace reads are needed to prepare again after a revocation.

Read-only review found an interaction between nonterminal transport errors and the
existing nondurable recovery gate. The exact eligible returned card now keeps its
normal input mounted during held IndexedDB persistence; storage, target, terminal
and navigation guards remain. Its regression keeps browser online status true,
fails access/completion/card reads, holds enqueue, then checks eventual durable
queue persistence. Browser testing also exposed an initial-login timestamp issue:
initial preparation now accepts the authenticated canonical session, while renewed
sessions still require fresh workspace reads.

## Validation

Environment: fresh task-owned PostgreSQL 17 container on loopback 55444, existing
schema-33 migrate/seed binaries, local API on 3000, Vite on 5173, synthetic accounts
and tournaments. No production/gg26.no access. Migration and development seed
succeeded; no database/backend source changed, so the Rust/SQL regression ladders
were not rerun for this frontend step.

- Full frontend unit/component suite: **919 passed** across 125 files, 42.12 seconds.
- TypeScript, ESLint and production build: passed.
- Real Chrome/API/PostgreSQL acceptance: **8 passed**, 32.1 seconds.
- Additional offline results-page navigation regression: **1 passed**, 2.7 seconds;
  Score remains reachable while result reads wait for connectivity, without a
  navigation implementation change.
- Focused production route browser regression: **16 passed**, 14.9 seconds.
  The optional broader run ended with SIGTERM after 13 passing cases; the complete
  affected route-loading suite was then run separately and passed.
- Diff whitespace and changed-file source-size review: passed.

The unit/component regressions cover the baseline return path, held persistence
with transport errors, route-marker rejection, sticky 401/403/404 evidence followed
by same-turn SSE clearing, list/detail/completion locks, cache removal, withdrawn
owner access, account/CSRF/logout changes, expiry, initial login, fresh re-preparation,
missing holes/wrong owners and read-only cards. Existing online first-gap resume,
confirmation, conflicts and queue tests remain in the full suite.

### Real browser acceptance

Stroke, team scramble, four-ball and Stableford each:

1. Open an authorized card online, then disable browser network access.
2. Persist two holes, move to hole 3, navigate to tournaments and use the Score link.
3. Explicitly return to hole 3 and persist a third hole.
4. Verify the original queued conditional request heads are unchanged.
5. Check pending-verification wording, unavailable card switching and confirmation.
6. Reconnect; verify queue drains, route leaves prepared mode and exact server values
   are `[4, 4, 4]` without moving the current hole.

Additional cases exercise an actual server-confirmed/locked round, a controlled
403 access response, a controlled short session-expiry response, and real logout/
login to a different seeded account with no prepared card. The denial and expiry
responses are deliberately injected; this report does not claim server-side
revocation can be learned without connectivity.

Layouts check 320×600, 390×844 and 1280×900, long names, queued/populated inputs,
return offer and unavailable/error states. No horizontal overflow; visible tested
controls meet 44px height and trial-click checks. Runtime console/page errors,
HTTP statuses and expected offline request failures are inspected. Screenshots
were visually inspected; excessive offer spacing was corrected and all eight
cases reran successfully afterward.

- [320px return offer](return-offer-320.png)
- [390px four-ball resumed input](four-ball-return-390.png)
- [1280px stroke resumed input](stroke-return-1280.png)

## Operational limits

This feature does not pin caches, persist full private cards or provide an offline
app shell. Normal inactive Query eviction can remove preparation while away.
Browser reload/closure/eviction requires connection before reopening. Session
expiry or observed denial/lock stops prepared return; queued edits are retained.
Unobserved server changes are resolved through existing conditional delivery and
fresh reads after reconnection. Confirmation remains online-only.

Deploy the updated frontend against the existing matching API/schema-33 release.
No new migration is required. Physical Android/iOS, native 200% zoom, hosted
rollout and browser eviction were not tested. Logs and additional screenshots
are under `/tmp/golf-offline-return-20260929/` and `/tmp/golf-offline-prepared-*.png`.
