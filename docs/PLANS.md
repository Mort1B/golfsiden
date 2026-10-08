# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None.

## Next candidate

**FANTASY-4 — authoritative round and total projections.**
See the [Fantasy contract](ARCHITECTURE.md#fantasy-competition-design).

- Goal: expose current private Fantasy results from authoritative golf facts and
  the persisted selections, with separate manager and golfer standings.
- Scope: bulk format fact adapters, private round/overall/breakdown APIs, source
  and visibility integration, backend/database tests and affected documentation.
  Reuse FANTASY-2 domain rules and FANTASY-3 temporal selections/source tokens.
- Behavior: include every tournament golfer regardless of selection; captain
  multipliers apply only to manager totals. Apply preserved net calculations,
  team ranking once before attribution, outcome-only match awards, independent
  Fantasy settlement, finisher-only placement and correction recomputation.
  Project permitted facts before every aggregation; publish post-commit
  invalidation and label pending/provisional/settled/withheld states explicitly.
- Invariants: all-round totals; immutable locked picks; preserved handicap/team
  snapshots; no hidden-result inference through totals, ranks or metadata; no
  fabricated scores or pending-as-zero awards; no sporting lifecycle changes.
- Validation: 9-team and smaller/larger fields, every supported format and round,
  hidden-result noninterference, corrections and disposition staleness, bulk read
  behavior and unrelated sporting regressions; complete backend/database ladders,
  read-only review and documentation checks. Revisit first-close per-entry cost
  before claiming large-field capacity.
- Stop: reviewed authoritative result contracts and published validation evidence.
  No frontend, deployment or queued work in this step.

## Fantasy implementation queue

Execute one bounded step after a separate user instruction; this is sequencing,
not permission to run the whole queue. Revalidate the checkout and detailed
scope at each intake. A partial foundation is not a playable feature.

1. **FANTASY-5 — complete mobile-first Fantasy UI.** Add strict runtime decoders,
   queries scoped to user/tournament, tournament navigation, admin setup/non-finish settlement, My
   Four, carry-forward preview/origin and captain/deadline submission, manager
   and golfer round/overall standings and explanations. Include Fantasy in
   score/match/visibility invalidation and return/session clearing. Handle
   loading/error/empty/populated/long content, stale edits, denied access,
   uncertain saves, expired deadlines and missed/withheld results. Validate
   full frontend ladder plus real Chrome at 320/390/1280 and API-backed flows.
   Stop with a usable complete game, not a mock or disconnected picker.

2. **FANTASY-6 — release acceptance and documentation.** Exercise a multi-round
   tournament with 9 two-person teams plus smaller/larger fixtures, changing
   partners/picks/captains and actual/net ace distinctions. Test the whole
   lifecycle, early-finished matches receiving outcome-only points, golfer
   totals independent of captain/selection, carry-forward chains,
   missed/invalid lineups, authorized corrections, hidden final release and
   return freshness. Run every affected full ladder and browser flows,
   migration/upgrade and permission refresh checks in disposable environments.
   Resolve read-only review; publish evidence, operator flow and
   readiness/limitations. Stop with a clean main aligned to origin; production
   deployment remains user-owned.

Every implementation step updates affected durable docs and LatestExplanation,
removes its completed plan item, commits only its scope and stops before the
next. No budgets, transfer penalties, newly generated picks, sporting team
generation, public Fantasy sharing, prizes/payments or offline queued selection
writes are in this plan.

## Later queue

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

No automatic opponents/byes/brackets, new sporting scoring rules, team match play,
extra holes, public match sharing, cold offline launch or background sync is included.
Fantasy scoring is separately planned above; it does not alter sporting rules.
