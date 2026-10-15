# Session-bound match completion and table reads — 2026-10-15

A1k closes the two remaining P2 [match summary boundaries](../private-read-inventory-2026-10-14/README.md#match-details):
`GET /api/rounds/{round}/match-play/completion` and
`GET /api/tournaments/{tournament}/match-table`.

## Original failure and repair

The [original regression](baseline.txt) holds `singles_matches`, starts each GET
and observes its real final-loader wait while the session is active. It waits
for natural expiry against the database clock, then releases the unchanged lock.
Both original GETs return **200 after expiry**. The same regression now returns
**401 unauthenticated** with `private, no-store`, unchanged sporting data and no
SSE. Retained evidence includes synthetic paths/statuses/error codes only.

Each public repository entry retains its existing `Result<_, Error>` contract
and owns a private complete attempt (`get_once` / `table_once`). After complete
progress or participant/award loading and projection, the existing `live` helper
checks session validity against database wall-clock time before commit. Existing
session/user and exact membership locks stay held. Completion is constructed
before the final check. Both empty-match completion and truly empty tables retain
that final authority boundary. This closes expiry during preceding waits; it
does not freeze time through commit, serialization or delivery, or retract a
response already delivered.

Only database SQLSTATE `40001` permits one fresh whole attempt. Context, exact
role, visibility and all facts are discarded and reloaded. An exhausted conflict
remains a repository database error, mapped to generic `503 service_unavailable`
by the existing GET-only mapper now used by these two handlers. Other errors and
common all-status cache middleware are unchanged. Completion still resolves its
stored round/format before membership; table still checks tournament existence
first. Their 404/403 distinctions remain.

Projection rules are unchanged: completion hides terminal/confirmed and readiness
metadata with nulls for a hidden final; the table excludes that final's awards
for non-admins. Empty completion is not ready. Participants without confirmed
visible awards remain unranked, and no participants means an empty entries array.
Exact half-units, ties/ranks, accepted win/draw/loss counts, deterministic ordering
and open/completed/locked readiness remain. Shared helpers, transition policy,
mutations, match card/list reads and sporting rules are unchanged. No schema,
frontend source, dependencies or hosting changes. Each ordinary success adds one
session query; no measured speedup is claimed.

## Regression coverage

Seventeen new PostgreSQL/API cases ([focused result](focused-tests.txt)) reuse
unchanged match-card and Stableford fixture support. The new empty fixture uses
ordinary constrained inserts; no production timing/failure hooks were added.

- Natural expiry during exact membership and final progress/award queries, both
  GETs, including genuinely empty tables and match sets. Eight-second setup
  windows require observed blocking and an active session before natural expiry.
- Revocation/credential replacement after extraction but before repository
  authority, later writers serialized behind valid readers and fresh denial
  after membership removal. The global-admin fixture has no membership bypass.
- A real released-to-hidden visibility change plus admin-to-viewer demotion
  invalidates the first snapshot. Both retry successfully with the new permitted
  view: completion progress/readiness null, all confirmed final awards excluded.
  Populated confirmed matches establish the before/after difference.
- Persistent synthetic `40001` and unrelated `XX000` injected only in isolated
  database views. Nontransactional sequence counters prove two versus one attempts,
  private generic 503 versus 500, no payload/database-detail escape, unchanged
  sporting state and no SSE.
- Full JSON parity for both endpoints, including unstarted and terminal-but-
  unconfirmed matches, a confirmed concession win and a confirmed 18-hole draw.
  Independent assertions establish 2/1/1/0 half-units, ranks 1/2/2/4, win/draw/loss
  counts, match UUID ordering, unranked versus truly empty and unchanged fixed
  sporting state after global handicap edits. Twelve concurrent reads retain the
  complete payloads without writes or SSE.
- Open readiness is true only after both terminal confirmations; completed is
  ready to lock, locked is ready for neither transition. These tests use the
  unchanged authorized transition paths.
- All four exact membership roles, hidden progress/awards, missing/invalid
  identities and wrong-format completion precedence are preserved.
- At the final loader, a deliberate constrained confirmation changes the match
  revision/audit/receipt and a participant name together. Pending reads retain
  coherent old progress or participant/award facts; fresh reads see the new
  confirmation and name. The fixture retains the database workflow guards.
- Cancellation releases authority locks and fresh reads succeed. Snapshots
  compare match aggregates, notes, audits, immutable receipts, fixed handicaps and
  legacy scores; subscriptions assert no read SSE.

Independent read-only design and final source/test review found no actionable
issues. Review executed no tests or database commands; runtime evidence belongs
to the primary agent.

## Validation

| Check | Result |
| --- | --- |
| Original final-loader regression | Expected failure: both GETs 200 after expiry |
| Focused PostgreSQL/API suite | 17 passed |
| Formatting | Passed |
| Workspace tests | 261 passed, one existing ignored benchmark |
| Strict all-target/all-feature Clippy | Passed; three unchanged vendored SQLx warnings |
| API binaries | Built |
| Full PostgreSQL 17 Alpine suite | 900 passed, four existing ignored; exit 0 |
| Full migrate/seed | Passed; 39 migrations current, seed completed |
| Frontend unit tests | 147 files, 1,133 tests passed |
| Frontend typecheck / lint / build | Passed |
| Local Chrome | Both views at 320, 390 and 1280 px passed |
| Read-only design/source/test review | No actionable findings |

The [validation summary](validation-results.txt) records complete commands and
results. Database-feature totals include the ordinary backend tests; the counts
are not additive. The four ignored cases are the existing projection benchmark
and three PostgreSQL profiling probes. No test was newly skipped.

Frontend source and schemas are unchanged; the full frontend ladder still ran.
The disposable PostgreSQL is 17.10 Alpine/musl with `en_US.utf8`. The existing
native-collation portability test issue remains separately queued; no collation
or unrelated test contract changed.

## Browser acceptance

The [probe](browser-probe.mjs) creates and confirms a synthetic match using
existing authorized flows before observing the GETs. The [results](browser-results.json)
cover six view/width cases and three logout checks:

- Real 200s retain exact `private, no-store`; the actual `matchApi` runtime decoder
  accepts completion readiness and two table participants, with 2/0 half-units
  and ranks 1/2. Long names wrap without horizontal page overflow.
- A controlled wait shows loading. Completion stays disabled while pending.
  Injected private 503 shows an error, leaves completion disabled and offers
  **Prøv igjen**, which retrieves valid data without a mutation.
- Empty completion injection shows 0 of 0 and disables completion; an empty table
  shows its existing no-players message. Initial private 403 shows the error,
  disables completion and leaves the table absent.
- Real logout produces private 401 on both GETs; the results URL redirects to
  login. No round/tournament mutations, page errors or unexpected network failures.

Viewport captures were visually checked:
[completion error at 320 px](completion-error-320.png),
[populated table at 390 px](table-populated-390.png) and
[empty completion at 1280 px](completion-empty-1280.png).
Captures are of particular viewports, not every scrolled control, physical-device
geometry or native zoom. Empty/error/denied injections test these views while the
rest of the synthetic fixture remains real. PostgreSQL regressions separately
prove genuinely empty datasets, natural expiry and exhausted-conflict mapping.

## Limits and remaining work

This closes these two GETs only. Two P2 routes remain from the historical source
inventory: invitation metadata (1) and user-only final-visibility metadata (1).
Three P3 course GETs remain. The historical A1f manifest and source anchors keep
their original revision; remaining findings have no new race reproduction here.
Legacy write expiry remains separately queued. Match listing and card reads were
already session-bound and are unchanged.

Browser fixtures use synthetic local data. Failure/empty/denied UI responses are
explicit injections; PostgreSQL tests independently prove expiry and conflict
mapping. No physical-device, load-capacity or public-host readiness claim is made.
The broader security/performance findings remain in the
[original review](../application-review-2026-10-14/README.md).
