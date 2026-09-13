# Four-ball domain foundation

The backend now has an isolated, tested four-ball calculation module. It computes
per-player handicaps and derives independent gross/net side results from two
player cards. **Four-ball is still unavailable in the application.** This step
adds no scoring-format enum, endpoint, database migration, queue behavior or UI.

The module is `backend/src/domain/four_ball/`; its full future integration contract
remains in [Architecture](ARCHITECTURE.md#planned-four-ball-stroke-play-contract).
Existing individual, scramble and foursomes calculations remain unchanged.

## Implemented behavior

The allowance function accepts the exact uncapped course-handicap numerator used
by the existing tee formula. It defaults to 85%, accepts 0–100%, applies the
allowance before final rounding and rounds signed halves toward positive infinity.
For example, 9.6 at 85% becomes 8; −10 at 85% becomes −8. Widened arithmetic and
checked snapshot conversion return explicit errors for unsupported ranges.
Disabled handicaps produce zero course and playing snapshots, while a zero
allowance retains the course snapshot and sets only Playing Handicap to zero.

Inputs distinguish numeric gross scores from unentered and explicit no-score.
Numeric scores retain the existing 1–20 range. The module uses each player's
preserved Playing Handicap and a full 18-hole allocation; partial cards never
redistribute strokes across only the played holes. Plus handicaps give strokes
back at the highest stroke indexes. Net scores may legitimately be negative.

Gross and net select their counting partners separately. Every equal winner is
retained in stable player-identity order for display. The contract's three example
holes total gross 14 (+2) and net 11 (−1); no team handicap is invented. One
numeric partner makes a side-hole scorable, while two missing/no-score inputs do
not. A complete side can have one partner's card entirely missing.

Full-card inputs require exactly 18 holes and a complete stroke-index permutation.
Duplicate partners and invalid layouts fail even on empty cards. Progress counts
side-holes once; no scored holes means no total. The completeness flag describes
arithmetic only and grants no confirmation or mutation authority. These domain
states have no persistence or transport serialization yet.

## Validation and limits

- All 17 focused tests passed, covering the documented arithmetic, no double
  rounding, negative half boundaries, disabled/zero allowance, uncapped indices,
  snapshot extremes, independent and tied winners, input ranges, pickups versus
  blanks, negative net, card progress, front-nine allocation and invalid layouts.
- The existing transport enum still rejects four-ball identifiers. A regression
  assertion preserves the legacy individual negative-half rounding policy.
- Formatting, the full backend test ladder (150 tests) and all-target/all-feature
  Clippy with warnings denied passed. The first sandboxed test attempt could not
  bind local mock-server sockets; the full rerun with local networking passed.
- The full database-feature suite passed (445 tests), and migrate/seed succeeded
  against a fresh temporary PostgreSQL 17.11 cluster. Docker socket access was
  denied, so existing local binaries were used without changing a shared database.
  The temporary server was stopped after validation.
- Read-only scoring review found no material issues in the implementation or
  tests. Production source files remain below the 400-line limit.
- Frontend and browser checks do not apply: no user-facing path or frontend file
  changed. Persistence, audit/revision/receipt guards, lifecycle/confirmation,
  private/public result projections, overall attribution and mobile scoring remain
  later integration work before four-ball can be exposed.

Code inspection also identified an existing extreme-input edge in the generic
stroke allocator at `i32::MIN`. The new foundation accepts only i16 snapshots,
which cannot reach it. Repair of that unrelated generic boundary is deferred to
the queued code review; existing calculations were not changed in this step.

The next candidate is the separately bounded Stableford pure domain foundation.
No subsequent format or integration step was started here.

**READY:** the bounded domain foundation is reviewed and validated. This is not
playable four-ball support or a full-format release verdict.
