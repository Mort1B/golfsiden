# Database authorization costs are now measured

**Completed — READY WITH KNOWN LIMITATIONS.** This iteration measures the unchanged player-filtered implementation at `c02059d`
using disposable PostgreSQL and an opt-in release-profile SQLx harness. It changes
no production queries, authorization, scoring, schema, API or frontend behavior.
The [measurement report](performance/database-authorization/README.md) records the
fixtures, normalized SQL statistics, raw timing samples and reproduction commands.

Across three exclusive release runs, 72 cases and 2,160 warmed samples reproduce
the exact query counts. A 24-match administrator listing performs 412 SELECTs and
returns 2,304 owner IDs through 48 repeated owner queries. Its warm median is 41.25ms
(range 38.60–64.51ms). The existing player filter reduces this to 21 SELECTs and 96
owner IDs, with a 2.24ms median (2.04–3.39ms). At 100 matches, a full administrator
listing returns 40,000 owner IDs and performs 1,704 SELECTs. These are measurements
of the current code, not a proposed optimization's speedup.

Full-card listings reuse mutation authorization for each opponent. An administrator
or scorer repeatedly resolves all eligible snapshot owners twice per selected
match. A linked player resolves only their flight's owners but repeats that work
for each attempted match; a viewer stops before owner enumeration. Filtering to
one card removes most full-list work but retains two whole-owner-set reads for a
privileged actor. Visibility changes projected facts; locked status suppresses
writable IDs after authorization rather than skipping those checks.

The supported next candidate is a listing-only shared authorization context:
resolve eligible owners once inside the same transaction and require both opponents
to belong to that set. Preserve live session/membership locking, flight/snapshot
semantics, independent writable rules, card construction and visibility. Explicitly
recheck expiry after waits/before commit; the current repeated checks use database
wall-clock time. Scoring and mutation paths remain separate. No proposed repair
has been implemented or measured, so this step claims no optimization speedup.

The fixture has two numeric notes and an accepted/confirmed early concession per
match. This focuses on authorization and permitted-projection behavior, not fully
populated 18-hole ledger processing. Fresh-connection first calls exclude connection
establishment and share warmed database/OS caches; they are not cold-storage
measurements. Repository elapsed time includes sequential database waits and card
construction, while SQL execution time excludes untracked planning, transaction
control and HTTP/browser work. The report gives descriptive ranges from one local
machine, not production latency or capacity guarantees.

## Validation

- 208 ordinary backend tests and 581 PostgreSQL tests pass, plus formatting and
all-feature Clippy. Migrate and seed pass. The three measurement tests are ignored
by the general suite and run explicitly, sequentially, on the exclusive server.
- All nine explicitly invoked release tests pass. The count audit verifies all
three runs and 216 fresh-connection samples in addition to the warmed samples.
- Every measured response matches its expected full-card result; filtered results
match the exact full-list subset and writable intersection. Hidden/released and
locked results remain correct. Removing membership denies both listing variants.
- Read-only review covers measurement isolation, semantic parity and attribution.
Python syntax and diff checks pass. Frontend/build/browser checks were not rerun:
no frontend or user-facing behavior changed.

The disposable service is removed after validation. The wider security review
remains queued after the separately approved performance work.
