# Playable individual Stableford

Stableford is now available from tournament creation through scoring, offline
synchronization, confirmation, round completion, standings and private history.
It uses individual player cards on a shared 18-hole tee. Administrators can edit
handicap use and the 0–100% allowance while draft; the default is 100%. Opening
freezes the tee, tournament handicap and correctly rounded playing handicap.
Teams remain administrator-managed and are not required for this format.

Native round standings rank gross/net points descending. Overall standings use
36 minus points for a complete card, or twice the resolved visible holes minus
points for a partial card. For example, 40 points contributes −4. A numeric 10
and a pickup may both earn zero points, but only the numeric entry preserves
actual strokes. Eighteen pickups form a complete zero-point card with +36 overall
contribution; eighteen blanks have no result and cannot be confirmed.

## Preserved input and result contracts

Migration 0029 adds dedicated player inputs, audits and immutable delivery
receipts. Numeric/pickup/numeric corrections retain the input UUID and advance
its positive revision. Writes, audits, receipts and confirmation invalidation
commit atomically under the round lock with current authority and session checks.
An actual change clears confirmation even when points stay unchanged; no-ops
and receipt replay preserve it. Completed cards remain correctable until locking.

Native points, actual strokes and overall equivalents have explicit versioned
value representations. Stableford entries do not put equivalents into legacy
stroke-total fields. Mixed overall entries expose labelled comparison totals;
stroke-only responses retain their prior shape. Private runtime decoders verify
both arithmetic and the result type against configured rounds. A hidden or draft
Stableford round still establishes the configured overall value basis.

The server projects permitted holes before deriving visible points, progress and
metadata, retaining the full 18-hole handicap allocation. Hidden completed finals
remain excluded from overall standings. Public links keep the existing summary
scope and expose only the selected converted value and permitted tie-break, never
player cards, input revisions or handicap details.

## Scoring and offline behavior

The mobile card shows numeric/pickup states, received strokes and server-confirmed
points. Pickup has explicit confirmation and keyboard focus restoration. Setup
and history explain the conversion rule. Stale draft settings reload current
values with an explicit replacement notice.

The device queue adds `stableford_v1` with player ownership, immutable heads and
acknowledged-revision successors. Existing numeric and four-ball serialized heads
are preserved. Conflicts show local and current numeric, pickup or blank states.
Unknown delivery survives reload and two tabs. Pending delivery, verification or
failed local storage blocks confirmation; retry/discard and navigation protection
remain available. Confirmation uses an atomic player-card lease and fresh online
read. Visual review found and corrected overlapping point labels on small screens;
rectangle checks now cover that failure as well as page overflow.

## Validation

- Backend: **196 standard tests**, formatting and all-feature Clippy with warnings
  denied passed. Changed backend production files remain below 400 substantive lines.
- PostgreSQL: the full database-feature ladder passed **528 tests across 54 suites**,
  including **19** focused Stableford cases. Fresh migration, populated schema-28
  upgrades preserving both older scoring protocols and receipt replay, and seeding
  twice passed on disposable PostgreSQL 17.11. Tests cover retained identity/ABA,
  direct SQL guards, current authority, receipt contention/late expiry, rollback,
  zero-point completion, same-points correction invalidation and hidden full-JSON
  noninterference. Mixed domain cases cover independent best-N, mandatory slots,
  once-per-partner four-ball attribution and final Stableford outside best-N.
- Frontend: **553 tests across 96 files**, strict typecheck, lint and production
  build passed. A final focused decoder run passed **49 tests**, including independent
  contribution-format mismatch and configured-value-basis regressions. Legacy
  arithmetic and immutable queue compatibility checks remain intact.
- Real Chrome: **22 distinct scenarios passed** across combined and focused runs:
  nine Stableford, six four-ball and seven legacy offline cases. Checks used
  320x600, 390x844 and 1280x900, covering creation/settings, loading/error/empty/
  populated/long-content states, pickups, both conflict choices, lost acknowledgements,
  two tabs, failed local storage, confirmation leases, completed corrections,
  locked history and hidden private/public results. All-draft tournaments and
  completed stroke rounds alongside draft Stableford also passed actual API
  decoding and overall rendering at all three widths. Keyboard focus, 44px controls,
  label/value non-overlap, overflow and console/network assertions passed with
  deliberate test failures accounted for.
- Read-only backend, queue/API and result/UI reviews have no open material findings.
  Review corrected two misdirected concurrency assertions and strengthened decoder
  rejection tests so they exercise the intended guards independently. Final review
  also caught draft rounds missing from the repository value-basis decision;
  loading all configured facts fixed it while existing status filters preserve
  contribution eligibility. Private API/public and Chrome regressions cover it.

Backend logs use `/tmp/stableford-{unit,full-db-final,db-final,clippy}.log`.
Frontend evidence uses `/tmp/sf-tests-final`, `/tmp/sf-final-decoder`,
`/tmp/sf-browser-all`, `/tmp/sf-browser-final`, `/tmp/sf-browser-last`,
`/tmp/sf-browser-draft`, `/tmp/sf-draft-db` and
`/tmp/sf-{typecheck,lint,build}`. Focused reruns resolved earlier harness failures;
these temporary artifacts are not published. Screenshots use
`/tmp/golf-offline-stableford-*-{320,390,1280}.png`.

## Scope and readiness

**READY:** individual Stableford is implemented, reviewed and validated. The build
retains the existing chunk-size warning: main JavaScript 710.55 kB, 203.10 kB gzip.
Performance work remains queued. This format is 18-hole only; cold offline launch,
background sync, queued confirmation and handicap-system submission remain outside
scope. Match play has its calculation foundation but is not yet playable.
