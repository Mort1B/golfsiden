# Durable offline scoring

An already-open authorized scorecard can now keep accepting hole scores after
connectivity drops. Each edit is saved on this device before delivery, so the
scorer can move between holes while offline. **Lokale scoreendringer** shows
pending edits, delivery failures, blocked edits and conflicts, with a link back
from other private workspace pages. Device-storage failure remains visibly
unsaved and keeps a short navigation guard.

When somebody else changes the same hole, **Sammenlign scorer** shows the local
and current server values. The scorer explicitly chooses **Behold serverscoren**
or **Bruk min lokale score**. Choosing local makes a new conditional request;
another intervening change requires another comparison. Discarding an edit
removes only its device copy and cannot undo a request that already reached the
server.

## Safe delivery and current server state

Migration 0027 adds server-controlled score revisions and immutable delivery
receipts. Conditional requests bind an account-scoped request ID to an immutable
target, expected absence or exact score ID/revision, and strokes. The repository
reauthorizes the session, membership, owner and editable round before every
attempt, including receipt hits. Score writes and receipts commit atomically.
Locked rounds and revoked access cannot be bypassed through replay.

For example, one request saves 5 but loses its response. Another scorer then saves
6. Retrying the original request acknowledges its earlier application without
restoring 5. If the first scorer entered a successor while waiting, that successor
expects the revision that actually saved 5 and conflicts with the later 6. It
never silently adopts the unrelated revision returned by a fresh read.

Existing legacy score writes retain their last-write-wins contract, while all
actual stroke changes advance revisions. True no-ops and matching retries add no
duplicate score audit, SSE event or confirmation invalidation. Member and public
result projections remain unchanged; only authorized scoring DTOs expose
revisions. Receipt retention is tied to parent data, with no timed pruning that
could turn an old retry into a new write.

The browser queue lives separately from authoritative TanStack Query data.
IndexedDB transactions coordinate tabs, immutable requests, delivery leases and
exact-generation conflict/discard decisions. Account and session fences prevent
old callbacks or another signed-in account from displaying or replaying the
queue. Logout retains unresolved edits for the original account's later login.
No credentials or full private scorecards are persisted.

A delivery acknowledgment is not treated as current server state. The browser
refreshes the authorized card before showing server-confirmed scores and net
values or allowing another edit on that hole. Bounded refresh failures retain
that verification state while other queued holes continue. Cached authorized
input remains usable during connection recovery without restoring cleared
private progress or hidden results.

Confirmation stays online-only. It requires an empty queue for that account/card,
completed verification and a fresh authorized read. A local confirmation lease
prevents another tab from enqueueing on that card during the operation. The
existing POST still confirms the current server card; it does not introduce an
immutable reviewed snapshot. Explicit correction mode continues to support
confirmed open/completed cards, while locked cards stay read-only.

## Validation

- Standard backend workspace/all-target suite: **133 passed**. Formatting and
  all-target/all-feature Clippy with warnings denied passed.
- Complete PostgreSQL workspace/all-target suite with `database-tests`:
  **428 passed**, including 12 conditional delivery integration tests. Coverage
  includes lost-response replay, mismatched request IDs, absent/stale/ABA conflicts,
  no-op effects, direct-SQL revision protection, authorization expiry, lock races
  and cross-round request-ID collision rollback.
- Clean migration and development seed passed against disposable PostgreSQL 17.
  A populated schema-26 database was created with the actual previous migrator,
  seed and API, then upgraded to 27. Its 18 existing scores gained revision 1;
  original score columns/timestamps, 18 audit rows, confirmation and round handicap
  snapshots had identical before/after fingerprints. No receipts were invented.
  Historical integration fixtures retain their old-schema assertions and use a
  bounded legacy fixture helper rather than current DTOs against old columns.
- Frontend suite: **493 passed in 82 files**. Typecheck, lint and production build
  passed. Tests cover durable transactions, repeated edits, successor revisions,
  cross-tab claims, exact-generation choices, account isolation, storage failure,
  request timeouts, verification and online-only confirmation.
- Chrome: **20 distinct scenarios passed across the resolved runs**: seven offline
  cases, eight native return-to-page cases, two handicap summaries, the scoring
  navigation flow and two public-sharing regressions. They cover offline
  reload/reconnect, both conflict choices, two-tab lost-response successors,
  logout/account isolation, device-storage failure, confirmed completed-card
  correction followed by locking, immediate offline discard and hanging reads
  after acknowledgment. Public anonymous visibility and private-cache isolation
  remain intact.
- Browser checks use 320×600, 390×844 and 1280×900 viewports, long content,
  loading/error/empty/populated/blocked states, overflow checks, 44px controls and
  trial-click reachability. Mobile/desktop screenshots were inspected; console
  and network outcomes were checked alongside visible behavior.
- Read-only source and durable-document review has no open findings. Changes
  preserve player/team ownership, handicap snapshots, lifecycle authority and
  hidden-result boundaries. The largest changed production source has 269
  substantive lines, below the 400-line limit.

Review and validation resolved strict request decoding of extra fields, legacy
schema fixtures, stale displays after acknowledgment, delayed cross-tab local
updates during delivery, local actions paused by offline mutation defaults, and
confirmation lease deadlines. Browser fixtures were updated for durable
navigation and open-round authorization; cancelled hanging-request handlers need
explicit cleanup. These changes preserve the intended production contracts.

**READY WITH KNOWN LIMITATIONS:** the production build retains its bundle advisory at 661.39 kB minified JavaScript
(192.23 kB gzip). The dependency audit reports existing development-tool advisories
for js-yaml (high), Vitest and @vitest/mocker (moderate); the new dev-only
fake-indexeddb dependency is unaffected. Those upgrades are separate maintenance.
Physical iOS/Safari and full production Compose integration were not exercised;
Chrome and the local API used disposable PostgreSQL 17 because Docker socket
access was unavailable. No production deployment or production sharing link was
created.

Deployment requires migration 0027, refreshed runtime grants and matching API
and frontend builds. Pending edits survive reload for later online delivery, but
cold offline app launch, service workers and background sync remain outside this
step. Clearing site data removes pending edits, which server backups do not
contain. No queued confirmations, score deletion or administrator correction UI
is introduced. Further roadmap work remains separately approved.
