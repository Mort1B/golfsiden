# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. The next candidate below needs implementation approval.

Every implementation step updates affected durable docs and LatestExplanation,
removes its completed plan item, commits only its scope and stops before the
next. No budgets, transfer penalties, newly generated picks, sporting team
generation, public Fantasy sharing, prizes/payments or offline queued selection
writes are in this plan.

## Next candidate

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

## Later queue

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

No further sporting scoring rules are planned. No automatic opponents/byes/brackets, team match play,
extra holes, public match sharing, cold offline launch or background sync is included.
Fantasy remains separate from sporting scores and completion rules.
