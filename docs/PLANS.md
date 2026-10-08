# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. The next candidate requires a separate user instruction.

## Next candidate

**FANTASY-5 — complete mobile-first Fantasy UI.**
See the [Fantasy contract](ARCHITECTURE.md#fantasy-competition-design) and the
[implemented APIs](Documentation.md#fantasy-backend-screens-planned).

- Goal: make the complete private Fantasy game usable inside its tournament.
- Scope: strict runtime decoders, user/tournament-scoped queries, navigation,
  admin setup and non-finish settlement, My Four, manager and golfer standings
  and breakdowns, frontend tests and affected documentation.
- Behavior: submit four picks and a captain with deadline/revision handling;
  show carry-forward preview and locked origin; expose round and all-round
  manager/golfer boards with explicit pending, provisional, settled, withheld,
  future and nonparticipating states. Connect score/match/visibility and Fantasy
  invalidations, return refresh and session clearing. Handle uncertain saves,
  stale edits, expired deadlines, denied access and missed/invalid selections.
- Invariants: preserve pre-lock lineup privacy and hidden-result noninterference;
  server-authoritative deadlines, selections and scoring; captain multipliers
  only on manager contributions; all scheduled rounds count; no offline writes.
- Validation: full frontend ladder; real Chrome at 320/390/1280 widths with
  loading/error/empty/populated/long-content states and API-backed selection,
  settlement, standings, privacy, correction and return/session flows. Run any
  additionally affected backend/database checks and resolve read-only review.
- Stop: usable complete game, documented and published. Do not start release
  acceptance or administer production hosting in this step.

## Fantasy implementation queue

Execute one bounded step after a separate user instruction; this is sequencing,
not permission to run the whole queue. Revalidate the checkout and detailed
scope at each intake. A partial foundation is not a playable feature.

1. **FANTASY-6 — release acceptance and documentation.** Exercise a multi-round
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
