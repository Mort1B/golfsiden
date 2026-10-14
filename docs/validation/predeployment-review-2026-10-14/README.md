# Pre-deployment logic and optimization assessment — 2026-10-14

Reviewed application revision: `8294e520722147ad18079f8ad8c99d856a607c6b`.
Scope: the recent Fantasy all-round presets (`7260928`), optional personal round
awards (`6921c83`), Fantasy scoring versions (`90b8843`), iPhone layout (`8294e52`)
and their immediate dependencies. This is an assessment, with no runtime,
dependency, schema or hosted-infrastructure change.

## Verdict

No confirmed deployment-blocking application defect was found in this scope.
Independent frontend/backend source review supports the intended scoring,
authorization, migration and recovery contracts. This is not an exhaustive audit
of the whole application or a production load/readiness certification.

One existing browser assertion is demonstrably unreliable and should be repaired
before using that suite as an unattended release gate. Two optimization candidates
are below. Neither has an established user-visible latency budget violation;
neither justifies changing authorization, locking or recovery guarantees.

## Findings

### R1 — P2: browser acceptance rejects recoverable projection conflicts

Location: `frontend/e2e/fantasyPreset.browser.ts:90` and `:119`.
The "overview recovers a failed round read and excludes a closed copy destination"
case collects every HTTP error status and requires all of them to be its injected
503. Current Fantasy query code deliberately retries `409 fantasy_conflict` up to
two times (`frontend/src/features/fantasy/useFantasy.ts:23` and
`fantasyRoundQuery.ts:10`). The status-only assertion rejects a retryable
read conflict even when all preset UI assertions succeed. The affected overall
or round-results query is not explicitly asserted to recover in this test.

Reproduction: copy that case unchanged except for diagnostic logging of failed
response status, path and error code; repeat ten times against the disposable API.
Six runs passed and four failed, all at the final status assertion. All four
included a 409 from overall or selected-round results. Two completed error bodies
contained `fantasy_conflict`; two bodies could not be read (the reason was not captured), so
this report does not infer their error code. Every run reached and passed the
preceding loading, failure, retry, closed-destination and draft assertions.

This demonstrates a current cause of intermittent failures matching the earlier
symptom. It does not recover the missing response from the original iPhone run.
The remaining 503 was intentionally injected and is not an API outage.

Smallest remediation: validate expected 503 and retryable 409 by endpoint, error
code and eventual recovered UI state. Keep unexpected responses and real transport
failures fatal. Classify a missing response body separately, accepting it only
when cancellation is established for the expected read; do not swallow arbitrary
JSON/read errors. Add a deterministic conflict/recovery scenario and verify a
non-conflict 409 still fails the observer. No production retry change is implied.

### R2 — P3: batch saves initiate repeated full-overview reads

Locations: `frontend/src/features/fantasy/FantasyPresets.tsx:15`,
`useFantasyBatch.ts:37`, `useFantasy.ts:51-52`, `refreshFantasy.ts:18-31`.
All round queries stay active. Each individual save has an authoritative preflight,
then invalidates the entire Fantasy query family, then explicitly fetches the saved
round again with `staleTime: 0`. An N-round batch normally initiates N²+2N round
reads before any SSE invalidation overlap. The preflight is intentional; the extra
post-refresh target read and whole-family work are optimization candidates.

A local Chrome probe counted requests beginning immediately before the batch
button click and ending at the all-confirmed success message. It used real API
fixtures with four eligible golfers, one manager and real SSE, with no mocked
responses or network throttling:

| Drafts saved | Round reads started | Game reads | Overall reads | Round-result reads | Lineup writes |
| --- | ---: | ---: | ---: | ---: | ---: |
| 3 | 24 | 6 | 6 | 6 | 3 |
| 5 | 60 | 10 | 10 | 10 | 5 |

The measured totals include every request start, including attempts that may be
canceled or still in flight at the cutoff. Source review explains overlapping
action/SSE refreshes, but these counts do not isolate each cause. Request-start
counts are not completed server-query counts. All writes were
acknowledged; there were no observed HTTP error responses in the successful probes.
This demonstrates request amplification, not a production throughput result.
Single-run completion times were 944/965ms on the same host while other checks
were running, so they do not establish a scaling law or a latency improvement.

Smallest safe first optimization: reuse a verified successful target read from
that exact refresh operation instead of issuing it again. Keep explicit fetching
for absent/inactive/newly mounted/invalidated targets and the pre-write fresh
read. Test request counts together with failed refresh, query erasure, session
replacement, unmount and immutable uncertain-write recovery. Narrowing all-root
invalidation is a larger follow-up requiring dependency and SSE analysis.

### R3 — P3: one-round results compute the whole tournament

Location: `backend/src/repositories/fantasy/results/mod.rs:29-121` and `:159-164`.
The shared result loader locks all tournament rounds, finalizes due selections,
loads tournament facts, projects every round, ranks both overall boards and hashes
all result envelopes. A round-only request subsequently discards all but its
requested round. This behavior predates the reviewed scoring-version change.

The redundant projection/aggregation is confirmed by source; its latency and lock
cost have not been isolated. Measure it on representative populated tournaments
before changing it. A future specialized round projection must preserve the
existing authorization/finalization boundary and return equivalent permitted
JSON, versions and hashes. Keep locks until concurrency evidence supports a change.
No query-cache shortcut or weakened authority check is recommended.

## Logic checks

- Migration 39 takes its cutoff after lock acquisition, preserves games with
  opened/expired/locked selection evidence and rejects runtime version changes.
  Eligible unlocked games use v2; legacy games retain v1.
- Actual aces take precedence once: v2 +20, other net albatross-or-better +5;
  v1 remains +10/+10. Match awards remain independent of hole categories.
- Closest-to-pin and longest-drive commands preserve exact replay/revision checks,
  administrator/session checks, locked-round correction acknowledgment and audit
  history. Awards affect personal net results before best-N selection and ties.
  Gross values, native hole scores, shared team rank and Fantasy facts are separate.
- Presets retain per-round revisions and independent drafts. Batch processing
  checks authority/eligibility before each write and pauses on problems. Recovery
  retains immutable uncertain requests; acknowledgment is separate from refresh.
- Result version mismatches suppress points/detail/settlement rendering. Provider
  lifetimes remain scoped to account, CSRF session and tournament.
- Mobile changes are CSS geometry, with no resize-driven state, write or remount.
  Safe-area ownership, compact rail scrolling, text wrapping and fixed-navigation
  clearance were reviewed against public and private page styles.

## Validation and environment

Local PostgreSQL 17 ran in owned rootless Podman container
`golf-review-20261014-pg` on port 55448. Schema 39 migration and development seed
succeeded. The API used that disposable database on port 3000; Vite ran on 5173.
All fixture accounts and results were synthetic. No hosted credentials were used.

| Check | Current result |
| --- | --- |
| Frontend unit/component tests | 1,100 passed across 141 files |
| Frontend typecheck, lint, production build | Passed |
| Rust format and Clippy, all targets/features | Passed; existing vendored SQLx warnings |
| Rust tests without database feature | 256 passed |
| Full Rust/PostgreSQL tests with database feature | 732 passed; 3 existing performance probes ignored |
| Clean schema migration and development seed | Passed, schema 39 |
| iPhone-sized Chrome flow/layout suite | 24 passed, 1.7 minutes |
| Repeated retained preset recovery scenario with diagnostic logging | 6 passed, 4 failed at the final HTTP-status assertion (R1) |
| Batch request-count probes | 2 passed, for 3 and 5 rounds |
| Independent frontend/backend source review | No confirmed application correctness defect; R2/R3 observations |

The PostgreSQL ladder includes schema-38-to-39 upgrade and concurrency tests. The
three ignored cases are pre-existing `pg_stat_statements` performance probes,
not missing scoring tests. Representative current desktop and portrait Fantasy
screenshots were inspected; browser scenarios also assert console/network state.
Unit-only Rust totals are included in the full database-feature total, not additive.
Logs are `/tmp/golf-review-{frontend-tests,build,clippy,backend-tests-local,database-tests,iphone,repeat,cost-recheck}.log`.

Commands follow `docs/AGENT_WORKFLOW.md`. Relevant browser commands, from
`frontend/`, were:

```bash
GOLF_FANTASY_BROWSER=1 npx playwright test --config playwright.lifecycle.config.ts reviewRepeat.browser.ts --grep 'overview recovers' --repeat-each=10 --output=/tmp/golf-review-repeat-results
npx playwright test --config playwright.lifecycle.config.ts reviewCost.browser.ts --output=/tmp/golf-review-cost-recheck
GOLF_IPHONE_API=1 npx playwright test --config playwright.iphone.config.ts --project=chrome --output=/tmp/golf-review-iphone-results
```

The two `review*.browser.ts` files were temporary diagnostic probes; they are not
product changes or replacements for the failing retained test. Copies and logs
are retained locally under `/tmp/golf-review-*`. The count probe's first attempt
failed its 5-second initial live-connection wait; a diagnostic rerun allowed the
same 30-second connection wait used by the existing preset tests and completed.

The initial sandboxed Rust run failed eight mock-server tests because socket
creation was forbidden. Those were rerun with local socket access. This is an
environment failure, not eight application defects. Existing vendored SQLx warnings
are unchanged.

Production build sizes: main JS 467.50 kB / 144.40 kB gzip, lazy Fantasy page 61.62 /
19.62 kB gzip, lazy tournament management 141.31 / 39.36 kB gzip. These are build
artifact sizes, not complete route-transfer measurements or evidence of a new
size regression. Route splitting remains in place; no speculative dependency
replacement is recommended by this assessment.

## Limits and next action

Repair R1 as one bounded test-only step, then consider the measured R2 cost.
R3 needs representative profiling before implementation. No optimization has been
applied and no speedup is claimed.

This review does not run production hosting, TLS/DNS, backup/restore or load tests.
Physical iPhone Chrome and Safari acceptance remains after deployment, including
real keyboards/toolbars, safe areas and native zoom. Previous WebKit acceptance
is documented in the [iPhone report](../iphone-layout-2026-10-14/README.md); it is
not represented as a newly executed check here. Duo/foldables remain excluded.
