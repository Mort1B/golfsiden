# Numeric score reads recheck session expiry after loading

A1g fixes a reproduced P2 gap in the ordinary numeric member-card, scoring-card
and shared score-access GETs. Before the change, all three could return 200 after
the session naturally expired during a membership wait. Existing locks protected
revocation and credential replacement, but time could advance while those locks
were held. The original regression now returns non-storeable 401 from each route,
without score writes or live events.

Each successful attempt checks database wall-clock session validity after its
complete payload is assembled, immediately before commit. This includes card
confirmation loading and member projection, full scoring summaries, four-ball
partner-to-team owner mapping, and viewer/unlinked-player/singles empty lists.
The boundary does not freeze time through commit or network delivery.

For example, an empty writable-owner list remains a successful authorized reply
for a valid viewer. If that session expires while waiting for membership, the
server now rejects the reply instead of returning a stale successful empty list.
An ordinary member card still uses preserved round handicaps and redacts a
hidden final's back nine exactly as before.

All three reads retain repeatable read and allow one fresh whole attempt after
`40001`. Existing scoring-card retry behavior is shared with the member read;
access owns a read-specific wrapper so shared mutation authorization is unchanged.
A second conflict yields generic non-storeable 503. No role, context or facts
survive the discarded attempt; other errors are not retried.

Selected GET middleware preserves `no-store` headers and fills missing ones with
`private, no-store`, including missing-resource errors. The existing 404 test now
asserts that deliberate contract. Mutation, conditional delivery, format-specific
card routes, formulas, schema and frontend source are unchanged. Each successful
selected read adds one final session query; no speedup or batching is claimed.

The 18 new PostgreSQL/API regressions cover authority writers, natural expiry at
late loading/empty branches, changed hidden-final context on retry, fresh ordering,
coherent scores/confirmation, cancellation, concurrent full JSON parity, nine-hole
foursomes snapshots, exact retry counts and no writes/SSE. Corrected focused
suites pass 58 tests. Independent read-only review found no actionable findings.
Formatting, the 261-test ordinary backend ladder, strict Clippy and binary build
pass. The full PostgreSQL ladder passes 831 tests, with four existing performance
probes ignored. Explicit migration and seed pass at schema 39; local health and
readiness return 200. The [validation report](validation/numeric-read-session-2026-10-14/README.md)
records the commands, initial failures and final evidence. Owned local validation
services were stopped and removed.

Chrome passes nine endpoint/width flows at 320, 390 and 1280px: loading, populated,
unentered/read-only fallback, denial, error and explicit read retry. Long names
wrap without horizontal overflow. Real typed decoders and 200/401 cache envelopes
pass; no score writes, page exceptions or unexpected network failures occur.
The report distinguishes injected UI failures from real database authority races,
retaining inspected screenshots and initial fixture corrections.

Ten P2 and three P3 routes from the historical A1f inventory remain. Physical
devices, public hosting, production load and other queued findings keep their
acceptance limits. [PLANS.md](PLANS.md) selects the next bounded candidate;
no queued repair is part of A1g.
