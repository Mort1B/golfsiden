# Latest iteration — match card session authority

A1j repairs the member and scoring match card GETs. The controlled final-notes
regression reproduced both original endpoints returning 200 after session expiry.
Both now return private 401. Existing session/user and exact membership locks
remain held while the repeatable-read transaction constructs the full card, then
an existing database wall-clock authority check runs before commit.

A `40001` snapshot conflict permits one complete fresh read. Context, roles,
match/opponent identity, scoring permission, ledger, frozen handicaps and notes
are discarded and reloaded. An exhausted conflict maps to generic private 503
only in these two handlers; unrelated errors retain their existing mapping.
Commands, listing, completion, table and shared helpers keep their contracts.

For example, an administrator card read queued on membership can lose that role
while final visibility changes from released to hidden. Its retry now uses the
new member authority and hidden context, returning only the permitted front nine;
the scoring variant denies. A queued read that expires while loading the final
numeric notes returns 401 instead of a successful old card.

Eighteen new PostgreSQL/API cases cover these races, bounded retry counts,
revocation/credential serialization, both-opponent/flight authority, hidden
noninterference, match lifecycle/result states, accepted report versus numeric
note semantics, coherent aggregate/note snapshots, cancellation and concurrent
full JSON parity without writes or SSE. Independent read-only source/test review
found no actionable issues. Workspace tests, strict Clippy and API binaries
passed, as did all 1,133 frontend tests, typecheck, lint and build. Chrome checked
both typed card views at 320, 390 and 1280 px: loading, populated long names,
private 503/read-only retry, empty, denied and logged-out states, without page
errors, unexpected network failures or match mutations. The full PostgreSQL
17.10 Alpine ladder passed 883 tests with four existing ignored probes/benchmarks;
migrate and seed passed through unchanged schema 39. See the
[acceptance report](validation/match-card-session-2026-10-15/README.md).

No schema, frontend source, sporting rules, dependencies or hosting changes.
Ordinary success adds one session query; no measured speedup is claimed. The
check closes preceding waits, not expiry through commit/serialization/delivery.
Four P2 routes and three P3 course routes remain in the historical read inventory.
