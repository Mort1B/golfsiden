# Stableford domain foundation

The backend now has an isolated Stableford calculation module for the approved
18-hole individual contract. It calculates gross/net points, resolved progress
and the user-selected overall equivalent while preserving numeric stroke facts.
**Stableford remains unavailable in the application.** No format enum, API,
persistence, queue, lifecycle, ranking or UI behavior changed.

The implementation is `backend/src/domain/stableford/`; the full future contract
is in [Architecture](ARCHITECTURE.md#planned-individual-stableford-contract).

## Implemented behavior

Numeric entries earn `max(0, 2 + par - strokes)` points in each metric, applying
the preserved signed Playing Handicap by hole before calculating net points.
Explicit pickups resolve a hole for zero points in both views and have no numeric
stroke value. Blanks have no result and remain unresolved. Negative adjusted net
strokes remain valid, so points have no six-point ceiling.

Points, actual gross/adjusted-net strokes and overall equivalents are separate
opaque types. For a partial card the equivalent is `2 * resolved - points`; a
complete card uses `36 - points`. No resolved holes means no contribution. A card
of 18 pickups is complete with zero points and a +36 equivalent, while 18 blanks
are unstarted. Completeness describes arithmetic only, not confirmation authority.

Full actual stroke totals exist only when all 18 holes have numeric entries.
Seventeen pars and a 10 on a par-4 produce actual gross 78, 34 points and a +2
overall equivalent. Replacing the 10 with a pickup retains points/equivalent but
removes the full stroke total. A numeric correction from 9 to 10 remains visible
in the derived stroke facts even when its points do not change.

The pure projection accepts a caller-authorized hole mask and skips hidden inputs
before calculating points, progress, contributions or actual-total availability.
Allocation still uses the full 18-hole layout. Nine permitted holes with 20 points
contribute −2, not +16. Hidden numeric/pickup/blank changes produce exactly the
same projected object. Membership checks, exclusion of a completed hidden final
and public transport policy remain later integration responsibilities.

## Shared boundary and compatibility

Four-ball and Stableford share the identical numeric/unentered/no-score types in
`domain/player_score_input.rs`. The four-ball module re-exports the types at their
original paths. Numeric construction now returns the shared `InvalidGrossScore`
error instead of a four-ball error variant. This is an internal foundation change;
no serialized request, receipt or existing runtime error contract changes.
Four-ball aggregation, allowance calculations and all existing format policies
remain unchanged.

Stableford takes a preserved i16 Playing Handicap and validates par against the
existing 2–7 range plus a complete 1–18 stroke-index permutation. The fixed-size
card input does not support nine-hole competitions. This step adds no Stableford
allowance or snapshot-opening entry point; the defined 100% default and snapshot
policy must be connected in a separately scoped later integration step.

## Validation and remaining work

- All 20 new focused tests passed: contract examples, independent gross/net points,
  pickups versus blanks, actual-total presence, complete/partial contributions,
  zero-point corrections, plus handicaps, more than six points, numeric/layout
  validation, signed i16 extremes and full-object hidden-input noninterference.
- The full backend suite (170 tests) and all-target/all-feature Clippy with
  warnings denied passed, along with formatting. Existing four-ball tests remain
  passing after the shared input extraction; the transport enum still rejects
  Stableford.
- The full database-feature suite (465 tests), migrate and seed passed against
  a fresh disposable PostgreSQL 17.11 cluster using local binaries and an isolated
  loopback port. The temporary server was stopped after validation.
- Read-only scoring review found no material issues in the implementation or
  tests. Changed production files remain below the 400-line limit. Documentation
  references and diff checks passed.
- Frontend/browser checks do not apply: no user-facing path or frontend file
  changed. Playable release still requires snapshot/opening, storage and audited
  numeric/no-score mutations, compatible receipts/offline queues, confirmation,
  standings/history, private/public decoding and mobile scoring integration.

The generic allocator's previously recorded `i32::MIN` edge remains in the code
review queue; both new foundations' i16 snapshot inputs cannot reach it. No
unrelated allocator change or subsequent implementation step was started.

The next candidate is the match-play pure domain foundation.

**READY:** the bounded Stableford foundation is reviewed and validated. This is
not playable Stableford support or a full-format release verdict.
