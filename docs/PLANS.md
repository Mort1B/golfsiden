# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. Planning only: the three requested changes below are not implementation
approval. Execute one bounded step at a time after approval.

Every implementation step updates affected durable docs and LatestExplanation,
removes its completed plan item, commits only its scope and stops before the
next. No budgets, transfer penalties, newly generated picks, sporting team
generation, public Fantasy sharing, prizes/payments or offline queued selection
writes are in this plan.

## Next candidate: FANTASY-PRESET — prepare every round in advance

**Goal:** Let an enrolled manager preset four golfers and a captain for every
existing tournament round before play, with independent per-round overrides.

**Current evidence:** `frontend/src/pages/FantasyPage.tsx` already lists all
rounds, and `backend/src/repositories/fantasy/lineups.rs` accepts a selection for
any round whose Fantasy window is open. The missing convenience is preparing and
reviewing all rounds together; deadlines remain authoritative.

**Scope:** The Fantasy lineup workspace, its draft/recovery state and focused
frontend/browser tests. Reuse existing per-round reads/saves, revisions and
immutable receipts. No bulk endpoint or schema change is initially needed.

**Behavior:**

- Show each round's saved four, captain, deadline and saved/unsaved/invalid/closed
  status together. Permit distinct future selections before earlier rounds begin.
- Offer an explicit action to copy a chosen four and captain into all still-open
  rounds. Preview destinations and existing selections that would be replaced;
  copying prepares drafts, and saving submits them. Preserve other unsaved edits
  unless their replacement is explicitly chosen. Resolve uncertain submissions
  before preparing replacements for them.
- Save reviewed rounds through the existing API in tournament order. Each round
  has its own request ID, expected revision and receipt; this is not an atomic
  tournament transaction. Show partial success accurately. Pause remaining saves
  on failure or uncertainty, retain unsubmitted drafts, and reconcile the exact
  uncertain request before resuming. Never resend acknowledged rounds because
  another save or a subsequent refresh failed.
- Each saved round remains independently editable until its authoritative close.
  Later source-lineup edits do not rewrite copied rounds. Newly created rounds
  appear unprepared and require another deliberate selection.
- Recheck eligibility on save and close. Picks remain player identities even
  before sporting teams are assigned; changing partners does not change picks.
  Carry-forward remains a fallback for rounds without a valid preset.
- Visibly exclude closed rounds. No automatic enrollment, late backfill,
  generated replacement picks, offline writes or access to others' private picks.

**Invariants:** Four distinct eligible golfers and a captain among them;
per-round server deadlines, pre-lock privacy, session fencing, idempotent recovery
and historical locked receipts remain authoritative.

**Validation:** Frontend full ladder from [AGENT_WORKFLOW.md](AGENT_WORKFLOW.md)
and real browser at 320/390/1280px. Cover three independently preset rounds,
copy-and-override, pre-team setup, a closed destination, expiry during saving,
withdrawal, insufficient eligible players, stale revision, partial success,
lost response/exact replay, failed refresh, reconnect and session replacement.
Verify carry-forward and second-account privacy; include loading/error/empty/
populated/long-name states, keyboard focus, console and failed-request inspection.
Backend/database ladders apply if an approved scope revision changes those layers.

**Documentation and stop:** Update affected Fantasy guide, README and architecture
descriptions, plus LatestExplanation. Stop after this flow is reviewed, validated
and published; do not implement the other candidates in this step.

## Later queue

### ROUND-AWARDS — optional closest-to-pin and longest-drive awards

**Goal and confirmed policy:** Let the tournament administrator optionally give
the closest-to-pin winner and/or longest-drive winner −1 each on their official
net golf result after a round. Both awards are independently optional: neither
is required for round completion or tournament results. Without an award, no
adjustment applies. Net golf results only: gross scores, recorded hole strokes
and all Fantasy points remain unchanged, including Fantasy placement points.

**Scope:** A typed domain award, append-only forward migration and repository/API
support, administrator round management, affected result projections and typed
frontend decoders/displays. Keep awards separate from hole score rows and
handicap snapshots. Do not repurpose Fantasy non-finish dispositions.

**Proposed behavior to finalize when activating this candidate:**

- Allow neither, either or both award types for each round; no winner is selected
  by default and no award is applied automatically. One winner and one −1 award
  per type per round. Proposed combination rule: the same player may win both,
  for a total −2, while repeated delivery of either award never adds another −1.
  Admin selects a participant from the preserved round field and records a
  reason. Persist award type, round/player identity, actor, timestamp and
  replacement/reversal history. Each award can be corrected or removed
  independently without changing the other.
- Adjust the winner's individual net round contribution and carry it into
  tournament standings under existing counted-round rules. In team rounds,
  preserve the shared team result and the partner's contribution; display the
  personal award separately. Do not attach it to a fabricated player score row.
- Define Stableford's net competition adjustment and display consistently with
  its points/equivalent result model before coding; do not subtract a stroke from
  an arbitrary hole or change actual stroke totals. Match play has no net stroke
  result: reject this stroke award there unless separately scoped later.
- Permit the award after round completion/lock, through an explicit audited admin
  path. Replacement/removal is a correction with a reason and fresh revision;
  ordinary locked score mutations remain rejected. An incomplete round with a
  non-finisher needs a separately defined after-play boundary before support.
- Display base net result, each named −1 award and adjusted result where relevant
  (round contribution, standings and history). Preserve gross views and hidden
  final-result visibility. Notify affected queries only after successful commit.
- Fantasy keeps unadjusted sporting placement facts as well as unchanged hole
  points. Audit shared leaderboard adapters so the award cannot indirectly change
  Fantasy placement, golfer totals, captain totals or manager rankings.

**Invariants:** Preserve exclusive player/team score ownership, round-specific
teams, handicap snapshots, gross/net separation, individual aggregation,
counted-round/tie policies and auditable locked-round corrections.

**Validation:** Domain cases for neither award (72 stays 72), either award
(72 becomes 71), different winners, both awards to one winner (72 becomes 70),
duplicate delivery per type, independent replacement/reversal, ties and
counted-round selection. Verify awards are never required for completion or
results. Cover team-partner isolation, the agreed Stableford behavior and
explicit unsupported-format/state
rejection. Assert unchanged Fantasy placement and totals even when the adjusted
golf ranking changes. PostgreSQL tests cover admin authorization, foreign-round/
tournament identities, uniqueness per round/award type, concurrent admins, stale
writes, locked-round corrections, clean/upgrade migration and seed behavior.
Run backend, PostgreSQL
and frontend ladders plus mobile/desktop admin/member flows, error/retry states,
long names and concealed results. Update behavior, architecture, deployment and
latest-explanation docs in the implementation step.

**Stop:** Publish only these two optional award types and their validation. No
general bonus editor, other prizes or automatic winner/tie detection.

### FANTASY-SCORING — distinguish net scoring from an exceptional actual shot

**Goal:** Keep the requested +5 for net albatross or better and give an actual
hole-in-one a substantially larger award. Make the numerical tradeoffs explicit;
there is no unique mathematically correct Fantasy scale without first choosing
an objective and, for probability-based weights, representative outcome data.

**Proposed scale:** Recommend +20 for an actual gross hole-in-one, subject to
acceptance when this candidate is activated. This replaces the earlier proposal
to retain +10 for aces. Other values remain as requested or already implemented:

| Recorded outcome | Proposed points |
| --- | ---: |
| Actual hole-in-one (gross 1) | +20 |
| Net albatross or better (net delta ≤ −3), without an actual ace | +5 |
| Net eagle | +3 |
| Net birdie | +1 |
| Net par | 0 |
| Net bogey / double / triple | −1 / −2 / −3 |
| Net quadruple or worse, or explicit pickup | −5 |

**Balance rationale and behavior:** The positive non-ace tiers are 1, 3, 5;
the ace is a separate exceptional-shot award, not an extrapolation from net
strokes. Net one alone does not establish an actual ace. Award one category
only: an actual ace also qualifying as net albatross gets +20, not +25. A
captained non-ace net albatross contributes +10 hole points; a captained ace
contributes +40. Placement and match awards and captain rules remain unchanged.
Match play still awards only its accepted match outcome, with no ace bonus.

An ace at 20 equals four net albatrosses, 20 net birdies or twice the first-place
placement award of 10. This deliberately allows one shot to outweigh that
placement award; it is a game-balance recommendation, not a calibrated claim
about event probabilities or equal expected points across handicap levels.
Preserve shared-team scoring: both selected partners contribute the same award,
so a team ace contributes 40 to a manager selecting both, or 60 if one is captain,
before other holes and placement. Review these effects before finalizing +20.

**Confirmed compatibility policy:** The user chose only games with no locked
selections yet. Introduce a new rules version for new games and upgrade existing
games only if no selection window has effectively closed. An expired deadline
or round opening counts as closed even before lazy materialization. Games whose
rules have frozen retain version 1, with +10 for both ace and net albatross;
do not recalculate started/completed locked games or mix versions in one game.

**Current evidence and scope:** `domain/fantasy/holes.rs` combines ace and
albatross at 10. The results repository reports version 1, frontend
`api/fantasy/decode.ts` accepts only version 1, and migrations 0035/0037 constrain
and freeze stored rules versions. Coordinate a forward migration, rule-version
dispatch in pure domain scoring, authoritative game/result metadata, result
revisions, API decoders, version-aware displayed rules, tests and documentation.
Never edit applied migrations or silently change the meaning of version 1.
Serialize eligibility for an upgrade with deadline/opening; ensure configuration
defaults create the new version and old games remain readable after deployment.

**Invariants:** Preserve recorded scores, snapshots, selections, privacy,
settlement and ace precedence. Results, breakdowns, revisions and displayed
rules must agree on the game's version. No unrelated scoring changes.

**Validation:** Regressions for non-ace net −3 and better (+5), gross ace also
net −3 (proposed +20 once), net eagle (+3), captains, shared team/four-ball
outcomes, Stableford and unchanged match-only awards. Include explicit balance
fixtures for an ace versus several birdies plus placement, both selected
partners, and captain multiplication. These demonstrate consequences, not
statistical calibration. Assert version-1 games retain +10 for ace/albatross,
new/unlocked games use the accepted new scale, and mixed-version games are
rejected. Verify manager/golfer round and overall totals, ties, revisions,
upgrade races and
expired-but-not-materialized windows. Run backend, PostgreSQL and frontend
ladders, clean/upgrade migration tests, and mobile/desktop browser checks of
both versions' rules, breakdowns and rankings. Update README, Documentation,
Architecture, LatestExplanation and deployment/version-compatibility guidance.

**Stop:** Publish the accepted ace/albatross scale and compatibility behavior.
Do not add a configurable scoring editor or recalculate locked version-1 games.

### Previously deferred work

- Optional offline preparation for browser closure/eviction: requires a separately
  scoped app-shell and private scorecard-retention/reauthorization design before
  implementation. Current in-memory return and durable queued edits remain distinct.

- Deferred broader security assessment: further production proxy/database privilege,
  recovery and dependency-advisory investigation. Revisit for a specific finding
  or a separately requested assessment, rather than making another broad review
  a prerequisite for continued functional testing.

### Remaining deployment acceptance

The remaining PERSIST-1 same-account/actual-login scenarios may be revisited if
testing feedback warrants them; see the original
[validation limits](validation/match-note-retention-2026-09-23/README.md). Local
PostgreSQL validation is now available through rootless Podman. The functional
checks cover real database-backed match recovery after locking, but do not claim
every separate session-replacement scenario.

Public-host acceptance still needs a Docker Engine environment and public DNS/TLS
validation. Native 200% browser zoom and physical Android Chrome remain unverified.
Hosting and deployment checks belong to the user. These historical acceptance
limits do not expand the application-readiness step into server administration
or another assessment. Broader device coverage and polish can follow feedback.

Outside the scoped optional round-awards proposal, no new sporting scoring rules
are planned. No automatic opponents/byes/brackets, team match play,
extra holes, public match sharing, cold offline launch or background sync is included.
Fantasy remains separate from sporting scores and completion rules.
