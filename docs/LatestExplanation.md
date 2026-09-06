# Latest iteration: Create another tournament and clearer result wording

Existing accounts can now create additional tournaments from Dine turneringer
through Opprett ny turnering, or by visiting /create while signed in. The
three-step wizard reuses tournament, round-plan and review fields without asking
for another username, password or player profile. Anonymous first-account
onboarding retains its original four-step/account/session/invitation behavior.

## Identity and transaction boundaries

The new CSRF-protected POST /api/tournaments uses a per-user request UUID. It
creates the draft trip, complete round plan and exact admin membership atomically.
An active linked player is entered with their current handicap captured for
this new tournament; otherwise the creator is a non-playing administrator.
Existing accounts, sessions, profiles, memberships and historical snapshots are
unchanged. Courses, pairings and invitations use existing administration after
creation; this path does not issue an invitation automatically.

Onboarding and signed-in creation share a pure normalized plan and transaction-
owned draft-plan insert. Schema 22 stores retry receipts with intentional
user/tournament cascade behavior. Session/user locks serialize separate sessions
of one account. Player facts are locked, and wall-clock session expiry is checked
after waits. Exact retries reauthorize admin access and return the same trip;
different payloads using the same key conflict. A successful plan remains
replayable after its end date; only a new creation checks today's UTC date.

The UI retains the submitted payload/key after an uncertain outcome, including
subsequent throttling or rejections. It blocks editing and duplicate submissions
until recovery, and never changes account identity. Account transitions unmount
the wizard so late responses cannot navigate or recreate private cache data.
Prior tournament lists are invalidated/refreshed, not overwritten.

## Wording corrections, not scoring changes

Completion now explains that visible open-round scores may already contribute
provisionally, while completion establishes a completed contribution for
qualification and counted-round selection. Standings and player history explain
qualification separately from displayed best-N selection. A mandatory round
reserves one counted slot even when not among the best scores.

A fully entered card now says Alle hull ført rather than implying the round is
completed. History uses Med / Ikke med i vist sammenlagtresultat instead of
suggesting excluded results were discarded. Completion, card confirmation,
locking and independent final-nine visibility remain distinct. No scoring,
ranking, qualification, lifecycle or visibility calculation changed.

## Validation and review

- Rust formatting, 114 workspace/all-target tests and all-feature Clippy with
  warnings denied passed.
- Full PostgreSQL 17 database ladder passed: 342 tests, including six creation
  tests covering multi-trip identity preservation, linked/unlinked/inactive
  players, cross-session concurrency, per-account keys, strict authority/input,
  rollback, expiry after a player-lock wait, and replay after the end date.
- Schema-21 upgrade preserves existing seed facts. Fresh schema-22 migration and
  repeated development seeding passed in a separate disposable database.
- All 348 frontend tests across 58 files, strict typecheck, ESLint and production
  build passed. Tests include sticky lost-response -> 429 -> retry, duplicate
  submissions, logout/account changes, unchanged anonymous onboarding, typed
  receipts and rendered wording regressions.
- Two real Chrome scenarios passed at 320, 390 and 1280px: an ordinary signed-in
  player created multiple independent trips with unchanged session and preserved
  previous memberships; a real committed request with a lost response recovered
  after a throttled retry without creating another trip. Live round standings,
  provisional qualification, player history, card completeness and lifecycle
  explanations were checked. Happy-path creation console/page errors and failing
  HTTP responses were empty before deliberate failure injection. Mobile review/
  standings and desktop completion screenshots were visually inspected.
- Independent read-only review found two retry edge cases; both were fixed and
  covered with regression tests. Final review reported no remaining findings.
  One full frontend run under concurrent build/database load timed out in an
  existing list-loading test; its focused rerun and subsequent full ladder passed
  without weakening test configuration or assertions.

## Release verdict and limitations

**READY WITH KNOWN LIMITATIONS.** The existing Vite bundle-size warning remains
non-blocking. Retry keys live only in the mounted wizard: after refreshing or
leaving an uncertain attempt, check Dine turneringer before starting a fresh one.

This change is code publication, not production deployment. Only disposable
PostgreSQL databases were changed. Deployment requires schema 22, refreshed
runtime permissions and matching API/frontend binaries, following the normal
backup/recovery runbook; no development seed belongs on retained data. A separate
production rollout/least-privilege rehearsal was not performed. Unrelated queued
product work remains out of scope.
