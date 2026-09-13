# Match-play domain foundation

The backend now has an isolated 18-hole singles match-play foundation. It
calculates relative handicaps, proposes numeric hole outcomes, derives a result
from ordered accepted reports and calculates exact match points. **Match play
remains unavailable in the application.** No format enum, API, database schema,
reporting authority, queue, lifecycle, standings or UI behavior changed.

The module is `backend/src/domain/match_play/`; the full future contract is in
[Architecture](ARCHITECTURE.md#planned-singles-match-play-contract).

## Implemented behavior

The default mode is net. With preserved Playing Handicaps 10 and 18, the lower
player receives zero and the higher receives eight strokes on indexes 1–8.
Signed plus handicaps use the same difference: −2 and 14 become 0 and 16.
Gross mode allocates zero. Widening before subtraction safely supports opposite
i16 extremes with a difference of 65,535. The foundation consumes preserved
Playing Handicaps; it does not calculate or freeze opening snapshots.

Two validated numeric gross scores can propose a hole outcome. Proposals are
separate from already-accepted reports, so recalculating a numeric comparison
cannot silently rewrite an agreed match score. The accepted report sequence must
resolve holes 1–18 in order; gaps, duplicate/out-of-order holes and any report
after a terminal result return explicit errors.

A lead greater than the remaining holes ends the match. Three up after 16 is 3&2;
two up after 16 remains in progress. Halving hole 17 then produces 2&1, while
losing the final two holes produces a draw. An 18-hole tie ends as a draw with no
extra holes. Concessions and organizer awards are distinct finish types without
invented stroke scores or numerical margins. A pre-start concession needs no
fake hole reports. Report counts represent resolved outcomes, not necessarily
holes physically played.

A caller-supplied confirmed status and a terminal result are both required for a
point award. Unconfirmed results award nothing; premature confirmation returns
an error. Wins/draws/losses use exact 2/1/0 half-point units. A win, draw and loss
sum to three half-units (1½ points), with checked addition preventing overflow.
These values never enter gross/net overall contributions in this foundation.

## Boundaries preserved

The new types have no transport serialization or persistence. They do not assign
opponents, grant authority to concede for somebody else, verify agreement/rulings,
withdraw concessions, correct records or implement receipt replay. Future reporting
adapters must validate those facts, retain provenance and record the effective
point needed for visibility. The domain's confirmation input is not an actual
confirmation action.

Restricted projections must supply permitted reports/events before deriving the
result. A test demonstrates identical state from the same permitted first nine
with different later winners; it is not a membership or final-visibility policy
implementation. Complete match-table exclusion for a hidden final and all public
sharing boundaries remain later integration responsibilities.

Existing four-ball, Stableford and legacy scoring calculations are unchanged.
The previously recorded generic allocator `i32::MIN` edge remains queued; the new
match allocation widens signed i16 inputs and does not call that allocator.

## Validation and remaining work

- All 21 new focused tests passed: signed differences and allocation sums, numeric
  gross/net proposals and mirrored opponents, strict early finishes, final-hole
  wins, draws, concessions/awards, malformed/trailing reports, confirmation gating,
  exact point sums and overflow, prefix derivation and runtime unavailability.
- The full backend suite (191 tests), formatting and all-target/all-feature Clippy
  with warnings denied passed. Existing format tests remain passing.
- The full database-feature suite (486 tests), migrate and seed passed against
  a fresh disposable PostgreSQL 17.11 cluster using local binaries and an isolated
  loopback port. The temporary server was stopped after validation.
- Read-only scoring review found no material issues in the implementation or
  tests. Changed production files remain below the 400-line limit. Documentation
  references and diff checks passed.
- Frontend/browser checks do not apply: no user-facing path or frontend file
  changed. Playable release still requires opponent setup, frozen mode/snapshots,
  authorized reporting and audited corrections, persistence/receipt guards,
  completion/confirmation, match-only/mixed configuration, private standings,
  visibility-safe projections, offline notes and mobile scoring integration.

All three new formats now have isolated domain foundations. The next candidate
is four-ball playable integration under its approved contract, scoped before
implementation; Stableford and match-play integration remain separate steps.

**READY:** the bounded match-play foundation is reviewed and validated. This is
not playable match-play support or a full-format release verdict.
