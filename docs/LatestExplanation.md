# Playable singles match play

Singles match play is now available from tournament creation and manual opponent
assignment through numeric notes, online reporting, confirmation, completion,
audited correction and private match history. It uses a shared 18-hole tee and
flights starting at hole 1. Every active entrant needs exactly one same-flight
opponent. Administrators retain control of all assignments.

The official mode defaults to net with fixed 100% allowance and can be changed
to gross while draft. Opening freezes mode, opponents, tee and individual Playing
Handicaps. Net play allocates the full relative difference: handicaps 10 and 18
become 0 and 8 received strokes. Gross play allocates zero. A lead greater than
holes remaining ends the match; a tie after 18 is a draw. A 3&2 finish requires
no invented scores on holes 17–18. A win, draw and loss give 1½ points across three
played matches, in the separate private table.

## Notes, accepted reports and corrections

Numeric notes belong to the individual opponents and may be queued offline.
They never accept an outcome automatically. Online reports preserve the agreed
next-hole result and its numeric, concession, agreed-halve or organizer-ruling
basis. Whole-match concessions and awards have explicit provenance and can end
play before hole 1 without fabricating scores. The actual conceder must be named
and communication attested; recorder authority does not invent sporting consent.

Confirmation is bound to the current match revision and awards exact 2/1/0
half-point units. Every assigned match must be terminal and confirmed before
round completion or locking. Numeric note edits never rewrite accepted evidence.
An exact administrator can correct a recording error or record an organizer ruling
with a reason and an explicit replacement ledger. Superseded facts remain audited;
correction clears confirmation and points atomically. The corrected result must be
confirmed again, including when corrected through the dedicated locked-round path.
Ordinary locked writes remain forbidden.

## Contracts, privacy and offline recovery

Migrations 0030–0031 add matches, unique round-local opponents, player notes,
audit and immutable account-scoped receipts. Commands serialize through round then
match locks, recheck session and scope after waits, and atomically update ledger,
revision, confirmation, audit and receipt. Exact authorized replay returns the
original acknowledgement even after finish; a fresh stale command conflicts.

Migration 0032 makes overall configuration explicitly absent for match-only trips:
`counted_rounds` and mandatory round are null. Mixed trips count only non-match
formats and cannot make a match mandatory. Deferred cross-table validation permits
tournament-before-round creation. An internal generation row prevents concurrent
format changes from invalidating N under READ COMMITTED or REPEATABLE READ while
preserving public configuration timestamps and no-op behavior. Legacy non-match
configuration and scoring data remain intact.

Match-only private overall requests return a typed not-applicable result after
membership authorization. Mixed responses retain their existing shapes and
Stableford value basis, including configured draft rounds. Matches never enter
best-N or displace an eligible open provisional round. Final-round identity remains
the actual scheduled final; a final match cannot supply a stroke tie-break.

Private match projections derive only from permitted events before calculating
lead or finish. Read cards omit revision, event IDs and audit metadata. Hidden
facts are null; the whole hidden final is excluded from the non-admin match table
until release, including front-nine finishes. Public links retain their existing
overall-only scope and are unavailable for match-only trips.

A separate `golf-match-notes-v1` IndexedDB database uses `match_notes_v1` and one
generation-checked immutable chain across both opponents. The existing numeric,
four-ball and Stableford queues remain compatible. Match-wide leases and persisted
online-action markers protect concurrent tabs and unknown acknowledgements.
Original receipt identities survive uncertain 401/403 responses; known accepted
heads are verified before removal. Terminal/locked states block unsent successors.
Explicit review shows old, local and permitted server values without silently
rebasing; hidden values are distinguished from blanks. Failed storage, background
metadata failures and scoring denial preserve unsaved notes and navigation guards.
Authorization loss exposes only local recovery while authoritative actions stop.

## Validation

- Backend: **204 standard tests**, formatting, all-feature Clippy with warnings
  denied and binary build passed. Production source remains below 400 substantive
  lines. The initial sandboxed unit run could not bind mock HTTP servers; the
  authorized rerun passed.
- PostgreSQL: **558 tests across 55 targets**, including **22 match integration
  tests**, passed on disposable PostgreSQL 17.11. Fresh migration through all
  **32 versions**, populated schema-29 preservation, schema-31 normalization,
  migration command and seed twice passed. Coverage includes session/membership
  changes after waits, atomic rollback, direct-SQL integrity, receipt contention,
  terminal/confirmation/correction races, nullable overall and hidden-result
  noninterference. Both isolation levels exercise the configuration guard.
- Frontend: **572 tests across 102 files**, strict typecheck, lint and production
  build passed. Tests cover runtime contract rejection, wizard transitions,
  final-match tie explanation and durable queue/receipt behavior.
- Real Chrome: **30 distinct scenarios passed**: eight match scenarios plus nine
  Stableford, six four-ball and seven legacy offline scenarios. Match checks at
  320x600, 390x844 and 1280x900 cover manual setup and mode changes, real wizard
  creation with nullable N, numeric reporting, pre-hole concession, early finish,
  draw points, confirmation revision changes, locked correction, private player
  scope, hidden results/release, two tabs, storage failure, non-admin external lock
  and scoring denial. A separate regression verifies a dirty note survives
  background round-metadata 500, then 403 local-only recovery and successful retry.
  Loading/error/empty/populated/long-content states, keyboard focus, 44px controls,
  overflow, scroll/trial-click navigation clearance and console/network checks pass.
  Screenshots at all three widths were visually inspected.
- Read-only backend, API/queue and UI reviews have no remaining material findings.
  Review drove repairs to snapshot-isolation eligibility guards, unknown receipt
  handling, revision-bound confirmation, player scope, focus restoration and
  recovery/privacy after authorization changes.

Backend logs use `/tmp/match-final-{build,fmt,unit-unsandboxed,clippy,full-db,
cargo-migrate,cargo-seed}.log`. Frontend logs include
`/tmp/match-frontend-final-tests2.log`, `/tmp/match-frontend-build.log`,
`/tmp/match-browser-final3.log`, `/tmp/match-browser-metadata.log` and
`/tmp/match-legacy-browser.log`. Screenshots use `/tmp/match-setup-320.png`,
`/tmp/match-results-390.png`, `/tmp/match-scoring-1280.png` and
`/tmp/match-draw-member-320.png`. Temporary evidence is not published.

## Scope and readiness

**READY:** the coherent singles format is implemented, reviewed and validated.
The existing build warning remains: main JavaScript **772.97 kB**, **220.43 kB gzip**.
Match card listing intentionally uses bounded per-match reads, capped at 500 manual
assignments per round; performance work remains a separate queued step.

Individual next-stroke concession and organizer-award controls were not each
clicked separately in Chrome; their typed contracts and backend variants are
covered by automated tests. No Docker deployment or new production restore
exercise was run in this step; native PostgreSQL migration/seed and Chrome/API
integration provide the recorded runtime evidence.

Nine-hole/extra-hole play, byes/brackets, team match play, public match sharing,
rules adjudication, handicap-system submission, cold offline launch, background
sync and offline authoritative reporting remain outside this release. The next
application code review, performance work and security review remain queued.
