# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. The next candidate requires a separate user instruction.

## Next candidate

**FANTASY-6 — release acceptance and documentation.**
See the [Fantasy contract](ARCHITECTURE.md#fantasy-competition-design) and the
[current playing/admin flow](Documentation.md#playing-and-administering-fantasy).

- Goal: verify the complete game across the intended multi-round tournament and
  publish evidence of readiness and practical limits.
- Scope: nine two-person teams plus smaller/larger fixtures; supported scoring
  formats, real browser flows, regression repairs within the verified Fantasy
  boundaries, tests and affected operator/release documentation.
- Behavior to verify: changing partners/picks/captains; physical versus net aces;
  early-finished outcome-only matches; all-round golfer totals independent of
  selection; carry-forward chains, missed/invalid lineups, authorized corrections,
  non-finish staleness, hidden-final release and return freshness.
- Invariants: preserved net snapshots, team placement once, immutable locked
  picks, current authorization, hidden-result noninterference and no sporting
  scoring/lifecycle changes. No generated teams or replacement Fantasy picks.
- Validation: complete affected ladders and browser scenarios; disposable
  migration/upgrade and runtime-permission refresh checks; read-only review;
  reproducible evidence and clearly stated remaining limits.
- Stop: publish readiness/operator documentation and validated repairs, with
  clean main aligned to origin. Production deployment remains user-owned; no
  new Fantasy features or unrelated assessment in this step.

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
Fantasy remains separate from sporting scores and completion rules.
