# Scoped Fantasy sporting facts — A1u

Baseline: `e3ee9663c0e21178aa982d6682dbec7c23aa5de6`. This implements the single
candidate supported by [A1t](../fantasy-measurement-2026-10-15/README.md), with no
new security/correctness severity rating.

## Result

The five-round populated fixture's scoped HTTP median with SQL tracking disabled
falls from **45.03 to 22.45ms** (50.1% in this local comparison). Canonical owner
rows fall from twenty to four; tracked bulk-facts SQL averages **10.59 to 2.72ms**.
This supports retaining the bounded change. The one-round control has the same
four owners and no consistent timing gain. No production speedup or save-refresh
benefit is inferred.

| Metric | One round baseline | One round candidate | Five rounds baseline | Five rounds candidate |
| --- | ---: | ---: | ---: | ---: |
| HTTP median, tracking none | 21.75 ms | 22.16 ms | 45.03 ms | 22.45 ms |
| HTTP P95, tracking none | 28.78 ms | 25.68 ms | 48.75 ms | 26.32 ms |
| HTTP median, tracking top | 22.07 ms | 20.67 ms | 43.52 ms | 20.61 ms |
| HTTP P95, tracking top | 29.66 ms | 23.93 ms | 45.78 ms | 29.71 ms |
| Mean tracked SQL per read | 3.421 ms | 2.884 ms | 11.042 ms | 3.229 ms |
| Mean canonical bulk SQL per read | 2.970 ms | 2.552 ms | 10.590 ms | 2.723 ms |
| Canonical owner rows per scoped read | 4 | 4 | 20 | 4 |
| Tracked statements per scoped read | 25 | 25 | 25 | 25 |
| Round rows locked per scoped read | 1 | 1 | 5 | 5 |

Each build/mode/fixture has 20 measured reads, 160 timed steady reads in total.
P95 uses nearest rank. Both candidate blocks improve the five-round HTTP median
in both observer modes; one-round shifts and observer-mode differences are
inconsistent and show local noise rather than a general speed benefit. No
statistical confidence interval or observer-overhead correction is claimed.
The 25 statements comprise 21 SELECT/WITH executions, two BEGINs, one ROLLBACK
and one COMMIT; SET TRANSACTION is not retained by pg_stat_statements. No tracked
shared blocks are read from storage in these steady blocks. Exact normalized
non-canonical fingerprints/call/row budgets remain equal across builds.

Both builds' overall/golfer/manager controls return four/twenty canonical owners,
preserving the full paths. Both first reads materialize one/five windows,
two/ten locked selections and zero/eight carried lineups from identical restored
state, with equal 26,208-byte scoped result bodies. First-read HTTP times are
54.01/53.40ms for one round and 106.17/75.04ms for five (baseline/candidate).
Those single materializing requests include lifecycle work and are excluded
from steady comparisons. They are not pure finalization or cold-cache timings.

## Change

Round results now pass their requested round to the private loader, which filters
the existing ordered tournament owner identifiers before canonical bulk-fact
loading. Other reads pass no round filter. The canonical SQL and fact JSON are
unchanged. This avoids fetching, hashing and deserializing other rounds' sporting
facts for one round projection. Full inventories, including owner identifiers,
remain tournament-wide, and no query or index is added.

Preauthorization, repeatable read, UUID-ordered all-round locks, enabled-game
lock, every due-window finalization/carry-forward, target validation, visibility,
final live-session check, commit and postcommit notifications remain intact.
Overall results and golfer/manager breakdowns still load all rounds. No frontend,
API/DTO, schema, dependency, scoring, batch, mutation recovery, SSE or hosting
change is included. The populated-read opportunity does not imply a benefit for
the all-draft save-refresh workload, which has no canonical sporting owners.

## Reproducible comparison

The [predeclared design](design.md) uses the same one/five-round fixture geometry
as A1t: eight zero-handicap golfers, four scramble teams per round, 18 holes and
two managers. Existing HTTP APIs create, configure, submit first-round lineups,
open, score, confirm and complete rounds. No direct sporting-state bypass is
used by this diagnostic. Test-only historical constructors remain confined to
the pre-existing Rust fixtures.

[baseline.py](baseline.py) builds a pinned Git archive and the current checkout
with the same locked debug profile/cache, and records hashes in
[builds.json](builds.json). [run.py](run.py) verifies both hashes, owns PostgreSQL,
API namespace and Vite, and runs [measure.mjs](measure.mjs) with real Chrome.
Both API and database ports are published only on loopback. The diagnostic API
uses the host's read-only Linux runtime libraries and no capabilities; this is
not production image/privilege acceptance.
The candidate package is explicitly rebuilt after the baseline so Cargo's shared
executable output cannot be mistaken for a cached candidate. Both hashes must
match provenance and differ. Baseline archive sources are removed after building.
Chrome's exact detached process-group ID is retained only in private temporary
metadata and checked independently during cleanup, including an exited leader.

The owned database is snapshotted before the first Fantasy lock materialization
and restored with the API stopped before the candidate's first read. Dumps and
complete response bodies stay in memory, outside Git. Each fixture runs
baseline/candidate/candidate/baseline; each block measures tracking none then top,
five warmups per mode and ten complete sequential scoped HTTP reads per mode.
There are 20 reads per build/mode/fixture. Global tracking reloads are checked
through empty none snapshots and positive top budgets. Collector/reset SQL is
excluded. All other tracked fingerprints, calls and rows must equal baseline;
only canonical owner rows may shrink.

First materializing reads are separate from steady samples. Both builds must
produce exactly equal scoped JSON and preserve identical materialized counts.
All round-result bodies, overall results and all eight golfer/two manager
breakdowns are compared in memory across builds. Isolated overall/golfer/manager
SQL controls retain all owners and all-round locks. Raw sanitized evidence is in
[measurements.json](measurements.json), with [summary.json](summary.json) derived
by [summarize.py](summarize.py). No identities, HTTP bodies, SQL parameters,
credentials or session tokens are retained.

## Validation and limits

[validate.py](validate.py) owns a separate disposable PostgreSQL fixture for the
focused tests, full affected Rust/PostgreSQL ladders, migrations, seed and build.
Its captured exits/counts are in [validation.json](validation.json). Local raw
command logs remain under `/tmp/golf-a1u-*.log`.

- Focused PostgreSQL: 22 passed, including the new fact-wait expiry regression.
- Rust workspace/all targets: 264 passed, zero failed, one ignored manual
  projection-only benchmark.
- PostgreSQL-feature workspace/all targets: 953 passed, zero failed, five ignored
  manual probes (projection, three match-authorization and one team-loading).
  This count includes unit tests; it is not 953 database-only tests.
- Formatting, strict all-feature Clippy, all 39 clean migrations, seed and build
  pass. Existing dependency warnings remain unchanged; Clippy exits zero.
- Real Chrome 155.0.8059.79 on PostgreSQL 17.10 Alpine passes both fixtures at
  320/1280: two manager/eight golfer rows, no overflow, equal API results,
  isolated anonymous 401, no unexpected browser or request errors.
- [run-results.json](run-results.json) records successful comparison and owned
  Chrome/Node/Vite/API/PostgreSQL cleanup. Temporary dumps, bodies and credentials
  are not retained. The accepted comparison runs after all validation/build jobs.

A startup pilot stopped before timing when Podman had already removed its
cidfile. Provenance checks also caught Cargo executable-output aliasing before
acceptance. Both are fixed in the runner/build helper. A complete earlier probe
passed before independent Chrome-group cleanup was added; only the final rerun's
samples are retained. These are diagnostic repairs, not application findings.

Independent read-only source/security and evidence reviews report no remaining
findings. They recompute all 160 timing samples, SQL/full-path budgets and
materialization counts; verify retained binary hashes against provenance and
validation totals against logs; and confirm the qualified conclusions and
sanitized JSON. Final static checks pass: JavaScript/Python/JSON syntax, 165 local
Markdown link targets, identity/credential-pattern checks, file size/scope and
EOF/whitespace checks. Post-run checks confirm ports 3000/5173 free, no owned
fixture container and no retained private credential/browser metadata.

New cross-surface assertions compare complete selected-round golfer and manager
rows and round summaries against the unchanged full-load breakdown/overall paths
in individual, Stableford, four-ball, scramble, foursomes and match fixtures,
including hidden final results and changing partners/carry-forward. Existing
byte/revision projection parity and canonical fingerprint regressions remain.
Foreign-round 404s must roll back lazy finalization without SSE. A new actual
API regression blocks on selected canonical facts, observes the wait, lets a
prearranged session expire using database time, then requires 401/no-store with
no committed windows, selections, lineups or notification. Existing sibling-round
lock/revocation and once-only cross-round finalization tests are unchanged.

Measurements are local debug/container/Vite results, not production latency,
throughput, contention or hosted-readiness acceptance. Warmups/setup already
touch database pages; first application reads are not cold-disk reads. SQL
execution aggregates exclude application hashing/decoding/projection and client
transport, and cannot be subtracted from HTTP to derive CPU attribution.
Tracking overhead and small-sample noise are reported without correction.
Frontend source/contracts remain unchanged; its unit/lint ladder is not rerun.
Physical-device and operator-owned hosted checks remain separate.

## Commands

Requires Rust/frontend dependencies, rootless Podman with cached
`postgres:17-alpine`, system Chrome, compatible Linux `/usr/lib64` and free
3000/5173 ports. Run the ladders before the comparison so they do not compete for
CPU/database time. Probes create only owned disposable data and remove it.

```bash
python3 docs/validation/fantasy-fact-scope-2026-10-15/validate.py
python3 docs/validation/fantasy-fact-scope-2026-10-15/baseline.py
python3 docs/validation/fantasy-fact-scope-2026-10-15/run.py
python3 docs/validation/fantasy-fact-scope-2026-10-15/summarize.py
```

The runner replaces only its own measurement/run/summary files; the build helper
replaces its own baseline binary/build metadata and cleans/rebuilds only the
golf-api package's artifacts in the shared target directory. It preserves source
files and other packages' caches. Do not direct these scripts at hosted/user
databases. No other optimization or queued work is implemented.
