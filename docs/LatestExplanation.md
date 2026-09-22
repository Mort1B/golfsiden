# Match listings share one authorization context

**Completed — READY WITH KNOWN LIMITATIONS.** Full and player-filtered match
listings now resolve eligible player owners once inside their existing transaction.
Both opponents must be eligible, with unchanged role/round gates and full-card
projection. Scoring, detail and mutation authorization remain on their existing
paths. The [report](performance/listing-authorization/README.md) retains the measured
comparison, validation evidence and an unresolved transaction-start issue.

For an administrator reading 24 matches in an open, released final, repository
SELECTs fall 412→174 and returned owner IDs fall 2,304→48. The local warm median falls
41.25→15.26ms (candidate range 13.85–19.52ms). One filtered card falls 21→13 SELECTs
and 2.24→1.44ms median. At 100 matches the full-list count falls 1,704→706, with owner
rows 40,000→200. Baseline and candidate are separate sessions on the same configured
local environment, not an interleaved trial or production performance guarantee.
Exact query/row reductions reproduce in all 72 cases across three release runs.

The new context locks the same active session/user and required membership and
calls the existing eligible-owner resolver. It lives only within one listing
transaction. Empty lists skip owner resolution, but still authorize membership.
No card evidence, handicap, visibility, ordering, schema, API or cache change is
included. Card construction still performs seven queries per match.

Session expiry is checked again immediately before commit using database wall-clock
time, including empty results and the last card after materialization waits. This
closes a gap in the old per-card checks. Both new expiry regressions fail against
the old implementation and pass with the repair. Session/user/membership locks
remain held through assembly, preserving ordering against revocation/removal.

## Validation

- 208 ordinary backend and 587 PostgreSQL tests pass. Formatting, all-feature Clippy,
migrate and seed pass. Six new tests cover expiry, both lock orderings, independent
comparison with untouched scoring/detail reads, and one-eligible-opponent denial.
- All nine explicitly invoked release measurement tests pass: 72 cases, 2,160 warmed
samples and 216 fresh-connection first calls. Query/owner-row/category assertions and
semantic equality pass. Source hashes identify the actual measured candidate.
- Real Chrome/API validation initially passed 10/11 cases. A focused rerun of all
three history widths passed, including hidden/released payloads, SSE and persisted
pageshow. The eight other match workflows passed initially. Layout/interaction
assertions and screenshots were checked. Frontend source and assets are unchanged;
its unit/type/build ladder was not repeated.
- Read-only source, race-test and retained-evidence review found no listing-repair
blocker. Diff, source limits, Python syntax and documentation checks pass.

## Known limitation and next step

The initial desktop return-refresh failure was SQLSTATE 25001 during transaction
initialization, before the changed authorization logic. A standalone SQLx 0.8.6
probe reproduces this error after cancelling a deliberately delayed transaction
start, without calling application code. The browser error is consistent with that
existing dependency/transaction-management defect, but its exact cancellation and
connection-reuse sequence has not been traced. The focused rerun does not erase
the initial failure or establish that an HTTP 500 is its only possible effect.

This step does not repair that transaction-start risk. A bounded cancellation and
pooled-connection cleanup/isolation repair is queued ahead of the wider security
review. The disposable validation services are removed after completion.
