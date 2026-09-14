# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. The application review is complete. **H1 is the next repair candidate**;
implement one finding at a time after instruction to proceed. Findings and
reproduction evidence are recorded in [LatestExplanation.md](LatestExplanation.md).

## Priorities

| Priority | ID | Finding | Repair order |
| --- | --- | --- | --- |
| High | H1 | Unsaved scoring input disappears after lock or access denial | 1 |
| Medium | M1 | Legacy score save/confirmation can commit after session expiry | 2 |
| Medium | M2 | Cached private results remain visible after a denied refresh | 3 |
| Low | L1 | Generic handicap allocator mishandles `i32::MIN` | 4 |
| Low | L2 | Course selection reports an incorrect or generic format error | 5 |
| Low | L3 | Match-only results remove tournament selection | 6 |

High means loss of the only local copy of user input. Medium means an authority
or private-display contract fails under a concrete transition. Low means a
bounded domain edge outside current snapshot inputs or recoverable UX/error
quality. Severity reflects demonstrated impact, not the amount of code to change.

## High — H1: preserve unsaved scoring input through authority changes

**Goal:** preserve the only copy of a failed local score edit until the user
explicitly discards it or it is safely stored as a clearly pending device copy.

**Trigger and evidence:** a device-storage failure leaves a correction only in
component state. An external lock changes the scoring/read query or an access
error replaces the scoring screen. The editable component unmounts, losing the
value and clearing its navigation guard. Reproduced in Chrome with a completed
individual stroke card: server 4, failed local correction 5, empty durable queue,
then external lock removed recovery and enabled logout. Shared source paths affect
individual stroke, scramble, foursomes, four-ball and Stableford.

**Scope:** `frontend/src/features/scoring/useScoreWorkspaceData.ts`,
`frontend/src/pages/ScorePage.tsx`, the legacy/four-ball/Stableford sync hooks and
their editable experience/recovery components. Keep match recovery compatible.
No backend contract or stored queue-protocol rewrite.

**Required behavior:** retain account/round/owner/player/hole-scoped unsaved numeric
or pickup intent through lock, scoring denial and background metadata failures.
When authority is denied, show only the local intent and recovery controls;
do not keep unauthorized canonical score data visible. Keep navigation guarded
until explicit discard or a valid durable save. Retrying failed device persistence
may work offline with the same account, target and original conditional expectation;
it must not invent authority or silently rebase. After an explicit lock or denial,
keep local-only recovery. Resuming server delivery requires fresh authorization
and canonical conflict review; never replay automatically into a locked round or
silently overwrite another scorer. Preserve failed values independently for both
four-ball partners. Real account changes must not expose the previous account's
intent. Ordinary logout remains guarded while nondurable data exists.

**Invariants:** exclusive score ownership; preserved historical handicap snapshots;
locked-round integrity; no match ledger changes; immutable existing queue heads,
receipt identities and account isolation; no false claim of device durability.

**Validation:** add failing regression tests before repair for failed IndexedDB
write followed by external lock, scoring 403 and background metadata failure.
Cover the legacy shared scorer plus four-ball and Stableford numeric/pickup paths;
verify offline retry of local persistence, recovery and explicit discard,
navigation guards, account isolation and absence of unauthorized server data. Run the complete frontend ladder and
real Chrome at 320x600, 390x844 and desktop widths. Re-run affected existing offline
and match recovery cases against the isolated API. Update affected documentation.

**Stop:** publish only H1 after review and validation; do not include M1/M2 or a
queue redesign. Any newly found concern returns to this queue.

## Medium — M1: recheck session validity before legacy commits

**Trigger:** compatible `PUT /api/rounds/{id}/scores` and scorecard `POST /confirm`
check the session before waiting for a membership lock. They then mutate and
commit without checking wall-clock expiry again. Both operations were reproduced
against PostgreSQL/API: a session expired after 1.5 seconds, the membership gate
held for four seconds, and the request returned 200 with persisted effects.

**Scope and repair:** `backend/src/repositories/scorecards/mutations.rs` and its
focused API/repository tests. Recheck the live session immediately before commit
for both authenticated paths, including no-op responses, using established error
mapping and transaction boundaries. Preserve the compatibility endpoints, ordinary
score revisions, audit/confirmation behavior and newer receipt replay contracts.

**Validation:** adapt the conditional-delivery membership-wait regression to
legacy save and confirmation. Hold the membership row, begin while valid, release
after expiry; expect 401 and zero new score/audit/confirmation effects or SSE event.
Cover unchanged-value save and repeat confirmation, valid sessions and the existing
conditional/four-ball/Stableford/match paths. Run backend and PostgreSQL ladders.
**Stop:** only these legacy commit checks and their tests/docs are in this step;
a wider authentication audit remains separately queued.

## Medium — M2: hide private projections after authoritative denial

**Trigger:** direct scorecards, player history and round results keep cached data
when a refetch returns 401/403/404. A healthy SSE connection need not immediately
clear the cache. Chrome reproduced a direct card still displaying gross 7 after
its card endpoint returned 403.

**Scope and repair:** `frontend/src/pages/DirectScorecardPage.tsx`,
`PlayerHistoryPage.tsx`, `LeaderboardPage.tsx` and focused private-query/error
helpers. Distinguish authority denial from transient network/server failures.
Immediately suppress and invalidate/remove affected private projections on denial;
a retry may render data only after current authorization succeeds. Preserve
permitted transient-error recovery and H1 local-only input recovery. Inspect the
same shared rendering helper's consumers without broad UI refactoring.

**Validation:** successful card/history/round-result read → denied background
refetch while SSE stays healthy; prior names, scores and links disappear. Exercise
401/403/404 separately, successful recovery and transient 500/offline behavior.
Keep hidden-final and same-user session refresh behavior intact. Run the frontend
ladder and representative mobile/desktop Chrome privacy cases.
**Stop:** fail-closed private rendering only; no new public scope or SSE redesign.

## Low — L1: make the generic allocator safe at the signed minimum

**Evidence:** `backend/src/domain/scoring.rs:107` casts `unsigned_abs()` back to
i32. For `i32::MIN`, 18 allocations sum to +2,147,483,646 instead of −2,147,483,648;
one-hole debug allocation panics. All 65,536 i16 handicaps conserved their expected
18-hole sum in the review probe, and production snapshots use i16. This is a
latent helper-contract defect, not an observed current score-result failure.

**Scope and repair:** retain a widened/unsigned magnitude until the signed result
is representable, or return a typed overflow error. Preserve allocation order and
legacy signed-rounding behavior. Do not change historical snapshots or broaden
this into general handicap-policy work.
**Validation:** test i32 minimum/maximum, ordinary plus handicaps, 1/9/18-hole
allocation, invalid arguments, sum conservation and debug/release parity. Run the
backend ladder and relevant scoring-format regressions.
**Stop:** allocator boundary and direct tests only.

## Low — L2: report the correct 18-hole course requirement

**Evidence:** `backend/src/repositories/round_configuration.rs:85` returns
`InvalidFourBallLayout` for Stableford too; the API reports “four-ball requires
18 holes.” Match play is absent from that explicit validation and reaches a
generic constraint response. PostgreSQL still rejects invalid layouts.

**Scope and repair:** introduce a format-aware layout error across the repository,
API mapping and affected frontend error mapping. Reject non-18-hole selections
for four-ball, Stableford and singles before attempting round configuration;
preserve transaction rollback and permitted 9-hole legacy formats.
**Validation:** submit nine-hole manual and relevant saved/provider tee selections
for each supported 18-hole format, assert actionable correct errors and unchanged
round/course configuration. Run affected backend/PostgreSQL ladders; run frontend
checks/browser validation if user-facing mapping changes.
**Stop:** format-specific validation/error quality only; no course-provider expansion.

## Low — L3: retain tournament selection in match-only results

**Evidence:** `frontend/src/pages/LeaderboardPage.tsx:87` returns `MatchResults`
before the shared tournament selector. Its navigation links remain within that
trip. A multi-tournament account must leave global results to switch away from a
selected match-only tournament.

**Scope and repair:** keep the global tournament selector above the format-specific
result body. Preserve match-only query suppression, private player scope, mixed
overall links and route canonicalization.
**Validation:** an account with match-only and stroke/mixed trips switches in both
directions from `/leaderboard`, including direct entry and back/forward navigation,
at mobile and desktop widths. Run the frontend ladder and focused Chrome checks.
**Stop:** selector/discovery repair only, with no standings redesign.

## Later queue

1. **Performance work:** measure representative workloads before scoping changes,
   including the existing frontend bundle warning and bounded match-card reads.
2. **Security review:** perform the separately scoped wider application/operational
   review after the above correctness repairs.

No automatic opponents/byes/brackets, team match play, extra holes, new scoring
rules, public match sharing, cold offline launch or background sync is included.
