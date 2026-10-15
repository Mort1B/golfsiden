# Fantasy result loading and refresh measurement — A1t

Measured application revision: `b6a81a34c6278e2c7c062a3a5bc436fae130519c`.
Production source, API contracts, migrations, indexes, dependencies and refresh
behavior are unchanged. This is a local performance investigation, with no new
rated security/correctness defect and no production-capacity claim.

## Outcome

A round-scoped result GET uses a constant statement count but loads canonical
sporting facts from every non-draft round. In this small fixture the bulk query
accounts for 83–95% of tracked SQL execution time. This supports one later
candidate: narrow fact loading for a round result while preserving all existing
authorization, transaction, finalization and locking behavior. No runtime
optimization is implemented here. Refresh deduplication is not selected: the
observed overlapping waves cancelled reads rather than completing two whole
duplicate sets, and cancelling a browser request does not prove zero server work.
The fact-loading candidate applies to populated result reads. The measured
save-refresh phase is all draft and returns zero canonical sporting owners;
narrowing populated facts is not a demonstrated fix for that phase's cost.

## Method and evidence

The [predeclared design](design.md) distinguishes editable save fixtures from
populated result fixtures. Two synthetic tournaments have one/five rounds, eight
zero-handicap golfers, four two-player scramble teams per round, 18 par-four
holes and two enrolled managers with different first-round picks. Setup, lineup
saves, opening, scoring, confirmation and completion use existing HTTP APIs.
Later rounds retain the same team composition for this cost comparison.

The actor is the administrator at 1280x900, default Min firer/first-round selection,
overall board and no expanded details, with full authorized visibility. The
page's existing presets/admin components keep N+3 Fantasy query keys active;
memberships, round inventory and roster add three other private keys. Selected
round views share their preset key. Initial navigation and SSE opening settle
before samples. Deadlines are unset and no batch action is used.

The final run uses PostgreSQL 17.10 Alpine, schema 39, real Chrome 155.0.8059.79,
Node 22.22.2, an unchanged debug API and the Vite development frontend. PostgreSQL
uses tmpfs, default 128MiB shared buffers, 4MiB work_mem, JIT on, IO timing on and
planning tracking off. The unchanged API binds inside the owned PostgreSQL
network namespace; PostgreSQL/API host ports and Vite bind to loopback. The API
container uses this Linux host's read-only `/usr/lib64` and built binary, no
capabilities and no-new-privileges. This is a diagnostic runtime arrangement,
not a production image or runtime-privilege acceptance test.

- [measurements.json](measurements.json): sanitized per-request records, timings,
  normalized SQL fingerprints/counts/rows/times/buffers and assertions.
- [summary.json](summary.json): reproducible aggregates, including normalized
  per-route completion/cancellation totals.
- [run-results.json](run-results.json): build/migration/run exits and owned cleanup.
- [run.py](run.py), [measure.mjs](measure.mjs), [summarize.py](summarize.py): runner,
  fixture/probe and aggregation support.

SQL fingerprints omit literal values/comments; no SQL parameters, HTTP bodies,
identities, credentials, session cookies or sensitive mutation payloads are
published. Browser fixtures use separate contexts for the second manager,
anonymous caller and nonmember. APIRequestContext setup/control requests are
separate from page-network refresh counts.

## Isolated round results

After the first materializing result read and five warmups, ABBA blocks use
tracking none/top/top/none, ten sequential reads per block: 20 reads per mode per
fixture. ALTER SYSTEM/reload changes actual API connections; an unmeasured read
follows each reload. None blocks produce no tracked statements. Top blocks each
require ten round-lock queries, the expected row counts, ten bulk queries and
210 SELECT/WITH executions. Every response is fully consumed and exactly equals
its fixture's first settled body. P95 uses nearest rank; these small local
samples are descriptive.

| Metric | One round | Five rounds |
| --- | ---: | ---: |
| HTTP median, tracking none | 21.28 ms | 44.98 ms |
| HTTP P95, tracking none | 27.39 ms | 57.71 ms |
| HTTP median, tracking top | 22.44 ms | 45.56 ms |
| HTTP P95, tracking top | 27.26 ms | 49.71 ms |
| Tracked statements per completed read | 25 | 25 |
| Tracked SQL mean per read | 3.272 ms | 10.659 ms |
| Canonical bulk-facts SQL mean per read | 2.716 ms | 10.081 ms |
| Bulk-facts share of tracked SQL time | 83.0% | 94.6% |
| Canonical owner rows per read | 4 | 20 |
| Round rows locked per read | 1 | 5 |
| Uncontended round-lock query mean | 0.039 ms | 0.045 ms |
| Result body bytes | 26,208 | 26,208 |

The 25 statements are 21 SELECT/WITH calls plus two BEGINs, one ROLLBACK and one
COMMIT. PostgreSQL does not retain SET TRANSACTION in this capture. These are
statement counts, not a claimed count of protocol round trips. The single bulk
statement returns one JSON fact object per owner and performs per-owner lateral
aggregation; a constant query count does not establish constant work.

Tracking raised the observed medians by 1.15/0.58ms (5.4%/1.3%). P95 moved in
opposite directions. There is no statistical demonstration of negligible
overhead, no correction applied to samples, and no speedup claim. No tracked
shared blocks were read from storage in these steady blocks; fixture setup and
warmups already touched pages. Neither phase is a cold-disk measurement.

HTTP includes loopback/proxy transport, pool/transaction work, SQL round trips,
server fact hashing/deserialization/projection and response handling plus client
JSON decoding. SQL execution is separately aggregated server time. It excludes
application CPU, client transport and some transaction costs; subtracting it
from HTTP would not isolate projection. Fact JSON byte size and Rust loading CPU
were not separately instrumented. This result is distinct from the earlier
projection-only R3 benchmark.

## Lazy finalization and locks

The first observed populated GET takes 49.64/88.27ms and is deliberately excluded
from steady samples. Although ordinary sporting rounds are already completed,
Fantasy lock materialization is still lazy. Scalar database evidence changes
locked windows from 0 to 1/5, locked selections from 0 to 2/10 and carried lineups
from 0 to 0/8. First-read SQL is retained separately. Subsequent read/reload/lock
samples preserve those counts and exact settled responses. The internal
`finalized` field is omitted from JSON, so it is not used as browser evidence.

All six controlled lock probes directly observe a PostgreSQL lock wait with a
blocking PID before holding the row for another 200ms and releasing it. HTTP
times range 432.86–442.28ms for one round and 452.07–540.49ms for five. These
include Podman/psql control startup and polling overhead; they do not measure a
pure 200ms wait, total transaction lock holding time, or production contention.
The uncontended query times above are not a reason to remove locks.

The unchanged [read boundary](../../../backend/src/repositories/fantasy/results/mod.rs)
preauthorizes, starts repeatable read, locks tournament rounds in UUID order,
reauthorizes, locks the enabled game, finalizes due windows, loads facts,
validates the target, projects, rechecks the live session and commits.
[Fact loading](../../../backend/src/repositories/fantasy/results/load.rs) hashes
and deserializes every returned canonical owner before scoped projection. Valid
GETs can materialize other rounds and emit committed invalidation. This is not
an entirely side-effect-free read boundary.

## Save, SSE and idle windows

Each fixture has one warmup, then ten deliberate individual captain edits:
five full/five minimal observers, alternating. Each sends a new immutable
request with the current revision. All 20 measured saves and both warmups create
exactly one accepted submitted receipt each; all ten fully observed writes are
completed PUT 200, with no write cancellation or retry.

| Per fully observed save, five samples each | One round | Five rounds |
| --- | ---: | ---: |
| Completed GETs | 7 | 11 |
| Of which Fantasy GETs | 4 | 8 |
| Cancelled GETs | 4 | 8 |
| Completed PUTs / accepted receipts | 1 / 1 | 1 / 1 |
| Conflict responses | 0 | 0 |
| Finite requests pending at tail cutoff | 0 | 0 |
| Median tracked SQL across the whole window | 72.24 ms | 173.53 ms |
| Rendered current-lineup median, full observer | 123.98 ms | 151.93 ms |
| Rendered current-lineup median, minimal observer | 132.16 ms | 165.70 ms |

The request-count window begins at click and ends after the UI receipt, the
existing edit control becoming enabled, a one-second tail and a finite-request
quiet barrier of at least 500ms. It is a **barrier-window count**, not a count
limited to exactly one second. No finite request is pending at cutoff in any
sample, and exactly one successful unfinished SSE request is present at every
save check and both three-second idle checks. Idle windows start no API
request. A finite quiet period cannot rule out indefinitely delayed SSE delivery.
The probe does not count individual SSE event types/delivery or establish
uninterrupted connection throughout minimally observed windows.

Source tracing explains the starts: the acknowledgment invalidates the whole
Fantasy family (4/8 active targets); a committed `round` SSE invalidates the
private workspace (7/11 targets). The measured pattern is consistent with
overlapping waves: 4/8 GETs abort and 7/11 finish. This attribution combines the
source trace with request evidence; it is not a per-SSE-event timing capture.
The full observer retains each route/status and explicit
`net::ERR_ABORTED` evidence. No unread error body is silently treated as success.
The [current-refresh proof](../../../frontend/src/features/fantasy/fantasyRefreshProof.ts)
and explicit target fallback remain unchanged. We do not identify a completed
duplicate full wave or claim all cancelled work is avoided at the database.

The SQL-window aggregate includes the mutation, active refresh requests,
partially processed cancellations and SSE authorization. Concurrent waits may
overlap; it is not wall time or SQL cost per completed GET. The raw `receipt_ms`
field actually ends when the current-lineup receipt renders from its authoritative
round GET; it includes reconciliation, Playwright action/assertion observation and
rendering, and is separate from immediate acknowledgment or PUT duration. The
post-ready one-second tail starts only after Endre valg becomes enabled, with an
additional variable wait after the measured rendered-lineup endpoint. That wait
is not timed independently. This boundary refinement is a recorded design
deviation; no one-second-from-acknowledgment latency is claimed.
Full observation has lower medians than
minimal here, so these five-sample groups establish neither zero overhead nor an
observer speed benefit. No timing subtraction or overhead correction is used.

## Validation and limits

The complete final run passes: unchanged locked API binary build, all 39 clean
migrations, frontend production build, real Chrome/API/PostgreSQL probes, 80
timed steady successful reads, six lock controls, two first-materializing reads,
20 measured saves plus two warmups, exact subsequent response parity, receipt
counts, anonymous 401/nonmember denial and other-manager pre-lock pick absence.
Each first-round result has eight settled golfers, two settled managers and 18
holes per golfer, with Cache-Control `no-store`. Both browser error lists are
empty. Owned API/PostgreSQL containers, Vite process and temporary credential
files are removed. No hosted operation ran.

Setup and independent review exposed diagnostic issues: missing Podman stdin,
an overlong synthetic username, waiting insufficiently for SSE headers, a
TIME_WAIT-sensitive port preflight, API listener containment, SQL comment
classification, and an invalid assumption that ordinary round opening had
already materialized Fantasy locks. Those failed pilots are excluded from timing
aggregates. The design always separated first materialization from steady reads;
the corrected probe directly records its actual writes. Later review tightened
cleanup exception handling and clears task-owned output before reruns. These are
probe repairs, not application findings. Final read-only backend review
independently recomputes the HTTP/SQL/window aggregates and accepts the qualified
fact-loading/finalization/lock conclusions. Frontend review accepts counts while
requiring the rendered-lineup/tail deviation and limited SSE/header/write claims
now recorded above. Security review verifies sanitized evidence and corrected
isolation/cleanup controls. Syntax and static documentation/scope checks
supplement the completed run. No blocking application or evidence finding remains.
Final static checks pass: JavaScript/Python syntax, all 149 local link targets in
affected Markdown, JSON parsing and retained identity/credential-pattern checks,
phase/receipt/cutoff assertions, EOF/whitespace and documentation-only source
scope. Separate post-run verification finds ports 3000/5173 free, no owned
fixture container and no retained task credential file.

The unchanged backend regression/Clippy and frontend unit/lint ladders are not
rerun for this documentation/diagnostic-only step. There is no production source,
schema or contract change. Existing revocation, final-expiry, credential-loss,
hidden-final noninterference, format diversity, mobile layout and physical-device
acceptance are not re-established by this small administrator/full-visibility
fixture. Local debug/container/dev-server timings do not predict production
latency, throughput, contention, capacity, image advisories or hosted readiness.
No empty offline-queue or batch measurement is included.

## Reproduce

Requires the repository's installed frontend dependencies, Rust build tools,
system Chrome, rootless Podman with cached `postgres:17-alpine`, Linux
`/usr/lib64` compatible with the locally built binary, and free ports 3000/5173.
The runner refuses occupied ports, creates only its own disposable containers
and random synthetic accounts, and removes its own fixtures in finally blocks.
It overwrites only this report's measurement/run output files.

```bash
python3 docs/validation/fantasy-measurement-2026-10-15/run.py
python3 docs/validation/fantasy-measurement-2026-10-15/summarize.py
node --check docs/validation/fantasy-measurement-2026-10-15/measure.mjs
git diff --check
```

Do not run this against hosted/user databases or concurrently with other local
services on those ports. The scalar lock helper and pg_stat resets operate only
inside the newly owned fixture.
