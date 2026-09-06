# Latest explanation

## Round lifecycle controls

The management workspace now lets the exact tournament administrator open,
complete and lock an existing round. Previously those backend actions were
implemented but had no UI controls. The Lifecycle section owns one URL-selected
round, and round detail pages link administrators directly to it.

Opening shows the existing server readiness report with affected entrants,
teams and flights and links to the selected round's setup. Completion and locking
show each required player/team scorecard, its hole progress and confirmation.
Editable links preserve the tagged owner in the existing scoring route;
read-only links use the existing historical card route.

Each transition requires explicit confirmation. Opening explains frozen setup
and handicap snapshots; completion explains counted results and continued
correction access; locking explains the end of ordinary correction access.
The final-back-nine visibility control remains independent.

## State and review decisions

No backend lifecycle policy or database schema changed. The frontend validates
mutation responses against the requested tournament, round and resulting status.
Actions fail closed during unavailable authority, loading, failed reads,
inconsistent state and incomplete or redacted readiness.

Reconciliation refreshes the affected private round, tournament, membership/list
and leaderboard consumers. It replaces reads started before the mutation outcome,
then inspects current query state rather than interpreting a newer SSE refetch
cancelling its request as a read failure. Tests cover both orderings: pre-commit
snapshots arriving late, and newer live reads replacing reconciliation.
Late mutation responses do not recreate private cache entries after unmount.

Review identified the pre-commit-read ordering risk and the existing manual
course form being discarded when a round opens. Both were resolved. Unsaved
manual fields remain mounted and disabled after opening; pairing drafts retain
their existing conflict flow. Cancellation restores keyboard focus after the
action becomes enabled again. Final read-only review found no remaining concrete
findings.

## Validation

- Frontend: 256 tests in 44 files, strict typecheck, lint and production build
  passed. React Testing Library covers controls, confirmations, exact-owner links,
  failure/retry, membership revocation, account changes and private cache cleanup.
- Backend: formatting, 114 workspace tests and strict all-target/all-feature
  Clippy passed.
- PostgreSQL 17: all 35 focused tests passed across round lifecycle, completion,
  completion concurrency, tournament authorization, private workspace reads and
  final visibility. Fresh migrations and development seed passed in isolated
  disposable databases.
- Chrome: both opt-in browser tests passed. The real local API/database flow
  covered tournament start, opening, one UI score entry, bulk score preparation
  through the authorized API, UI confirmation, completion, a second admin's UI
  correction, reconfirmation and locking. Scramble, foursomes, individual play
  and the 18-hole final were exercised. Real SSE updated readiness and another
  admin's lock. The final remained hidden for a non-admin member after locking;
  that member could not access management or its round administration link.
- Browser state/layout checks covered 320px, 390px and desktop widths, touch
  target height, keyboard confirmation/cancellation, retained disabled manual
  drafts, loading, retryable failure, empty owners/rounds and long names.
  Loading/error/empty fixtures used scoped response injection. The lifecycle run
  had no uncaught page errors, post-login console errors or unexpected network
  failures; the signed-out session probe's expected 401 was accounted for.

Run instructions for the opt-in browser suite are in Documentation.md. The
browser checks used the local development proxy, not a production deployment.
The entire PostgreSQL suite and deployment/recovery drills were not repeated:
backend/schema and production wiring were unchanged, and the focused reused
contracts were exercised instead. The existing Vite chunk warning remains
(568.20 kB minified, 165.03 kB gzip); bundle splitting is outside this step.

## Example and boundary

If a completed scramble card is corrected, its confirmation disappears and
"Lås runden" becomes disabled. The administrator follows "Bekreft scorekort" for
that exact team, confirms the corrected card and returns to lock the round.
Locking never releases the final's hidden holes.

Member flight presentation, flight progress and tournament closure remain queued.
This iteration stops after the round lifecycle UI.
