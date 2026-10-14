# Pre-deployment fixes — 2026-10-14

This sequence addresses R1–R3 from the
[assessment](../predeployment-review-2026-10-14/README.md). The user authorized all
three fixes together; each is bounded, validated and published separately.
Hosting and physical iPhone acceptance remain operator-owned.

## R1: precise browser error evidence

The preset recovery test no longer treats every error response as its injected
503. Its observer permits only the expected GET endpoints/status/error-code
combinations. A body that cannot be read is accepted only when Playwright records
an explicit canceled GET; unexpected body errors, codes, statuses, writes and
other transport failures stay fatal. Body inspection promises are collected and
awaited, including those added while earlier checks finish.

The retained scenario verifies both overall and round boards after retry, along
with its existing loading, error, closed-destination and draft assertions. A
separate scenario waits for initial live/read stabilization, injects a conflict
on each projection endpoint, requires readable `fantasy_conflict` evidence for
both, and verifies both boards finish refreshing and render again. Pure tests
also reject an unrelated 409 and unread responses without cancellation evidence.
No application behavior, production retry policy or scoring rule changed.

Validation passed: 1,102 tests across 142 files; typecheck, lint, strict browser
TypeScript and production build; 20/20 repeated Chrome cases (ten original
recovery plus ten delivered-conflict cases); two retained copy/uncertain-response
cases. Independent read-only review has no remaining findings. The final logs
are `/tmp/golf-r1-final-{tests,build,browser}.log` and
`/tmp/golf-r1-retained.log`. Commands from `frontend/`:

```bash
npm run test
npm run typecheck
npm run lint
npx tsc -p tsconfig.browser.json --noEmit
npm run build
GOLF_FANTASY_BROWSER=1 npx playwright test --config playwright.lifecycle.config.ts fantasyPreset.browser.ts --grep 'overview recovers|both projection' --repeat-each=10 --timeout=45000 --output=/tmp/golf-r1-final-browser
GOLF_FANTASY_BROWSER=1 npx playwright test --config playwright.lifecycle.config.ts fantasyPreset.browser.ts --grep 'presets three|pauses partial' --output=/tmp/golf-r1-retained
```

Local API/Vite use an owned schema-39 PostgreSQL 17 fixture container
`golf-fixes-20261014-pg` on port 55449. Migration and development seed passed.
All accounts/results are synthetic. The backend was unchanged for R1, so its
complete ladder was not repeated; the assessment records its preceding green run.

An intermediate observer waited for `response.finished()` before reading bodies;
a superseded repetition run stalled and was stopped. The final observer reads
JSON directly, then checks actual request cancellation if reading fails. The
independent review also required proof that injected conflicts were delivered,
which is now asserted per endpoint. Intermediate runs are not acceptance evidence.


## R2: reuse only a proven current target refresh

The coordinator now records ephemeral evidence of a network fetch followed by a
non-manual success during its own invalidation. It reuses a target only if the
same query still has that success counter, active/idle state, data and no pending
invalidation, with the canonical session checked immediately before access.
An existing initial request, manual cache write, missing/new/inactive/replaced
query, failure or erased data cannot provide that proof. The former explicit
read remains the fallback. The subscription is removed in `finally`.

Full-root invalidation, SSE updates and per-write batch preflight are retained.
This removes a known duplicate; it does not claim to eliminate the whole batch's
quadratic overview refresh cost. Broader invalidation changes would need their
own dependency and privacy analysis.

Proof tests cover active successful refresh, manual writes, older in-flight reads,
new/inactive targets, erasure/replacement, cancellation, failure and real overlapping
refreshes where late canceled data cannot satisfy the newer read. Existing tests
continue to cover session changes, acknowledgment/refresh separation, activation
that mounts failing queries and exact uncertain-write replay. An older refresh
feedback test now asserts reuse instead of requiring the duplicate read; it still
requires the later uncertain outcome and exactly two writes.

The deterministic three-round batch regression requires 12 round reads (three
preflights plus nine reconciliation reads), compared with 15 before R2. The test
has no live SSE. Repeating the assessment's real-SSE Chrome probe reduced round
request starts from 24 to 21 for three saves and from 60 to 55 for five saves.
The extra game/overall/round-result refresh counts remained 6/6/6 and 10/10/10;
writes remained exactly three/five. These totals include started requests that
may be canceled or still pending at the success cutoff. This is one avoided
round request per saved draft, not a claim about completed database queries.

Validation passed: 1,113 frontend tests across 143 files, typecheck, lint, strict
browser TypeScript and production build; all four preset/recovery Chrome cases
and both real-SSE count probes. The final overlapping-refresh proof test was
rerun after correcting the fixture to resolve both fetches: TanStack's silently
canceled first promise follows the replacement request, so awaiting it before
resolving the replacement incorrectly deadlocked the initial test. All ten proof
cases then passed. Read-only review found no production correctness issue.
Logs: `/tmp/golf-r2-{focused,tests,build,browser}.log`; the temporary count probe
was removed from the checkout and retained at `/tmp/golf-r2-Cost.browser.ts`.
No backend or migration code changed in R2; the backend ladder is deferred to R3.

## R3: project only the requested round

Round-result reads now call the existing round projector once, rather than
projecting every round, calculating overall ranks and hashing discarded results.
The shared read helper retains preliminary authorization, repeatable-read
isolation, stable all-round locks, tournament-wide lazy finalization, full bulk
loading, target validation, session recheck and commit. Overall and both breakdown
endpoints retain full projection. Shared response finishing preserves revision
hashes and the internal finalization flag that triggers post-commit invalidation.

The loader still validates all loaded facts. An unrelated round's domain-only
projection error no longer blocks a valid selected-round result; the failing
round and overall projection still reject it. A regression records this deliberate
scope behavior. No SQL, schema, dependency, scoring formula or API shape changed.

### Projection measurement

A pre-change release baseline was recorded before switching the round endpoint:
median 0.418/1.187/1.942 ms for one/three/five rounds. The paired final test warms
both paths 50 times and alternates 500 samples per path, using `black_box` with
fixture creation outside timing. Fixture: 36 golfers, 24 managers, 18 scored holes
per round and locked selections. Environment: Rust 1.99.0, default optimized
release profile, x86_64 Intel i5-1135G7.

| Rounds | Full median | Scoped median | Full p95 | Scoped p95 |
| --- | ---: | ---: | ---: | ---: |
| 1 | 0.419 ms | 0.358 ms | 0.434 ms | 0.370 ms |
| 3 | 1.179 ms | 0.368 ms | 1.216 ms | 0.380 ms |
| 5 | 1.941 ms | 0.368 ms | 1.998 ms | 0.382 ms |

These measure projection work only, including response construction/hashing and
result disposal. They exclude database loading, lock waits and network time; no
end-to-end API latency improvement is claimed. The ignored benchmark is an
observational tool, not a timing threshold in CI.

### Regression coverage and review

The new parity test compares 720 serialized round responses across both scoring
versions, draft/open/completed/locked states, full/front-nine visibility, three
callers, numeric/match formats and both finalization flags. The internal flag is
checked separately because it is omitted from JSON. The match fixture covers
pending outcomes; retained PostgreSQL match tests cover accepted ledgers and
corrections. Retained numeric result tests cover DNF and stale DNF. New transaction
tests verify another due round is finalized by a valid scoped read, one event
follows commit, and revocation while waiting cannot commit lazy materialization.
Existing invalid-target rollback and privacy tests remain unchanged.

Focused validation passed: two unit tests and six PostgreSQL result tests; the
manual release benchmark was run explicitly before and after the change. A first
sandboxed database attempt lacked socket permission and was successfully rerun
with escalation. Independent read-only review found no actionable issue and
identified the fixture limits recorded above. Production `results/mod.rs` is
283 physical lines, below the repository limit.

Reproduction commands from the repository root (set `DATABASE_URL` to an approved
disposable PostgreSQL database):

```bash
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --features database-tests
cargo run -p golf-api --bin migrate
cargo run -p golf-api --bin seed
cargo test -p golf-api --release --lib projection_benchmark -- --ignored --nocapture
```

Retained browser command from `frontend/`, with disposable API/Vite running:

```bash
GOLF_FANTASY_BROWSER=1 npx playwright test --config playwright.lifecycle.config.ts fantasyPreset.browser.ts fantasyScoring.browser.ts fantasyRelease.browser.ts fantasyReleaseFormats.browser.ts --output=/tmp/golf-r3-browser-results
```

Logs: `/tmp/r3-{baseline-benchmark,paired-benchmark,unit,database}.log` and
`/tmp/golf-r3-{tests,clippy,build,postgres,migrate,seed,browser}.log`.

Final checks: formatting, all-target backend tests (**258 passed**, one manual
benchmark ignored in the ordinary run), Clippy with `-D warnings`, and binary
build passed. Clippy printed three existing vendored `sqlx-postgres` warnings;
there were no application diagnostics. The benchmark was executed separately as
recorded above. The retained Chrome run passed **9 scenarios in 1.8 minutes**.
One historical-upgrade browser case was explicitly skipped because the current
schema-39 fixture lacks preserved schema-38 browser sessions; no migration changed
in R3. Both versions are covered by exact projection parity and the PostgreSQL
scoring tests. R2's complete frontend ladder remains applicable because R3 changes
no frontend code.

The complete PostgreSQL ladder passed **736 tests, zero failures**, with four
ignored timing probes. The new projection benchmark was run explicitly; the
three pre-existing match-scale probes require exclusive PostgreSQL with
`pg_stat_statements` and were not run. Migration and development seed checks
passed against the disposable schema-39 database. Documentation link and diff
whitespace checks passed. Review found no remaining actionable issue.

All three authorized fixes are complete. Physical iPhone Chrome/Safari checks,
public hosting acceptance, broader invalidation changes and SQL/locking
optimizations remain outside these fixes.
