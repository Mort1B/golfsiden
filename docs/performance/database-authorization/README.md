# Database authorization measurement

This step measures the unchanged production implementation at `c02059d`. It adds
an opt-in ignored SQLx benchmark and retained evidence, not an authorization repair.
The [filtered history repair](../history-filter/README.md) already limits selected
history to one full card per round; the current investigation measures the SQL
work still performed inside each listing.

## Method and boundaries

The benchmark calls the real `match_play::reads::list` / `list_for_player`
repositories. Each call retains its repeatable-read transaction, live-session and
membership locks, per-opponent mutation authorization, full-card construction and
visibility projection. Repository elapsed time includes pool acquisition, sequential
client/server waits, row decoding, ledger validation and card construction. It
excludes HTTP session extraction, HTTP transport, JSON serialization and browser
work. PostgreSQL SELECT execution time excludes transaction control and untracked
planning and is recorded independently; subtracting
it from repository time does not isolate network latency.

An exclusive rootless PostgreSQL 17 container uses tmpfs data on loopback TCP with
`shared_preload_libraries=pg_stat_statements`, top-level tracking and I/O timing.
Retained settings include shared buffers, work memory, JIT and planning tracking.
The host has an Intel Core i5-1135G7. The Rust harness uses the default optimized
release profile and current locked dependencies. No heavy validation jobs run
concurrently with final measurement. This is a local descriptive benchmark, not
a production capacity or latency guarantee.

Each synthetic tournament has one 18-hole final match round and 12/24/100 matches,
with two preserved player snapshots and one two-person flight per match. Handicaps
vary from 0 to 19. Every match has two numeric notes (holes 1 and 10), an accepted
pre-hole concession, and confirmation through the real command path. This makes
hidden/released metadata and note projection observable without making accepted
ledger decoding dominate an authorization-focused experiment. It is not a fully
populated 18-hole scoring workload; that workload can change absolute time and
proportions. The 100-match case is a stress fixture, not a capacity commitment.

Administrator, scorer, linked player and viewer read the full list and a selected
card. An additional linked-player case selects another flight's card, which remains
readable but not writable. Hidden and released finals run at all sizes; 24 matches
also exercises locked rounds. The linked player owns the first match and can write
both opponents in that flight. An artificial single giant flight would produce a
different owner-set cost and is not used.

All 72 cases run three times. Each case opens a fresh single-connection pool and
times its first repository call after connection establishment. The database has
already been populated/read: shared buffers and OS cache are not flushed. These
are **fresh-connection first calls, not cold-storage measurements**. A separate
one-connection pool runs three untimed warmups followed by ten measured reads.
JSON conversion and exact-result equality assertions occur after each timer stops.
Prepared statement caches accumulate across cases. Role/state order is fixed,
with full/filtered order alternating by role; this is not randomized sampling.

`pg_stat_statements_reset()` runs outside the timer, immediately before the ten
warmed calls. Statistics are read afterward for the exact test database, excluding
instrumentation. Tests and repetitions run sequentially because reset is global
across databases. No other database workload runs during final collection.
Normalized SQL, calls, returned rows, execution time and shared-buffer counts are
retained. SELECT counts exclude transaction control; direct repository calls omit
the one additional HTTP authentication SELECT. Returned owner rows are materialized
query results, not scanned rows or bytes transferred.

## Semantic and count checks

Before timing, filtered cards and writable IDs must exactly equal filtering the
unfiltered result. Every timed response must equal the established response.
Card counts, hidden confirmation metadata and writable role/state results are
asserted. After measurement, removing membership from the still-live linked
session must deny both listing variants. Existing full PostgreSQL tests separately
cover broader expiry, revocation, visibility and command concurrency behavior.

The summarizer fails unless all three runs have the full case matrix and exact
predicted SELECT/owner-row counts. With K selected matches (M for full listing,
1 for this filter), the predictions are:

| Actor | Repository SELECTs | Owner-set queries | Returned owner IDs |
| --- | ---: | ---: | ---: |
| Admin/scorer |4+17K|2K|2K×2M|
| Viewer |4+12K|0|0|
| Linked player, full list |8+13M|M+1|2(M+1)|
| Linked player, own match |21|2|4|
| Linked player, another flight |17|1|2|

Four initial SELECTs resolve round, live session, membership and ordered match IDs.
Each card then resolves its aggregate and invokes authorization. Admin/scorer
checks both opponents, repeatedly retrieving every round snapshot owner. A linked
player stops after the first denied opponent; a viewer returns no owner set before
that query. Locked status suppresses writable IDs after authorization, so it does
not eliminate those queries. Visibility affects the permitted card projection,
not authorization counts.

## Reproduction

Use only a dedicated disposable server. The benchmark resets instance-wide
statistics; its opt-in is not permission to run against a shared database.

Start PostgreSQL 17 with the settings above, configure `DATABASE_URL` privately,
and set `GOLF_AUTH_MEASURE=1`. From the repository root:

```sh
python3 docs/performance/database-authorization/run.py /tmp/golf-auth-final
python3 docs/performance/database-authorization/summarize.py /tmp/golf-auth-final docs/performance/database-authorization
```

The runner creates a new output directory and runs all three ignored SQLx tests
with `--release --ignored --test-threads=1`. SQLx gives each size an isolated
migrated database. Only synthetic identities are used. Stop/remove the disposable
container afterward. The evidence records source/test hashes, Rust and PostgreSQL
versions, settings, normalized queries and all raw timing samples.

## Results

**READY WITH KNOWN LIMITATIONS.** All three exclusive release runs pass: 72 cases,
2,160 warmed samples and 216 fresh-connection first calls. Exact SELECT counts,
owner-query calls/rows and session/membership/round lookup counts reproduce across
all three runs. Each timed result passes semantic equality; all three fixture sizes
reject both listing variants after membership removal. Retained PostgreSQL is 17.10,
with 128MiB shared buffers, 4MiB work memory, JIT on, planning tracking off and I/O
timing on.

These are open, released-final cases. Warm medians/ranges combine 30 samples across
three runs. Admin/scorer query counts are identical; the table reports admin times.
All role/visibility/state timings remain in [summary.json](summary.json) and
[evidence.json](evidence.json).

| Matches | Admin full SELECTs | Owner IDs returned | Warm full median [range], ms | Filtered SELECTs / owner IDs | Warm filtered median [range], ms |
| ---: | ---: | ---: | ---: | ---: | ---: |
|12|208|576|21.79 [19.16–27.95]|21 /48|2.15 [1.91–3.40]|
|24|412|2,304|41.25 [38.60–64.51]|21 /96|2.24 [2.04–3.39]|
|100|1,704|40,000|208.44 [195.05–236.43]|21 /400|2.83 [2.26–4.22]|

For 24 matches the full linked-player read executes 320 SELECTs, returns 50 owner
IDs and takes 37.30ms median; the viewer executes 292 SELECTs, returns no owner set
and takes 24.53ms. A linked player's own filtered match executes 21 SELECTs and
returns four owner IDs; another flight's card executes 17 and returns two, with
no writable ID. Hidden/released and locked cases retain the predicted query counts;
locked non-admins still perform that work before receiving no writable IDs.

The 24-match administrator listing repeats live-session and membership reads 73
times each, round→tournament authorization lookup 48 times, and owner enumeration 48
times. The 100-match equivalents are 301/301/200/200. One filtered privileged card
still performs 4/4/2/2 respectively. This confirms repeated authorization work,
not merely redundant transfer of card data.

At 24 matches, average PostgreSQL SELECT execution per administrator full read has
a median of 6.83ms across runs, of which 2.23ms belongs to owner enumeration. At 100,
those values are 51.88ms and 29.06ms. These server figures are averages per warmed
batch, then medians across runs; they are not the per-call repository medians and
are not directly subtractable to label the remainder as network time.

Fresh-connection first-call medians for admin full/filtered are 40.55/15.31ms at 12,
66.21/14.46ms at 24, and 226.18/16.05ms at 100. First calls include statement preparation
but exclude connection establishment, and are not always slower in every role
(for example, the 100-match linked-player case). Different statement-cache/plan
states, fixed ordering and local scheduling limit causal timing comparisons.

The filtered endpoint already avoids almost all card-count growth. The remaining
privileged owner-row volume still grows with the total roster even for one card,
and full listing owner enumeration grows quadratically when players=2×matches.
This motivates a narrowly scoped shared authorization context. It does not justify
relaxing privacy or demonstrate how much a proposed repair would improve latency.

## Candidate boundary

The repeated owner-set resolution supports a separately approved **listing-only
shared authorization context**. Resolve eligible player owners once inside the
same listing transaction, then require both opponents to be eligible. Target one
owner-set query per listing instead of two per authorized card, while keeping full
card construction, ordering, projection and independent writable status rules.
Do not use `writable_owners`, which opens another transaction and deliberately
returns no singles-match owners. Mutations and scoring reads stay on their existing
paths; bulk card loading is separate work.

A repair must preserve locked live-session/membership checks, linked-player and
flight scope, snapshot ownership and fail-closed errors. Current repeated session
checks evaluate expiry with `clock_timestamp()`; consolidating them requires an
explicit expiry recheck after waits/before commit rather than silently extending
authority until a transaction finishes. Require parity and expiry/revocation/lock
race tests, unchanged private API output, and a rerun of these counts/timings before
claiming an improvement. This report establishes repeated work; it does not prove
a proposed implementation's speedup.

## Validation

- Backend formatting, 208 ordinary tests and all-target/all-feature Clippy pass.
- Full PostgreSQL ladder: 581 tests pass; the three measurement tests are
  intentionally ignored there and exercised explicitly in the exclusive release
  runs. Migrate and seed pass on the disposable base database.
- Independent read-only review checks fixture validity, transaction/authority
  boundaries, count predictions, global-reset isolation and timing attribution.
- Python syntax and diff checks pass. Production backend/frontend and migrations
  are unchanged; frontend tests/build and browser checks are not applicable to
  this standalone measurement/documentation step and were not rerun.

## Limits

No authorization optimization has been implemented or benchmarked. Three sequential
runs on one local machine do not establish production tail latency, concurrency
throughput, cold-disk behavior or performance at arbitrary roster/flight sizes.
Connection establishment, TLS, HTTP authentication/serialization and frontend
rendering are excluded. Statistics tracking itself has overhead. Statements are
executed serially over loopback; there is no independent remote-database latency
experiment. Owner rows are returned rows, not scanned rows or transfer bytes.
The larger event/notes payload and decoding costs measured in the prior browser
report are a different workload and must not be added/subtracted from these times.
