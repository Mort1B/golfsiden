# Plans

`PLANS.md` contains only the active implementation step and a short work queue.
Completed behavior belongs in `Documentation.md`; durable technical decisions
belong in `ARCHITECTURE.md`.

## Active step

None. Choose the next bounded candidate before beginning implementation.

## Next candidate — 3. Improve scoring flow and page spacing

- **Goal:** Make entering the next score the primary task and reduce page clutter.
- **Scope:** Score route selection, scoring composition/selectors, tournament-list
  spacing, and tournament-results introductory copy.
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

## Queued candidates

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
