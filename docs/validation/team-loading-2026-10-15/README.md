# A8 — bounded team-member loading, 2026-10-15

The P3 [original A8 finding](../application-review-2026-10-14/README.md#a8--p3-team-member-queries-grow-with-team-count)
was a sequential member query for every team. The shared loader now keeps the
existing ordered team SELECT, binds its selected UUIDs as one array and fetches
their members once. The empty-team-list path skips that second SELECT. Grouping
only looks up/removes members; output order comes from the original teams vector
and SQL member ordering.

Only `backend/src/repositories/teams.rs` production source changes. The ordinary
internal wrapper, session wrapper and every mutation function are byte-identical
to baseline `ecc494f98f40f3947a1b3cd91371a511bf53ff23`. Handlers, DTOs, schema,
indexes, frontend, dependencies, cache middleware and live events are untouched.
The existing `session_read` owner still controls repeatable read, session/user
share locks, stored-round membership, one full serialization retry and the final
wall-clock session check, including on empty results.

## Query-count regression and response parity

The retained [PostgreSQL measurement test](../../../backend/tests/team_loading/measurement.rs)
uses an exclusively owned PostgreSQL 17.10 Alpine/musl instance with
`pg_stat_statements` preloaded. Three warm-ups precede 30 sequential samples per
size and path. `internal` calls the actual ordinary repository wrapper; `session`
calls the actual protected wrapper; `http` runs the actual Axum router/extractor,
handler, serialization and response-body collection using Tower `oneshot`.
It includes test request construction/JSON decoding and excludes TCP, proxy,
TLS and browser rendering. This is a local debug-build application-read probe.

The original production loader was measured before implementation. The budget
test intentionally failed only its final constant-query assertion, after saving
all 12 sample groups. Six expected failures cover 12/100 teams on each of the
three paths. The repaired loader passes the same test. Normal PostgreSQL/API
parity tests pass both original and repaired code.

| Teams | Original data SELECTs/read | Batched data SELECTs/read |
| --- | --- | --- |
| 0 | 1 | 1 |
| 1 | 2 | 2 |
| 12 | 13 | 2 |
| 100 | 101 | 2 |

Data SELECT counts are identical across all three paths. The ordinary wrapper
has zero authority SELECTs; the protected wrapper has four (two session/user
checks, stored round parent, exact membership); HTTP adds the nonlocking session
extractor for five. These counts remain unchanged. `pg_stat_statements` also
records two transaction statements per read, BEGIN and COMMIT. Its utility
visibility is not a protocol-round-trip counter: `SET TRANSACTION` remains in
protected source but is absent from these recorded statistics.

Complete JSON is compared against an independent copy of the original ordered
per-team SQL outside timing. Cross-revision SHA256 hashes cover every response
field and array order. The fixture uses deterministic UUIDs and team timestamps,
0/1/12/100 teams, eight members on three of every four teams, empty teams,
duplicate/non-null positions, null positions, mixed-case native SQL collation,
duplicate display names and player-UUID tie-breaks. Team names are unique under
the existing `(round_id,name)` constraint; exact duplicate-name team ties are
unreachable, and the existing team UUID tie-break stays intact. A separate
ordinary PostgreSQL test checks same-tournament sibling rounds, a foreign
tournament's team and private HTTP 403 without cross-tournament leakage.

## Local cost and limits

The primary [baseline](baseline.json) and [current](current.json) captures run
serially after this task's acceptance ladder and cleanup, with the same synthetic
data, session lifetime, one-connection measured pool, PostgreSQL version/settings
and `en_US.utf8` database collation. Both probes use three warm-ups and 30 measured
reads per group. [comparison.json](comparison.json) verifies all 12 complete
response hashes, exact counts and settings before computing these milliseconds:

| Teams | Path | Median before → after, ms | p95 before → after, ms |
| --- | --- | --- | --- |
| 0 | internal | 0.442 → 0.431 | 0.689 → 0.592 |
| 0 | session | 0.929 → 1.023 | 1.319 → 1.241 |
| 0 | HTTP | 1.271 → 1.444 | 2.108 → 1.950 |
| 1 | internal | 0.766 → 0.866 | 1.121 → 1.523 |
| 1 | session | 1.290 → 1.801 | 1.736 → 2.596 |
| 1 | HTTP | 1.684 → 2.276 | 2.750 → 3.259 |
| 12 | internal | 4.016 → 1.544 | 4.443 → 2.049 |
| 12 | session | 4.502 → 2.236 | 5.733 → 2.838 |
| 12 | HTTP | 5.776 → 3.433 | 6.738 → 4.338 |
| 100 | internal | 21.755 → 6.596 | 25.386 → 7.346 |
| 100 | session | 25.356 → 7.394 | 32.894 → 8.323 |
| 100 | HTTP | 31.351 → 12.740 | 34.715 → 13.463 |

The many-team local medians/p95 improve; empty/single-team HTTP medians are
slower. Those small-case timings do not establish a product regression or an
overall speedup. No additional small-case optimization was selected.

The first current capture overlapped backend compilation and is used only for
counts/parity. A later serial attempt incorrectly shared Cargo's target directory
between the temporary baseline archive and this checkout. Cargo reused the
baseline test executable for the current command, which failed the query budget
with the original N+1 SQL. That current capture is discarded. `cargo clean -p
golf-api` removed this package's generated artifacts; current source was freshly
compiled in this checkout and passed before its final capture was accepted.
The valid serial baseline already compiled the verified original source. These
attempts, source/probe hashes, clean rebuild and all fixture cleanup confirmations
are retained in [validation-results.json](validation-results.json). The temporary
baseline source archive was removed. Reproduction must use distinct target dirs.

No current production frontend caller was identified in the original review.
Fewer data round trips do not establish hosted latency, throughput, a production
budget breach or browser speedup. The probe has one sequential reader, warm
caches, small synthetic datasets, an unoptimized debug build and no controlled
host load or interleaved randomized A/B. Empty/single-team cost must be reported
even where unchanged or slower. No load or memory-capacity claim follows.

## Validation and review

- Baseline query-budget regression: expected final assertion failure, six size/
  path cases; baseline payload contract passed.
- Repaired exclusive measurement: one passed, complete response hashes match.
- New ordinary PostgreSQL/API payload/isolation test: one passed.
- Existing pairing/team session suite: 14 passed, covering revocation,
  credential replacement, expiry during membership/member loading, empty-result
  final checks, cancellation, concurrent membership/snapshot changes, no writes
  and no SSE. The loading gates still block `team_memberships` after batching.
- `cargo fmt --all -- --check`: passed.
- `cargo test --locked --workspace --all-targets`: 264 passed, one existing
  manual projection benchmark ignored.
- `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`:
  passed; the same three pre-existing vendored SQLx warnings remain.
- `cargo test --locked --workspace --all-targets --features database-tests --
  --test-threads=2`: 933 passed, zero failed, five ignored (the four existing
  manual/exclusive probes plus the new explicitly run exclusive measurement).
  This includes the new ordinary payload/isolation test.
- `cargo run --locked -p golf-api --bin migrate` and `--bin seed`: passed;
  39 successful migrations. The uniquely owned acceptance container was removed.
- Independent read-only design/source review: no production finding; a P3
  rerun-harness stale-output issue was found and repaired by deleting only the
  task-owned `measurement.json` before each measurement command. A missing fresh
  output can no longer accept an unrelated cargo exit 101 as the baseline
  regression. Subsequent harness/draft-documentation review found no remaining
  findings. Final read-only review independently recalculated all 12 hashes,
  counts, medians/p95 and source/probe provenance, and found no issues in the
  evidence, documentation or scoped plan closure.

One new measurement test is intentionally ignored in the ordinary database
ladder because it resets shared `pg_stat_statements` and requires exclusive
ownership/explicit `GOLF_TEAM_MEASURE=1`; it is run explicitly here. Existing
exclusive match measurements and the manual Fantasy projection benchmark retain
their ignored status. No source failure is hidden by a new skip.

The initial probe setup assumed three recorded transaction statements for
protected reads. PostgreSQL reported two; that instrumentation assertion was
corrected before the original-loader regression was captured. It did not imply
a missing transaction isolation command or product defect. Both task-owned
fixtures from that setup and the completed baseline were removed.

Frontend/browser ladders are unaffected: no user-facing UI, API contract,
frontend source or cache policy changed. They were not rerun. No migration,
deployment configuration, operator procedure or hosted service was changed.
Hosted/device acceptance, course-read authority, driver/collation/test maintenance
and other performance work remain separately queued.

## Reproduction

The [runner](postgres_ladder.py) creates a unique container, records its CID
before start, binds an ephemeral loopback port, uses synthetic databases and
removes only that recorded CID in `finally`, even on failure. Raw logs stay under
the selected local output path; JSON records no database URL, credentials or
session token. Use rootless Podman and the existing `postgres:17-alpine` image.

```sh
python3 docs/validation/team-loading-2026-10-15/postgres_ladder.py \
  --phase current --output /tmp/golf-a8-current
python3 docs/validation/team-loading-2026-10-15/compare.py \
  --baseline /tmp/golf-a8-baseline/measurement.json \
  --current /tmp/golf-a8-current/measurement.json \
  --output /tmp/golf-a8-comparison.json
```

To capture the original loader, use a separate temporary checkout/archive of the
baseline revision, copy only the new team-loading test modules and this runner
into it, and run `--phase baseline --measurement-only`. Do not roll back the
working tree. Use distinct Cargo target directories for the two source trees;
do not share `CARGO_TARGET_DIR` across the baseline archive and current checkout.
`--phase current --measurement-only` runs only the exclusive probe
for a serial timing comparison after acceptance. The baseline runner requires a
fresh generated measurement file, all 12 groups and the six expected constant-
budget failures; any other failure aborts. [compare.py](compare.py) verifies
matching version/settings, sizes/paths/repeats, all payload hashes, exact old/new
data counts and unchanged authority/recorded transaction counts, then summarizes
medians and nearest-rank p95. It does not replace the retained raw samples.
