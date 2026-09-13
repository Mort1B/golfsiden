# Plans

`PLANS.md` contains only current and queued work. Completed behavior belongs in
`Documentation.md`; durable technical decisions belong in `ARCHITECTURE.md`.

## Active step — singles match-play playable integration

**Status: reviewed implementation step; awaiting implementation instruction.**
Implement after an explicit instruction to proceed with this bounded step. The
existing [singles contract](ARCHITECTURE.md#planned-singles-match-play-contract)
is the product baseline; this plan makes its release boundary concrete.

### Goal and scope

Make 18-hole singles playable from creation and manual opponent assignment through
online reporting, confirmation, completion and private match history. Add the
separate 1/½/0 match-points table and durable offline numeric notes. Deliver one
coherent format; do not expose it in creation before every path is integrated.

Reuse `domain/match_play` for relative handicap allocation, contiguous outcomes,
terminal results and exact half-point units. Add dedicated match persistence,
transport and UI; do not represent opponents as teams or wins as stroke scores.
Existing stroke, four-ball and Stableford behavior, historical snapshots, result
shapes and persisted offline operations remain compatible.

### Setup, snapshots and overall configuration

- Add `singles_match_play` to exhaustive format boundaries. Exact tournament admins
  assign two distinct active entrants per round-local match, in the same flight.
  Every active entrant must have exactly one opponent before opening; odd counts,
  duplicates, missing assignments and cross-flight opponents block opening.
  No automatic pairings, byes or absent-player wins. Other formats retain their
  administrator-managed teams and existing readiness rules.
- Require the round's shared 18-hole tee and flights starting at hole 1 (unset is
  1). Draft settings select one official gross/net mode, default net, with fixed
  100% allowance. Keep one authoritative mode: map the existing handicap-enabled
  setting to gross/net rather than storing contradictory independent switches.
  Opening freezes opponents, tee, mode and individual Playing Handicaps using the
  new-format signed rounding policy. Net allocation is the full relative
  difference, widened before subtraction; gross allocation is zero.
- Distinguish scheduled round count from configured overall-eligible count.
  A match never counts toward gross/net best-N, provisional selection or mandatory
  eligibility. Mixed tournaments require N within eligible formats and cannot
  choose a match as mandatory. Match-only configuration uses explicit
  `counted_rounds: null`, `mandatory_round_id: null`; never N=0. Creation, onboarding,
  draft edits, start validation, database constraints, decoders and all summary
  consumers must agree. Existing non-match tournament payloads retain numeric N.
  Preserve configuration timestamp/no-op and start/open/snapshot freeze semantics.
  Cross-table eligible-format constraints must support the existing transaction
  inserting tournament before rounds (defer consistency checks until commit).
- Private overall GETs for match-only tournaments return a typed not-applicable
  result after normal authorization, distinct from empty results or an error.
  UI avoids fetching unavailable overall data and explains the separate table.
  Mixed overall responses preserve existing stroke/Stableford value contracts,
  including the configured Stableford basis even while that round is draft.
  Current overall selection considers only open eligible formats; a later open
  match cannot displace an open stroke round. Apply eligibility in repository
  current-visibility selection and frontend cross-round validation too; exclude
  match facts from the ordinary round-result builder while retaining all configured
  formats for value-basis and final-round identity. Final-round tie-breaking and hidden
  final policy still refer to the actual final scheduled round. A final match
  provides no comparable stroke tie-break; shared places remain shared.

### Match aggregate, commands and authority

- A forward migration adds match assignments, player-owned numeric notes,
  append-only report/correction audit, current aggregate revision, confirmation
  and account-scoped command receipts. Enforce tournament/round/player identity,
  distinct opponents, one match per entrant per round and lifecycle restrictions
  in PostgreSQL. Preserve schema-29 data and all existing receipts unchanged.
- Serialize every match mutation through parent round then match locks. Check
  current session, membership, scope, expected aggregate revision and lifecycle
  in the same transaction. Recheck session validity after waits and before commit.
  Revisions are positive integers serialized as decimal strings. Payloads are
  typed and reject unknown/mixed variants; stable request UUID plus exact payload
  identifies an immutable account-scoped receipt. An authorized exact replay
  returns its original acknowledgement even after terminal/lock; a mismatched
  reuse or fresh stale command conflicts without partial effects.
- Numeric-note commands target one opponent and hole; values use the existing
  validated numeric range, with an explicit clear-note variant for correction of
  unfinished notes. A blank never proposes a loss or halve. A note change advances
  the match revision but does not rewrite any accepted outcome. Ordinary notes
  cannot be changed after terminal state or round lock.
- Online report commands resolve the next contiguous hole only. They record the
  agreed winner/halve and a typed basis: both completed numeric scores;
  communicated next-stroke concession with counted gross score and named actual
  conceder; communicated hole concession; agreed halve with play-begun and mutual
  agreement attestations; or exact-admin organizer ruling with reason. Numeric
  comparison is a proposal, never automatic acceptance. Preserve the numeric
  evidence used at acceptance so later note edits cannot alter its meaning.
- Whole-match concession is an explicit terminal command naming the actual
  conceder and attesting communication. Only an exact admin can record an
  organizer award, with a reason. Recorder authority does not authorize inventing
  a concession for another player. Ordinary reports/concessions/confirmation use
  existing exact-admin/scorer or own-flight player scope with current membership;
  require write authority for both opponent records. Viewers and outsiders fail.
- Store actor, basis, request identity, revision, timestamp and effective point in
  the played sequence for accepted facts. A terminal concession before hole 1 is
  valid and creates no fictitious hole scores. Effective point must be validated
  against the current accepted sequence, not freely chosen to bypass visibility.
  Reject gaps, duplicate or post-finish reports. Strict lead greater than holes
  remaining ends the match; equality does not. Tied after 18 is a draw.

### Confirmation and corrections

- One online match confirmation covers both opponents. Require terminal state,
  current aggregate revision, fresh authority and explicit agreed/awarded-result
  attestation. Award exact 2/1/0 half-point units only after confirmation. Round
  completion requires every assigned match confirmed and terminal; unplayed holes
  after an early finish do not create missing-score obligations.
- An exact-admin, reason-required correction command explicitly replaces the
  affected accepted ledger and names superseded reports/events. Validate the
  complete replacement, preserve history, and atomically invalidate confirmation
  and awarded points. Never resurrect incompatible later reports or silently
  recalculate agreement from numeric edits. Distinguish recording error from an
  organizer ruling; never label it withdrawal of a real concession.
- Corrections may reopen a terminal match. Require newly necessary reports before
  reconfirmation. Locked rounds reject every ordinary write. Their explicit
  audited admin correction workflow must also support resolving the replacement
  ledger and reconfirming it, without unlocking ordinary scoring. Parent lifecycle
  is retained; expose pending corrected results and withhold points until valid
  reconfirmation, including previously completed tournaments. Fresh correction and
  confirmation commands use expected revisions and immutable receipts too.
- Terminal reports, notes, confirmation, completion, lock and correction must
  serialize predictably. SSE invalidates match workspace, readiness, match table
  and affected history only after commit; payloads expose no hidden outcome.

### Private results, public compatibility and offline UI

- Add dedicated typed read/scoring projections and a private match table with
  exact points, played and W/D/L counts. Sum all confirmed permitted matches;
  rank by points descending with shared places and deterministic display order.
  Players without confirmed matches remain unranked. Match-only player history and
  scoring discovery must resolve entrants without depending on overall entries.
  No best-N, margin bonus,
  provisional draws or bracket progression. Label each match's frozen mode.
- Private member/history reads retain repeatable-read authorization and membership
  `FOR SHARE` through assembly, no-store headers and existing 401/403/404 behavior.
  Restricted reads derive solely from permitted reports/events before calculating
  lead or metadata. Hidden basis suppresses winner, finish, confirmation and
  points. Until final release, exclude the entire hidden final from non-admin
  match-table totals/ranks/counts, even if one match finished on the front nine.
  No full aggregate revision, timestamps or audit fields may leak hidden changes
  through read-only projections. Authorized scoring reads retain established scope.
- Existing public links expose only overall gross/net summaries. No public match
  table, opponent history or cards. Reject share creation for match-only tournaments
  with an explicit unavailable state; mixed public results retain format exclusions
  and final-round protection. Do not change existing public field allowlists.
- On an already loaded match card, a separately tagged/versioned durable queue
  stores player-owned numeric notes only. Isolate by account and match; preserve
  all legacy/stableford/four-ball serialized heads and fingerprints. Immutable
  queued commands use expected aggregate revision, explicit predecessor-linked
  successors and receipts. Synchronize one match sequence across both opponents;
  acknowledgement of an unrelated head must not silently rebase another draft.
- Reconcile lost acknowledgement before successors; stale/terminal/locked drafts
  remain visible for explicit old/local/server review. Never silently discard or
  replay them after a finish. Online reports, concessions, awards, corrections and
  confirmation require a fresh canonical read and an atomic local match-wide lease
  covering both opponents' notes and in-flight commands. Failed storage, unknown
  delivery or any pending edit blocks those actions. Release the lease on failures
  and identity changes; server revisions remain authoritative across devices.
- Explain connectivity limits before play. Mobile UI separates numeric proposals,
  accepted reports, terminal results and confirmed points, with explicit concession
  and correction attestations. Preserve failed/queued navigation protection and
  retry/discard choices. Cover setup, own/flight scoring discovery, gross/net mode,
  hidden read-only history, match-only/mixed standings and public unavailable states.

### Ownership, validation and stop condition

Primary owns integration, scope, documents and publication. If specialists help,
use one writer at a time: backend owns domain/repository/API/migration/tests;
frontend then owns strict contracts, queue, setup/scoring/results UI and tests.
Read-only reviews cover scoring, authority, privacy, concurrency and compatibility.
Split production files before 400 substantive lines; do not add a generic match
framework or expand existing modules past their responsibilities.

Run the full backend, PostgreSQL and frontend ladders in
[AGENT_WORKFLOW.md](AGENT_WORKFLOW.md#validation-ladders). Verify fresh migration,
populated schema-29 upgrade and seed twice against disposable PostgreSQL. Include:

- Relative allocations 10/18, −2/14, −4/−1, 0/40 and i16 extremes; signed snapshot
  rounding; gross mode; 3&2, dormie/equality, 18-hole draw and pre-hole concession.
- No points before confirmation; win/draw/loss arithmetic; incomplete assignments;
  frozen opponents/mode; report provenance, missing numeric facts and explicit
  clear-note behavior; late numeric edits preserving accepted evidence.
- Own-flight/admin/scorer/viewer authorization, revoked membership/session after
  waits, direct SQL guards, receipts/no-op replay, lost ACK, rollback of audits and
  points, and races among notes/reports/terminal/confirmation/correction/lock.
  Include locked correction reopening and reconfirmation without ordinary access.
- All-match, all-draft match, mixed and multiple-open-round contracts; N and
  mandatory validation; final scheduled match ties; no match contribution in any
  overall/private/public projection; unchanged old-format serialized responses.
- Paired hidden fixtures with identical permitted prefixes but different hidden
  reports, terminal events and corrections: byte-equivalent permitted JSON and
  identical visible match-table totals/ranks/counts. Cover before-hole-1 events,
  front-nine finishes, hidden effective points and final release.
- Strict decoder rejection and durable queue compatibility; cross-opponent
  successors, two tabs, account isolation, storage failure, unknown delivery,
  match-wide leases, terminal blocked drafts and fresh canonical verification.
- Real Chrome at 320x600, 390x844 and 1280x900: loading/error/empty/populated/long
  content, manual setup, notes/reports/concessions, early finish, draw, correction,
  confirmation, offline/reconnect, match-only/mixed/private/public views, keyboard
  and focus restoration, 44px controls, overflow and console/network assertions.

Update affected Architecture, Documentation, LatestExplanation and deployment
procedures in the implementation step, recording exact evidence and any blockers.
Stop only after the coherent format passes review/validation or explicitly report
NOT READY; never publish a selectable partial format. On completion remove this
active step, commit only scoped files, push `main`, verify clean `HEAD == origin/main`,
and stop before the queue below. This planning iteration changes `PLANS.md` only;
review and publish the plan without enabling match play or claiming release readiness.

## Later, as separate bounded steps

1. **Code review:** Review the application across completed formats, including the
   generic stroke allocator's `i32::MIN` edge, unreachable through new foundations'
   i16 snapshot boundaries.
2. **Performance work:** Measure representative workloads and scope changes from
   findings, including the existing frontend bundle-size warning.
3. **Security review:** Review the resulting application and address findings in
   separately bounded steps.

No automatic opponents/byes/brackets, team match play, extra holes, 9-hole mode,
handicap-system submission, rules adjudication, public match sharing, cold offline
launch, background sync or offline authoritative reporting is included.
