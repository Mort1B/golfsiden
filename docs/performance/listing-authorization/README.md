# Listing-only shared authorization

This repair uses one transaction-local authority context for full and
player-filtered match listings. It reuses the existing eligible-owner resolver
without changing scoring or mutation authorization. The earlier
[database measurement](../database-authorization/README.md) and its raw evidence
remain unchanged as the baseline.

## Behavior and safety boundary

The listing resolves the stored round first, locks the active session/user and
required tournament membership, and selects ordered match IDs. For a nonempty
listing it resolves the eligible player-owner set once; both opponents must be in
that set, and the existing draft/locked status rules still determine whether a
match is writable. Viewer/unlinked-player owner resolution remains empty. Empty
listings skip owner resolution but still require membership.

Cards use the unchanged aggregate, handicap, ledger, holes, notes and visibility
builders. Hidden metadata, preserved handicaps, ordering, API shape and cache
identity do not change. The authority context never leaves the request or lives
beyond its transaction. It is not the separately transactional `writable_owners`
API and is not used for scoring, detail reads or commands.

Immediately before commit, the listing checks the locked live session again using
PostgreSQL `clock_timestamp()`. This covers expiry during owner/card loading or a
membership wait, including the last card and empty results. It closes a gap in the
previous per-card checks, which could finish before later materialization waited.
The final check does not replace the initial admission or release any existing
session/user/membership locks. An admitted read can finish before a waiting revoke
or membership removal commits; subsequent reads deny access. A read waiting behind
an already-changing authority row fails closed (a repeatable-read serialization
conflict is allowed), followed by typed denial on a fresh read.

## Measurement procedure

Run the unchanged ignored release harness from the preceding measurement with the
candidate implementation. Same 72 cases, three runs, ten warmed samples after three
warmups per case, and one fresh-connection first call per case/run. Sizes are
12/24/100 matches; roles are admin/scorer/linked player/viewer plus a linked-player
other-flight filter; finals are hidden/released, with locked 24-match cases. The
fixture has two notes and one confirmed early concession per match. It is an
authorization-focused workload, not fully populated 18-hole ledger processing.

The dedicated PostgreSQL 17 container uses loopback TCP and tmpfs, top-level
pg_stat_statements and I/O timing. Release builds use the default Cargo profile.
No heavy checks run during timing. Database/OS caches are not flushed. New-connection
first-call timing excludes connection establishment; it is not cold-storage timing.
The warmed connection accumulates prepared statements across cases. Repository
elapsed time includes pool acquisition, sequential waits and card construction;
SQL SELECT execution time excludes transaction control and untracked planning.
Neither includes HTTP/browser work or JSON serialization in the timer.

The baseline and candidate are separate measurement sessions on the same host
(Intel i5-1135G7) with the same configured conditions and identical harness/fixture.
They are not an interleaved controlled trial. Treat timing differences as local
descriptive evidence, not production speedup or a causal decomposition. Exact query
and owner-row reductions are independently audited and do not depend on timing noise.
The retained candidate identifies pre-publication HEAD as `baseCommit` and the
actual measured implementation with production/test source hashes.

The new summarizer requires exactly two session SELECTs, one membership SELECT,
no per-opponent round lookup and one eligible-owner query per applicable listing.
For K selected cards: 6+7K SELECTs for eligible roles, 5+7K for viewers/unlinked players,
and 5 for empty listings (covered by source/tests, outside the timing matrix).
Both opponents are still checked. Owner rows fall to the single resolved set:
2M for an admin/scorer, two for the linked-player fixture, zero for a viewer.

## Results

**READY WITH KNOWN LIMITATIONS**, including the unresolved transaction-start issue
below. All 72 cases reproduce the expected query counts across three exclusive
release runs (2,160 warmed and 216 fresh-connection samples). Every measured response
passes semantic equality. Session checks are exactly two per listing, membership
one, and per-opponent round lookups zero. Eligible-owner enumeration is one query
per nonempty eligible listing, including locked rounds; viewers perform none.

The following rows are administrator reads of an open, released final. Warm ranges
cover 30 candidate samples; baseline and candidate were measured separately.

| Matches | Full SELECTs, before→after | Owner rows, before→after | Full warm median, before→after | Candidate full range | Filtered SELECTs, before→after | Filtered warm median, before→after |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
|12|208→90|576→24|21.79→8.13ms|7.16–10.43ms|21→13|2.15→1.71ms|
|24|412→174|2,304→48|41.25→15.26ms|13.85–19.52ms|21→13|2.24→1.44ms|
|100|1,704→706|40,000→200|208.44→63.61ms|49.50–79.39ms|21→13|2.83→1.70ms|

For 24 matches, owner enumeration falls 48→1 calls; at 100 it falls 200→1. A privileged
filtered read goes 2→1 owner queries, with 96→48 returned owner IDs at 24 matches.
The remaining set still scales with roster size, but it is no longer reloaded
for each opponent. All linked-player and viewer cases preserve the expected writable
results. Empty listing initialization deliberately adds a final expiry check rather
than claiming every request does less work.

[comparison.json](comparison.json) retains every baseline/candidate case side by
side. [summary.json](summary.json) retains medians/ranges and query categories;
[evidence.json](evidence.json) retains all timing samples, normalized statement
statistics, PostgreSQL settings and source hashes. The timing improvement is local
descriptive evidence under these conditions; the exact query/row reductions are
the stronger reproducible result.

## Validation

Six new PostgreSQL tests cover independent comparison with untouched scoring
reads, full-card equality with untouched detail reads, unlinked players and linked
viewers, and defensive incomplete-flight data with only one eligible opponent in
either slot. The incomplete-flight fixture disables only the open-pairing guard
inside an isolated setup transaction and restores it before reading; ordinary
production writes cannot create that state.

Controlled lock tests cover last-card and empty-list expiry, admitted reads holding
session/membership locks through materialization, and the opposite ordering where
revocation/removal commits before a waiting read resumes. Both expiry regressions
fail against the prior implementation and pass with the repair. The existing
listing suite covers draft/open/completed/locked states, hidden/released/corrected
finals, missing players, frozen handicaps and authenticated private/no-store APIs.

The retained [validation record](validation.json) includes the original failure
and the focused rerun without treating the diagnostic probe as a cleanup fix.

Validation ladder: 208 ordinary backend tests and 587 PostgreSQL tests pass, plus
formatting, all-target/all-feature Clippy, migrate and seed. The three release
measurement tests are intentionally ignored by the general suite and invoked
separately. Frontend source/contracts and assets are unchanged, so its unit/type/
build ladder was not rerun. Actual Chrome/API tests exercised the candidate backend:
the initial run passed 10/11 cases; see the unresolved issue below. All three filtered
history cases passed on the focused rerun, including hidden/released payloads,
SSE and persisted-pageshow refresh at 320/390/1280px. The eight other match cases
passed initially. Screenshots and layout/interaction assertions were checked.

## Known transaction-start cancellation issue

**Subsequent repair:** the [transaction-cancellation report](../transaction-cancellation/README.md)
records the ordinary-BEGIN and HTTP-disconnect reproducers and the driver
backport. The account below preserves this listing iteration's original evidence
and limitation; its passing rerun was not itself a repair.

One desktop return-refresh check encountered SQLSTATE 25001 (`SET TRANSACTION
ISOLATION LEVEL must be called before any query`) before listing authorization
began. This is consistent with an independently reproduced SQLx 0.8.6
transaction-start cancellation defect. The precise browser cancellation sequence
remains unconfirmed. This step does not repair that existing transaction-management
risk, and the passing focused rerun does not erase the initial 10/11 result.

The retained [standalone probe](cancellation-probe.rs) uses only SQLx on the
exclusive disposable database. It deliberately widens the transaction-start window
with `BEGIN; SELECT pg_sleep(0.4)`, cancels that future after 100ms, then runs a pooled
query and a new transaction/isolation request. It reproduces 25001 without calling
any application authorization code. In the installed SQLx source, transaction depth
is recorded after readiness is awaited, while cancellation rollback requires a
positive recorded depth. This supports a dependency-level diagnosis; it is not a
trace of the browser's exact sequence or proof that 500 is the only possible effect.
The probe asserts the defect and is retained as diagnostic evidence outside the
normal test suite, not as a regression test asserting correct behavior.

A separately bounded cancellation/pooled-connection repair is queued before the
wider security review. It must prove rollback or connection retirement and correct
cross-request isolation after interrupted transaction initialization, rather than
suppressing the observed error or disabling privacy refreshes.

## Reproduction

Start an exclusive disposable PostgreSQL server using the baseline report's settings.
Configure its `DATABASE_URL` privately and set `GOLF_AUTH_MEASURE=1`. Run from the
repository root after heavy validation completes:

```sh
python3 docs/performance/database-authorization/run.py /tmp/golf-list-auth-final
python3 docs/performance/listing-authorization/summarize.py /tmp/golf-list-auth-final docs/performance/listing-authorization
```

The statistics reset is global, so never run against a shared server or concurrently
with other database tests. Stop/remove the disposable server after validation.
The historical baseline scripts/artifacts have not been rewritten.

## Limits

No bulk card loading, parallel SQL, changed scoring rules, schema changes or wider
security repair is included. Card construction still costs seven SELECTs per match.
The whole tournament match table is unchanged. Local release measurements do not
establish production tail latency, concurrent throughput, remote database behavior,
cold caches, physical-device performance or an improvement for every workload.
