# Session-bound match card reads — 2026-10-15

A1j closes the two P2 [match detail boundaries](../private-read-inventory-2026-10-14/README.md#match-details):
`GET /api/rounds/{round}/match-play/matches/{id}` and its `/scoring` variant.

## Original failure and repair

The [original regression](baseline.txt) holds `singles_match_notes`, starts each
GET and observes its real PostgreSQL wait while its session is active. The test
waits for natural expiry against the database clock, then releases the unchanged
relation lock. Both original GETs return **200 after expiry**. The same test now
returns **401 unauthenticated** on both, with `private, no-store`, unchanged
match/note/audit/receipt/handicap/score state and no SSE. Retained evidence contains
only synthetic paths/statuses/error codes, with no payloads or credentials.

Each successful repeatable-read attempt now calls the existing `live` authority
check after full card construction, including the final notes query and derived
visibility/result metadata, immediately before commit. Existing session/user and
exact membership locks stay held. This closes expiry during preceding waits; it
does not freeze time through commit, serialization or delivery, or retract a
response already delivered.

The public repository `get` retains its `Result<Card, Error>` contract and wraps
private `get_once`. Only database SQLSTATE `40001` allows one fresh whole attempt.
Stored context, session, membership, aggregate, both-opponent scoring permission,
frozen handicaps, holes, names, notes and result projection are all reloaded. A
second conflict remains a repository database error and is translated to generic
`503 service_unavailable` only by the two card GET handlers. Unrelated errors use
the unchanged mapper. Common all-status `private, no-store` middleware is retained.

Lookup order remains stored round/format, active session, exact membership,
stored match, then scoring editability and both-opponent authority. Wrong format
remains 404 before membership resolution. Draft member cards can succeed with
unfrozen handicaps; draft scoring remains 403. Completed scoring remains usable;
locked scoring is exact-admin only. Payloads, ordering, accepted ledger semantics,
concessions/rulings, half-points, fixed handicap rules and hidden front-nine
projection are unchanged. Shared context/authority/build and command helpers,
listing, completion and table retain their own contracts. No schema, frontend
source, dependencies, sporting rules, mutation retries or hosting changes. Each
ordinary successful read adds one session query; no measured speedup is claimed.

## Regression coverage

Eighteen new PostgreSQL/API cases ([focused result](focused-tests.txt)) reuse
the existing fixture without changing it:

- Expiry during exact membership and final notes waits, both GETs. Eight-second
  setup windows verify real blocking and an active session before waiting for
  unchanged natural expiry; no production timing hooks.
- Revocation and credential replacement after extraction but before repository
  authority, later writers serialized behind valid readers, and membership
  removal retried with fresh denial. The admin fixture is also a global admin;
  missing exact membership grants no bypass.
- Real released-to-hidden final visibility and admin-to-viewer demotion during
  snapshot invalidation: fresh member projection hides back nine and scoring
  denies. A separate retry reloads changed names while retaining fixed handicaps
  despite changes to current global handicap.
- Persistent synthetic `40001` and unrelated `XX000`, injected only in isolated
  database views. Nontransactional sequence counters prove two versus one attempts,
  generic private 503 versus 500, no data/database-detail escape, unchanged
  sporting state and no SSE.
- Full JSON parity for both projections through numeric notes, accepted numeric
  reports, subsequent differing notes, concessions, terminal-unconfirmed,
  confirmed, organizer correction and reconfirmation. Independent expectations
  assert accepted lead, preserved +40 handicap, relative allocation, correction
  state and 2/0 versus 0/2 half-units. Twelve concurrent full JSON comparisons
  preserve the final result after global handicap edits.
- Exact roles, linked/unlinked player and own/other-flight scoring, draft/format/
  completed/locked behavior, missing/invalid identities and adjacent command
  authentication/cache behavior. An isolated defensive pairing fixture proves
  one eligible opponent cannot authorize scoring for the other.
- Hidden terminal/concession/award/confirmation changes leave the entire member
  JSON unchanged, with hidden metadata null and accepted IDs/revision omitted.
- Final notes loading retains coherent old aggregate revision and notes while a
  deliberate concurrent note command changes both; a fresh GET sees the changed
  revision/note. The fixture retains database workflow guards and matching
  aggregate audit/receipt. Cancellation releases retained authority and fresh
  reads succeed.

State snapshots include match aggregates, numeric notes, audits, immutable
receipts, fixed handicaps and legacy scores; subscriptions assert no read SSE.
Independent read-only design and final source/test review found no actionable
issues. Review did not execute tests; runtime evidence below belongs to the
primary agent.

## Validation

| Check | Result |
| --- | --- |
| Original final-note regression | Expected failure: both GETs returned 200 after expiry |
| Focused PostgreSQL/API suite | 18 passed |
| `cargo fmt --all -- --check` | Passed |
| `cargo test --workspace --all-targets` | 261 passed, one existing ignored benchmark |
| Strict all-target/all-feature Clippy | Passed; three unchanged vendored SQLx warnings |
| API binaries | Built |
| Full PostgreSQL 17 Alpine suite | 883 passed, four existing ignored; exit 0 |
| Full migrate/seed | Passed; 39 migrations current, seed completed |
| Frontend unit tests | 147 files, 1,133 tests passed |
| Frontend typecheck / lint / build | Passed |
| Local Chrome | Both projections at 320, 390 and 1280 px passed |
| Read-only design/source/test review | No actionable findings |

The [validation summary](validation-results.txt) records the complete commands and
results. Database-feature totals include the ordinary backend tests; these counts
are not additive. The four ignored cases are the existing projection benchmark
and three PostgreSQL profiling probes. No test was newly skipped.

Frontend source and schemas are unchanged; the full frontend ladder still ran.
The PostgreSQL run uses PostgreSQL 17.10 Alpine/musl with `en_US.utf8`, preserving native
collation assumed by an existing pairing test. That unrelated test portability
issue remains separately queued. No production data or hosted environment was used.

## Browser acceptance

The [probe](browser-probe.mjs) creates an isolated synthetic match through the
existing authorized application flows, then stops fixture mutations before
observing the read-only checks. The [results](browser-results.json) cover six
view/width combinations and three logout checks:

- Real successful GETs retain exact `private, no-store`; the actual `matchApi`
  runtime decoder accepts both payloads, revision, accepted report and numeric
  note. Long opponent names wrap without horizontal page overflow.
- A controlled response wait exposes loading without a card; injected private
  503 displays an error and **Prøv igjen** retrieves the populated match.
- Coherent empty-card injection retains 18 holes and displays **Ikke startet**.
  Initial private 403 displays the error without a card.
- Actual logout makes both GETs private 401 and the scoring URL redirects to
  login. No match mutations, page errors or unexpected network failures occurred.

Three viewport captures were visually inspected:
[populated scoring at 390 px](scoring-populated-390.png),
[member error at 320 px](member-error-320.png) and
[empty member card at 1280 px](member-empty-1280.png).
These are viewport captures, not assertions about every scrolled control,
physical-device geometry, offline recovery or native browser zoom. The injected
503 verifies the existing UI state; the database attempt-counter test separately
proves real exhausted-conflict HTTP mapping.

## Limits and remaining work

This closes two GETs only. Four P2 routes remain from the historical source
inventory: match completion/table (2), invitation metadata (1) and user-only
final-visibility metadata (1). Three P3 course GETs remain. The historical A1f
manifest and source anchors retain their original revision; those remaining
findings have no new race reproduction in this step. Match listing was already
session-bound and is unchanged. Legacy write expiry remains separately queued.

Browser fixtures are synthetic local data. Failure/empty/denied UI states use
explicit response injection; natural-expiry races are proved by PostgreSQL tests.
No physical-device, load-capacity or public-host readiness claim is made. Full
application security and performance findings remain in the
[original review](../application-review-2026-10-14/README.md).
