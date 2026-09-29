# Offline use on golf courses: current support and gaps

Date: 2026-09-29. Historical assessment before OFFLINE-RETURN-1.
The in-app return gap identified here is now addressed by the
[offline-return implementation and validation](../offline-return-2026-09-29/README.md).
Reload/closed-app launch limitations still apply. The table below describes the
pre-implementation baseline.

Read-only assessment of the current checkout. No offline runtime,
format, public-sharing, backend or database change is included.

**Existing open-card scoring supports interrupted connectivity. Reliable reopening
without coverage is not implemented.** Device-saved edits and an offline-ready
scorecard are different capabilities: preserving edits does not preserve the app
shell or all the data required to reopen the card.

## Practical behavior

| Situation | Current behavior |
| --- | --- |
| Lose signal with an authorized scorecard already open | Stroke/team, four-ball and Stableford hole entries can commit to IndexedDB and show saved-on-device status. Moving between holes remains supported. |
| Signal returns while the private workspace is running | Queue retries/backoff, reconnect/return wakeups, cross-tab leases and fresh server verification are implemented. Conflicts require an explicit choice; revoked access or locked rounds block delivery while retaining pending edits. |
| Navigate elsewhere, then use the main Score link without signal | Resume deliberately demands fresh reads. Returning to the previously loaded card is not provided as an explicit offline fallback. |
| Reload, reopen, or have the phone discard the browser page without signal | No supported offline launch. There is no service worker/offline app shell or persisted full authorized scorecard/query cache. Pending edits remain available for later online recovery, subject to browser storage retention. |
| Session expires or user logs out | Delivery needs a valid session. Device-saved queues stay account-owned; another account cannot read/replay them. Memory-only input labelled not saved on device does not survive authentication teardown/full reload. |
| Confirm a card or use authoritative match actions | Requires connectivity and server verification. Match numeric notes have a durable queue; match reports, concessions, awards, corrections and confirmation are online actions. |
| Close/background the app | No guaranteed background delivery. Reopen online and verify saved-on-server status. Clearing site data removes pending local edits; unsent device edits are absent from server backups. |

## Evidence and limits

Source inspection by a read-only project explorer and the primary agent:

- [ScorePage](../../../frontend/src/pages/ScorePage.tsx) requires fetched-after-mount
  reads on the main resume route; [workspace queries](../../../frontend/src/features/scoring/useScoreWorkspaceData.ts)
  request fresh data for that route.
- [Queue database](../../../frontend/src/features/scoring/offline/database.ts),
  [runtime](../../../frontend/src/features/scoring/offline/runtime.ts), and
  [provider](../../../frontend/src/features/scoring/offline/ScoreQueueProvider.tsx)
  implement durable account-owned edits and foreground delivery.
- [Application bootstrap](../../../frontend/src/main.tsx) creates an in-memory
  QueryClient; the existing runtime does not register an offline app shell.
- [Match queue](../../../frontend/src/features/matchPlay/offline/runtime.ts) is
  separate from authoritative match-result actions.
- The existing [offline browser test](../../../frontend/e2e/offlineScoring.browser.ts)
  named “real offline holes survive reload” explicitly returns online before its
  reload. It establishes durable-queue retention, not offline reloading.

The frontend suite was rerun during the accompanying management fix and includes
existing offline storage/runtime tests. Previously recorded real-server browser
coverage remains historical evidence. This assessment did not run a new offline
browser/server probe, simulate physical phone eviction, or test gg26.no or course
connectivity. Design limitations above are source-supported, not a newly observed
loss of device-saved scores.

## Use today

Open the correct scorecard while connected before entering a coverage gap. Keep
that page open, and wait for the explicit saved-on-device state after entries.
Avoid logout, reload or navigation away while disconnected. Reconnect with the
same account, resolve any reported conflicts, and check saved-on-server status
before confirming the card. This is a temporary operating procedure, not a
substitute for reliable offline reopening on a phone.

## Recommended next work

`OFFLINE-RETURN-1` in [the plan](../../PLANS.md) provides an explicit return to the
same account's exact previously loaded card while the app is still running,
without weakening normal online freshness checks. It must preserve conditional
revisions, denied/locked access handling and explicit pending-verification status.

A separate offline-preparation design is needed for browser closure/eviction:
prepare the app shell and exact scoring workspace while connected, define private
local-data retention and expiry, permit local entries without pretending to have
current server authority, then reauthorize and reconcile after reconnecting.
That capability is not implemented by the management fixes or the smaller
in-app-return proposal. No new scoring formats or public sharing are required.
