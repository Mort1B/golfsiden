# Round lifecycle expiry before commit

Authorized round opening, completion and locking now recheck the active session
with database wall-clock time immediately before commit. The check follows all
inner lifecycle work, including the separate singles-match transition branch.
It reuses existing authority locks and leaves readiness and sporting rules intact.

For example, an opening request can pass initial authorization and then wait on
membership until its session expires. It previously committed an open round.
It now returns the existing `401 unauthenticated` response, retaining the draft
round and rolling back player/team handicap snapshots and the first tournament
handicap-lock marker. The same guarantee covers later waits after pending status
writes. Rejected requests emit no SSE; a valid renewed session can deliberately
submit again and gets the ordinary single round event after successful commit.
No automatic mutation retry is added, and time is not frozen during COMMIT.

Six PostgreSQL/API regressions cover individual stroke play, two-player foursomes
and singles match play at membership and late status-write waits. Each exercises
all three lifecycle actions with expiry and valid renewed-session controls:
18 rejected and 18 successful requests. Rejections preserve complete stored
lifecycle state, scoring data, confirmations and audit/receipt history. The new
regression failed against the original source with 200 instead of 401.

Validation passed: 259 ordinary backend tests, 754 tests in the PostgreSQL-feature
ladder, formatting, all-feature Clippy, API build and schema-39 migration/seed
checks. The database total includes ordinary tests; four manual performance
probes were ignored. The initial default-concurrency database run stopped on an
unchanged A1 test's controlled-wait timeout. That test passed in isolation, and
the entire ladder passed with two test threads. The failure and rerun conditions
are retained; precise timeout attribution and test-only maintenance are deferred.

The unchanged Chrome lifecycle suite passed both scenarios at mobile and desktop
widths, covering opening/completion/locking, live updates, corrections, access
restrictions, final privacy and loading/error/retry/empty/long-content states.
Representative screenshots were inspected. Read-only review found no actionable
issue. No frontend, API shape, schema, dependency or deployment change is included;
frontend unit/type/lint/build checks were not repeated for this backend-only step.
The additional session query has no measured latency claim. Physical-device,
hosted and production-load acceptance remain separate.

The [validation report](validation/round-lifecycle-session-2026-10-14/README.md)
contains exact evidence and limits. [PLANS.md](PLANS.md) selects A3's authentication
ordering repair next. Other private read/write families and assessment findings
remain queued; no adjacent implementation was started.
