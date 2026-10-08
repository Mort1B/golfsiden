# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

**FANTASY-1 — finalize the Fantasy rule and lifecycle contract (documentation only).**
Started on user instruction. This bounded step finalizes the contract; it does
not change runtime code. Product choices about match scoring and missed lineups
have been presented to the user and remain pending until answered.
See the [planned contract](ARCHITECTURE.md#fantasy-competition-design-planned-not-implemented).

- Goal: make every scheduled round and incomplete-result state unambiguous before
  writing persistence or user-facing code.
- Scope: this plan, the planned architecture contract and its acceptance examples;
  no source, migrations, dependencies, deployments or runtime validation.
- Confirmed: four freely selected golfers each round, one captain doubling the
  entire result, net scoring, shared team results for both partners, all-round
  Fantasy total, fixed placement table, actual ace/net albatross-or-better +10,
  triple bogey -3, quad-or-worse/pickup -5, non-finishers keep recorded hole points.
  Early-finished matches retain earned points without additions or penalties for
  unplayed holes. Include a golfer points leaderboard for each round and overall,
  showing base Fantasy points before any manager's captain multiplier.
- Pending user decisions: match outcome/placement/net/concession rules and
  invalid-at-deadline/missed-lineup policy. Match rounds must not be silently
  excluded from “all rounds.” Lifecycle defaults and the Fantasy-only non-finish
  disposition boundary are defined in the architecture contract.
- Invariants: Fantasy stays separate from sporting results, uses preserved
  handicaps, never fabricates scores, respects privacy and cannot mutate locked
  sporting facts. Distinguish manager accounts from selected golfer identities.
- Validation: check source compatibility, arithmetic examples, decision status,
  relative links and whitespace; independent read-only contract review.
- Stop: record agreed examples and unresolved blockers honestly; only queue
  FANTASY-2 when the contract is resolved. Do not implement in this planning step.

## Fantasy implementation queue

Execute one bounded step after a separate user instruction; this is sequencing,
not permission to run the whole queue. Revalidate the checkout and detailed scope
at each intake. All steps retain the contract's all-round requirement and the root
invariants; a partial foundation must never be advertised as a playable feature.

1. **FANTASY-2 — pure scoring and typed result states.** Own `domain/fantasy` and
   unit tests. Implement agreed net categories/ace precedence, placement lookup,
   shared-team attribution, captain multiplication, all-round summation and ties;
   model pending/pickup/DNF/withheld explicitly. Include the resolved match policy.
   Validate arithmetic, signed handicaps, negative totals and every contract
   example with backend unit/format/Clippy checks. Stop at a deterministic tested
   domain boundary: no new routes, database tables or UI.
2. **FANTASY-3 — game and selection persistence/API.** Add forward migrations,
   repositories and handlers for exact-admin enable/rules/deadlines and member
   entries/four picks/captain, preserved Fantasy handicap inputs and revision-bound
   admin non-finish dispositions with owner-wide source generations maintained by
   relevant score/confirmation writes. Enforce complete atomic lineups, tenant identity,
   revisions/replay, pre-lock secrecy and deadline/opening serialization. Respect
   claiming/withdrawal and revoked sessions. Validate PostgreSQL fresh/upgrade,
   repeated seed, direct constraints, API authority and controlled lock races,
   plus the full backend ladder. Stop with reviewed usable selection contracts;
   update architecture/current API/deployment docs, no Fantasy UI yet.
3. **FANTASY-4 — authoritative round and total projections.** Add bulk format fact
   adapters and private manager and golfer round/overall/breakdown APIs. Include
   all tournament golfers regardless of selection; never apply captain multipliers
   to the golfer board. Use raw score states and preserved net calculations,
   including team ranking once before attribution.
   Implement agreed finality/DNF/match/correction handling, including independent
   Fantasy settlement and finisher-only placement, and privacy projection
   before every aggregation. Integrate post-commit invalidations. Test 9-team and
   variable fields, all formats/rounds, hidden-result noninterference, correction
   recomputation and unrelated sporting regression; run backend/database ladders.
   Stop at reviewed result contracts with no fabricated/pending-as-zero results.
4. **FANTASY-5 — complete mobile-first Fantasy UI.** Add strict runtime decoders,
   user/tournament-scoped queries, tournament navigation, admin setup/non-finish
   settlement, My Four and
   captain/deadline submission, manager and golfer round/overall standings and
   explanations. Include Fantasy in score/match/visibility invalidation and
   return/session clearing.
   Handle loading/error/empty/populated/long content, stale edits, denied access,
   uncertain saves, expired deadlines and missed/withheld results. Validate full
   frontend ladder plus real Chrome at 320/390/1280 and API-backed flows. Stop with
   a usable complete game, not a mock or disconnected picker.
5. **FANTASY-6 — release acceptance and documentation.** Exercise a multi-round
   tournament with 9 two-person teams plus smaller/larger fixtures, changing
   partners/picks/captains and actual/net ace distinctions. Test the whole lifecycle,
   early-finished matches retaining earned points, golfer totals independent of
   captain/selection, missed/invalid lineups, authorized corrections, hidden final
   release and return freshness. Run every affected full ladder and browser flows,
   migration/upgrade and permission refresh checks in disposable environments. Resolve read-only
   review; publish evidence, operator flow and readiness/limitations. Stop with a
   clean main aligned to origin; production deployment remains user-owned.

Every implementation step updates affected durable docs and LatestExplanation,
removes its completed plan item, commits only its scope and stops before the next.
No budgets, transfer penalties, auto-picks, sporting team generation, public Fantasy
sharing, prizes/payments or offline queued selection writes are in this plan.

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
