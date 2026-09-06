# Plans

`PLANS.md` contains only the active implementation step and a short work queue.
Completed behavior belongs in `Documentation.md`; durable technical decisions
belong in `ARCHITECTURE.md`.

## Active step

None. The following sequence is planning-only, requested on 2026-09-06.
No application changes or runtime troubleshooting are authorized by this plan.
Promote exactly one candidate to the active step when implementation is requested;
complete, validate, review, document, and publish it before starting another.

## Next candidate — 1. Simplify the profile

- **Goal:** Put tournament access first and reduce the prominence of occasional
  account actions.
- **Scope:** Profile page/forms/styles, profile request validation and persistence,
  affected account guidance, tests, and current contract documentation.
- **Behavior:** Place “Mine turneringer” directly after the page heading, before
  profile forms and refresh guidance. Keep name and handicap readily visible
  below it. “Endre brukernavn” and “Endre passord” are separate, initially collapsed,
  keyboard-accessible sections with visible focus and clear pending/error states.
  Keep mutation feedback visible when a section is collapsed.
- Remove “Begrunnelse for handicapendring” from the self-service profile. This is
  a contract change: the current form and repository require a reason on handicap
  changes. Update client/server together; use a server-owned description such as
  “Egen profilendring” for self-service history while retaining actor, timestamp,
  and handicap history. Do not invent a user-provided explanation. Preserve the
  explicit reason requirement for administrator tournament-handicap corrections.
- Replace prominent UTF-8 guidance with simple password advice. Preserve the
  actual 12–128 UTF-8-byte contract; do not label it as 12–128 characters or change
  password policy implicitly. On invalid input, explain whether the password is
  too short or too long and how to fix it; show the exact byte limit and explain
  that some characters use more space only where needed. Check shared guidance
  across profile, creator onboarding, and invitation registration for consistency.
- **Invariants:** Preserve session/CSRF checks, optimistic versions, credential
  verification, private-cache isolation, inactive-player restrictions, comma/point
  handicap input, and fixed tournament/historical handicap snapshots.
- **Validation:** Profile/credential interaction and failure coverage; ASCII,
  Norwegian, and emoji password boundaries measured in bytes; profile handicap
  changes without typed reasons still audited; administrator corrections still
  require reasons. Run affected frontend, backend, and PostgreSQL ladders and
  inspect mobile/desktop layout, keyboard use, and async states.
- **Stop:** Profile changes and directly affected contracts documented and checked;
  no administration, scoring, or leaderboard work in this step.

## Queued candidates

### 2. Make administration task-oriented

- **Goal:** Give organizers a concise starting point with actionable links.
- **Scope:** Existing tournament management/lifecycle UI and authoritative
  readiness queries. Prefer existing endpoints and bounded per-round reads.
- **Behavior:** Add an organizer summary above the controls, ordered by round and
  next relevant action. Examples, only when supported by current server data:
  “Runde 2 er klar til å åpnes”, “Runde 3 har 2 spillere uten flight”, and
  “1 scorekort må bekreftes”. Link to the exact round's existing opening,
  flight-assignment, or confirmation control/card. Distinguish incomplete cards
  from complete cards awaiting confirmation. Show a calm all-clear/empty state.
  Use “Rundestyring” in place of user-facing “Livsløp”.
- **Invariants:** Summary links do not execute transitions or bypass readiness,
  exact-admin authorization, confirmation, or refresh gates. Stale/failed reads
  cannot claim readiness. Preserve administrator-managed teams and final-round
  visibility. This is an on-page summary, not missing-score alerts or notifications.
- **Validation:** Correct counts and destinations across draft/open/completed/
  locked rounds; incomplete versus unconfirmed cards; refresh/error/retry and
  membership changes. Run the frontend ladder and mobile/desktop browser checks;
  add backend/PostgreSQL ladders only if an API change proves necessary.
- **Stop:** Summary and navigation wording are complete; existing lifecycle
  controls and business rules remain authoritative.

### 3. Improve scoring flow and page spacing

- **Goal:** Make entering the next score the primary task and reduce page clutter.
- **Scope:** Score route selection, scoring composition/selectors, tournament-list
  spacing, and tournament-results introductory copy. Profile requests are in step 1.
- **Behavior:** On ordinary exit and re-entry to scoring, choose the lowest-numbered
  hole without a persisted score for the selected round and tagged owner, after
  authoritative card loading. Preserve that round/owner context within the session;
  clear private context on identity change. Do not count unsaved/failed input as a
  registered score. If every hole is scored, open the summary/confirmation view.
- Preserve intentional explicit hole links and in-page manual navigation. Define
  ordinary return through the Score navigation separately from a direct hole URL;
  browser Back/Forward retain their explicit route selection. Do not jump holes
  on each save or background refresh. Keep quick owner switches at the same hole.
  Read-only or restricted cards retain visible-hole canonicalization.
- Put active hole entry near the top, after a compact owner/round identity and any
  essential save/error/lock notice. Proposed selector design: show current
  tournament, round, and owner with an expandable “Bytt turnering, runde eller
  spiller/lag” section containing the existing labeled selects. Keep quick card
  switching and hole navigation easily reachable. Check this arrangement in the
  browser before settling spacing; no custom dropdown widget is required.
- Add clear vertical spacing between “Opprett ny turnering” and the current/
  archive/all filter row on the tournament list. Remove introductory explanation
  text above Resultater → Turnering's leaderboard; retain column labels, live/
  provisional indicators, visibility notices, and error/empty states.
- **Invariants:** Preserve serialized score writes, retry/discard and navigation
  guards, tagged player/team ownership, confirmation/correction gates, private
  reads, and administrator-controlled final-nine visibility.
- **Validation:** Return with gaps, no scores, all scores, failed saves, refreshed
  data, multiple rounds/owners, explicit links, Back/Forward, quick switching,
  locked/restricted cards, and identity changes. Run the frontend ladder and real
  browser checks at 320px, 390px, and desktop with long names and async states.
- **Stop:** Scoring navigation/layout and the two small list/results presentation
  changes are complete; leaderboard synchronization is handled separately.

### 4. Diagnose and repair tournament leaderboard live updates

- **Goal:** Make eligible saved scores visible in tournament standings as reliably
  as round standings, without manual refresh.
- **Current evidence:** Score-save/confirmation helpers already invalidate both
  gross/net tournament keys (`features/scoring/queries.ts`). Tournament aggregation
  has provisional contributions and counted/mandatory-round selection. The report
  is not yet reproduced; neither stale cache nor aggregation is a confirmed cause.
- **Investigation:** Build a reproducible case recording round status/format,
  counted/mandatory settings, owner and role, visibility setting, and saved hole.
  Compare the persisted card, round leaderboard response, tournament response, and
  rendered table before/after a save. Trace post-commit SSE publication, subscription,
  reconnect handling, canonical keys, mutation invalidation, and refetch races.
  Inspect provisional-round selection as well as team-to-player contribution
  attribution; distinguish a correctly unchanged counted total from stale data.
- **Scope/behavior:** Repair only demonstrated defects in that path, with a failing
  regression test first. Eligible committed changes must refresh gross/net
  tournament standings in both the scoring session and another connected member
  session. Failed writes must not appear as saved results. Reconnect and returning
  to results must reconcile authoritative data. Preserve counted-round rules;
  if the requested outcome needs a scoring-policy change, record that separately
  instead of silently changing ranking or qualification.
- **Invariants:** Preserve historical ownership/handicap snapshots, membership
  privacy, gross/net separation, and Phase 7C hide/release/re-hide behavior. Hidden
  scores must never leak through live totals, caches, or contribution drilldowns.
- **Validation:** Exercise individual and team rounds, partial/unconfirmed cards,
  completed/locked corrections where authorized, counted/non-counted and mandatory
  rounds, gross/net, same-session return, two-session SSE, reconnect, and final-nine
  visibility transitions. Use focused regression tests plus all affected workflow
  ladders, PostgreSQL for repository/API changes, and real-browser evidence.
- **Stop:** Reproduction, root cause, bounded fix and regression evidence are
  documented; if not reproducible, record the tested matrix and precise missing
  evidence without claiming the issue fixed. No unrelated scoring-policy redesign.

## Shared completion requirements for future implementation

Each active step must satisfy `docs/AGENT_WORKFLOW.md`, including applicable
read-only specialist review for contract, handicap, scoring, or synchronization
changes. Browser checks cover relevant loading, error, empty, populated, and
long-content states. Record exact blockers for skipped checks. Update
`Documentation.md` and `LatestExplanation.md` when behavior changes, and
`ARCHITECTURE.md` when a durable boundary/contract changes. Validate any necessary
migration against PostgreSQL after reading `migrations/AGENTS.md`.

## Later

- Configurable tie-breaks, public share links, offline scoring, and account
  recovery.
- Decide whether PostgreSQL should enforce at most one open round per tournament;
  reads currently select the highest-numbered open round deterministically.
- After roadmap completion, performance work, and security review, decide the
  contracts for four-ball, Stableford, and match play separately.
