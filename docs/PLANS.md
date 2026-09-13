# Plans

`PLANS.md` contains only current and queued work. Completed behavior belongs in
`Documentation.md`; durable technical decisions belong in `ARCHITECTURE.md`.

## Active step

### Stableford playable integration

**Status:** Proposed implementation step; planning and review only until approved.
The existing calculation foundation is complete. This step makes the approved
[individual Stableford contract](ARCHITECTURE.md#planned-individual-stableford-contract)
playable as one coherent release. It does not reopen the format's product rules.

**Goal:** An organizer can configure an 18-hole individual Stableford round;
authorized players can record numeric strokes or explicit pickups, synchronize
edits, confirm cards, complete the round, and see native points and correctly
labelled mixed-format overall contributions through private history and existing
public overall summaries.

#### Scope and ownership

- Backend domain: integrate `domain/stableford/` and shared player input types
  through the closed format policy, creation/readiness, opening snapshots,
  player-card completion, round ranking and tournament contribution selection.
  Arithmetic remains in the domain; handlers only validate/map typed transport.
- Persistence/API: add forward migrations after schema 28 for player-owned
  Stableford inputs, retained versions, audits and immutable delivery receipts.
  Dedicated Stableford storage and a versioned conditional command boundary keep
  existing numeric and four-ball records and request fingerprints unchanged.
  Reuse helpers only where ownership, authority and lifecycle semantics match.
- Frontend: align format setup, typed API/runtime decoding, scoring and read-only
  history, offline delivery, confirmation, native round standings and labelled
  private/public overall views. Extend the queue with a distinct Stableford
  protocol; do not route a player card through four-ball team ownership.
- Primary agent owns integration, plan state, documentation and publication.
  Backend/persistence and frontend implementation run sequentially. Read-only
  review covers the cross-layer contracts, privacy and concurrency.

#### Required behavior and invariants

1. **Configuration and snapshots:** Stableford is individual, with no required or
   inferred team, one shared 18-hole tee and existing flight authority. Reject
   non-18-hole configuration. Default allowance is 100%, configurable from 0–100%
   while draft. Freeze the uncapped tournament handicap, tee and allowance at
   opening. Apply allowance to unrounded Course Handicap, then round once with
   exact halves toward positive infinity. Disabled handicaps allocate zero;
   plus handicaps return strokes on the highest indexes. Preserve legacy rounding.
2. **Input and delivery:** Store actual numeric gross strokes or explicit
   `Plukket opp / ingen score`; absence remains unentered. Never fabricate strokes,
   delete/recreate an input on pickup, or accept client-calculated points.
   Numeric/pickup/numeric transitions keep UUID identity and advance positive
   revisions with audits. Conditional writes use exact expected versions and
   account-scoped immutable receipts. Recheck current authority, session and round
   state after lock waits, including replay; locked rounds reject ordinary writes.
   Input, audit, receipt and confirmation invalidation commit atomically. Existing
   numeric-only formats continue to reject pickups.
3. **Completeness and confirmation:** Each numeric or explicit pickup resolves
   one hole. Eighteen resolved holes permit online confirmation even at zero
   points; blanks are never auto-filled. Require a fresh authorized card, no
   pending/unknown delivery or verification, and an atomic cross-tab player-card
   lease. Actual changes clear confirmation even when points do not change;
   no-ops/replays preserve it. Retain open/completed corrections and explicit
   lifecycle transitions. Completion requires every required player card resolved
   and confirmed; locking still requires current confirmation.
4. **Points and units:** Use the tested gross/net formula and signed per-hole
   handicap allocation; no six-point ceiling or net-stroke floor. Pickups earn
   zero in both metrics; blanks have no result. Rank native points descending with
   shared positions, keeping unstarted players unranked. Only 18 numeric holes
   yield actual full stroke totals. Keep actual strokes, native points and overall
   equivalents distinct in domain types, transport, decoders and labels; never put
   an equivalent in an actual-stroke field. Preserve stroke-only response meaning
   with a discriminated/versioned result contract and strict decoder regressions.
5. **Overall selection:** Complete cards contribute `36 - points` independently
   for gross/net; permitted partial cards use `2 * resolved_visible_holes - points`.
   No resolved visible hole means no contribution. Best-N selects the lowest
   equivalents; preserve mandatory slots, completed-only qualification, the
   highest-numbered-open provisional round and once-per-player round attribution.
   Final-round tie-breaks compare completed visible equivalents, including finals
   outside best-N. Preserve all existing formats and hidden/incomparable ties.
6. **Privacy and live data:** Authorize membership-private reads and assemble them
   in one repeatable-read transaction with membership share locking and no-store
   responses. Project permitted holes before deriving points, progress, completion,
   confirmation metadata and tie explanations, retaining full-layout allocation.
   Exclude hidden completed finals under existing policy. Hidden back-nine edits
   must not change non-admin/private or public result projections. Public sharing
   adds only the necessary non-private value-basis discriminator/label to its
   existing overall summary, never private cards, hole states or identifiers.
   Emit existing targeted invalidations only after commit and refetch canonical
   data before showing points as server-confirmed.
7. **Offline and UI:** Persist numeric/pickup intent before delivery; retain
   immutable heads, acknowledged-predecessor successors, account isolation,
   explicit conflicts/discard and unknown-delivery reconciliation. Conflicts show
   actual blank/numeric/pickup states. Preserve existing numeric and four-ball
   serialized queues and receipts exactly. Retain retry/discard and navigation
   protection on storage failure. Show original input, received strokes, gross/net
   points, resolved progress and labelled overall equivalents. Pickup needs an
   explicit confirmation; gross zero points must never imply net zero or recommend
   pickup. Loading, errors, pending data and read-only/locked states stay usable.

All root product invariants remain mandatory. Teams remain administrator-managed;
historical calculations use preserved snapshots, never current handicaps.

#### Validation and review

- Run the complete backend and frontend ladders in `AGENT_WORKFLOW.md`, plus the
  complete database-feature ladder on disposable PostgreSQL. Exercise fresh
  migration, a populated schema-28 upgrade retaining legacy/four-ball data and
  receipts, and seed twice. Test direct SQL guards as well as API failures.
- Cover retained identity through numeric/pickup/numeric ABA, lost acknowledgements,
  concurrent receipts, stale versions, rollback, membership/session/round-lock
  races, and scoring versus confirmation/completion. Include numeric 9→10 with
  unchanged zero points and true no-op/replay confirmation preservation.
- Cover blanks versus 18 pickups (zero points, +36 equivalent), 18 pars (36/0),
  40 points (−4), plus/disabled/boundary allowances, and the contract's six-hole
  example (6/9 points, +6/+3 equivalent). Nine visible holes with 20 points yield
  −2. Test partial/incomplete/completed cards and absent actual totals with pickups.
- Exercise mixed stroke/Stableford/four-ball best-N, mandatory and final-round
  ties; native descending points and unstarted versus zero-point ranking; private
  history and public labels; hidden-input noninterference including metadata.
- Preserve legacy serialized queue heads/fingerprints and receipt replay through
  upgrade. Cover two tabs, account switching, predecessor chains, pickup conflicts,
  unknown delivery, fresh-read cancellation, storage failure and confirmation leases.
- Use real Chrome at 320x600, 390x844 and 1280x900 for setup, scoring, pickup,
  reconnect/conflicts, confirmation, completed corrections, locked history and
  private/public results. Include loading/error/empty/populated/long-content states,
  keyboard focus, 44px controls, overflow/navigation clearance, console and network
  assertions, and the existing offline regression suite.
- Obtain read-only plan and implementation reviews for correctness, authorization,
  migration safety, units, privacy and concurrency. Resolve material findings;
  record any unavailable validation with its exact blocker. Keep production files
  below 400 substantive lines and run diff/document-link checks.

#### Documentation and stop condition

Update `ARCHITECTURE.md`, `Documentation.md`, `LatestExplanation.md` and affected
deployment/migration guidance with implemented contracts and validation evidence.
Only expose Stableford once all setup, persistence, scoring, results and client
paths are coherent. After validation and review, close this active step, commit
only scoped files to `main`, push, and verify a clean worktree with
`HEAD == origin/main`. Report readiness and exact limitations, then stop.

Excluded: match-play integration, modified/team/nine-hole Stableford, per-player
tees, handicap submission or adjudication, new public drilldowns, cold offline
launch, background sync, queued confirmation, automatic teams, generic allocator
repair, performance and security-review projects. Those remain separate work.

## Later, as separate bounded steps

1. **Match-play playable integration:** Implement the approved singles format,
   draws after 18 holes and separate 1/½/0 match-points table without changing
   gross/net overall totals. Scope reporting, authority and confirmation explicitly.
2. **Code review:** Review the application across completed formats, including the
   generic stroke allocator's `i32::MIN` edge, which is unreachable through the new
   foundations' i16 snapshot boundaries.
3. **Performance work:** Measure representative workloads and scope changes from
   the findings, including the existing frontend bundle-size warning.
4. **Security review:** Review the resulting application and address findings in
   separately bounded steps.
