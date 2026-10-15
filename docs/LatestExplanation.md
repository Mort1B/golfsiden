# Latest iteration — match completion and table session authority

A1k repairs the match completion and tournament match-table GETs. A real
controlled wait on the final match query reproduced both original endpoints
returning 200 after session expiry. The same regression now returns private 401,
including genuinely empty results. Existing active session/user and exact
membership locks remain held while each repeatable-read attempt loads all facts
and projects the result; a final database wall-clock check runs before commit.

A `40001` snapshot conflict permits one fresh whole read. Stored context, exact
role, hidden-final policy, progress, participants and awards are discarded and
reloaded. An exhausted conflict maps to generic private 503 in these GET handlers.
Other errors, lookup order and payloads retain their meanings. The completion
projection is built before its final check. Card/list reads, transitions, commands,
shared authorization helpers and all sporting rules are unchanged.

For example, an administrator read queued on membership can lose that role while
final visibility changes from released to hidden. Its retry must use the new
policy: completion returns null progress/readiness, and the table excludes those
confirmed final awards. A table read waiting on awards keeps participant names
and award facts in the same snapshot; a subsequent GET sees a deliberate
concurrent name and confirmation change together.

Seventeen focused PostgreSQL/API cases passed, including expiry, empty results,
revocation/credential serialization, fresh retry authority/visibility, bounded
attempt counts, win/draw/loss units and shared ranks, lifecycle readiness,
coherent snapshots, cancellation and concurrent full JSON parity without writes
or SSE. Independent read-only design/source/test review found no actionable
issues. Workspace tests, strict Clippy and API binaries passed, as did all 1,133
frontend tests, typecheck, lint and build. Chrome passed both typed views at 320,
390 and 1280 px: loading, populated long names, private 503/read-only retry, empty,
denied and logout states, with no page errors, unexpected network failures or
mutations. The full PostgreSQL 17.10 Alpine ladder passed 900 tests with four
existing ignored probes/benchmarks; migrate and seed passed through unchanged
schema 39. See the [acceptance report](validation/match-summary-session-2026-10-15/README.md).

No schema, frontend source, dependency or hosting changes. Ordinary success adds
one session query per GET; no measured speedup is claimed. The final check closes
preceding waits, not expiry through commit/serialization/delivery. Two P2 routes
and three P3 course routes remain in the historical read inventory.
