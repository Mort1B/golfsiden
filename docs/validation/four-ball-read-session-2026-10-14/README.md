# Session-bound four-ball card reads — 2026-10-14

A1h closes two P2 routes from the [private-read inventory](../private-read-inventory-2026-10-14/README.md#four-ball):
`GET /api/rounds/{id}/four-ball/scorecards/{team_id}` and its `/scoring` variant.

## Original failure and repair

The [original final-loading regression](baseline-regression.txt) populates and
confirms a synthetic card, holds `scorecard_confirmations`, starts each GET,
observes its real PostgreSQL wait while the session remains active, then waits
for natural database-clock expiry before releasing the unchanged relation lock.
Both original GETs return **200 after expiry**. The same regression now returns
**401 unauthenticated** for both, with `private, no-store`, unchanged score state
and no SSE. Evidence retains synthetic paths/statuses/error codes, not payloads.

The [earlier membership-gate baseline](baseline-membership.txt) differs: member
GET returned 200 after expiry, while scoring already returned 401 because its
later both-partner authorization rechecked the session. That existing check did
not cover subsequent input/confirmation waits. The repair preserves it and adds
a final central active-session check after full build, including the last
confirmation query and domain visibility projection, immediately before commit.
Empty cards follow the same path. This does not freeze time through COMMIT,
serialization or delivery, or retract data already delivered.

The public repository `get` wraps its unchanged repeatable-read attempt in
`get_once`. Only database SQLSTATE `40001` permits one complete fresh attempt;
context, principal, exact membership, partners, scoring authorization, inputs,
confirmation and derived projection are rebuilt. Nested authorization database
errors retain their existing flattening. A second conflict becomes
`ReadUnavailable`, mapped to generic `503 service_unavailable` only by the two GET
handlers. All other read errors delegate to the existing mapper. Existing
all-status `private, no-store` middleware is unchanged.

Original lookup/error order stays context, active session, exact membership,
format, stored side/two frozen partners, then scoring editability/permissions.
Member reads still require membership; scoring additionally requires both player
cards to be writable. Mutation bodies, shared authorization/build/context helpers,
conditional delivery, confirmation and their error mapping remain unchanged.
No mutation gains retry or SSE behavior. Fact SQL, payloads, ordering, input and
winner rules, frozen handicaps, nine-hole rejection, hidden front-nine projection,
frontend source, schema, dependencies and hosting configuration are unchanged.
An ordinary success adds one final session query; no speedup is claimed.

## Regression coverage

Seventeen new PostgreSQL/API cases reuse the existing four-ball fixture without
changing it and cover:

- Revocation/credential replacement before transactional authority, later writers
  serialized behind valid reads, and fresh denial after membership removal.
- Real expiry at membership, input and final confirmation waits on both GETs,
  with empty and populated/confirmed cards. Eight-second setup windows verify
  active sessions and actual `pg_blocking_pids` before waiting for unchanged
  natural expiry; there are no production timing hooks.
- An actual released-to-hidden final context change with admin-to-viewer demotion:
  member retry returns front-nine/null protected metadata; scoring retry denies.
- Concurrent partner-name changes with membership snapshot invalidation: fresh
  reads rebuild both partner ordering and names and match a subsequent full GET.
- Persistent synthetic `40001` versus `XX000`: nontransactional sequence counters
  prove exactly two versus one attempts, generic 503 versus 500, no private card
  or database detail, unchanged inputs/audits/receipts/confirmations and no SSE.
- Twelve concurrent full JSON comparisons across both sides/projections, actor-
  free member entries, scoring revisions, tied gross winners, a separate net
  winner, explicit pickup/unentered inputs, signed frozen handicaps unchanged
  after current global handicap edits, and no read writes/SSE.
- Exact four-role, flight and global-admin non-bypass behavior; missing/invalid
  resources; format/state error precedence; retained nine-hole rejection and
  completed/locked card behavior; adjacent mutation/cache compatibility.
- True one-partner authority: a schema-only view hides the actor's flight link
  while preserving stored frozen rows. Direct authorization proves their own
  player card succeeds and the other fails; member GET remains 200 while side
  scoring GET is 403. This is fail-closed read perturbation, not a supported
  production pairing configuration or a change to pairing guards.
- Late confirmation loading retains one coherent old input/confirmation snapshot
  while a deliberate concurrent correction changes both; a fresh GET sees the
  change. Cancellation releases retained session locks and connections are reused.

Independent read-only design and source/test review found no actionable findings.
It specifically checked final query placement, whole-attempt retry, both-partner
proof, actual visibility change, preserved error order and mutation boundaries.

## Validation

| Check | Result |
| --- | --- |
| Original final-confirmation regression | Expected failure: both GETs 200 after expiry |
| Same regression after repair | Passed: both private 401 |
| Corrected focused suites | 34 passed: 17 new and 17 existing four-ball tests |
| Backend all-target tests | 261 passed; 1 existing benchmark ignored |
| Formatting / strict all-feature/target Clippy / binary build | Passed; 3 unchanged vendored SQLx warnings |
| Full PostgreSQL-feature ladder | 848 passed; 4 existing performance probes ignored |
| Explicit migration / seed | Passed, schema 39; separate browser fixture also passed |
| Local health / readiness | Both 200; PostgreSQL 17.10, schema 39, no failed migrations |
| Chrome at 320/390/1280px | Six endpoint/width flows passed |
| Independent read-only review | No actionable findings |

The first expanded run passed 15/16 new tests. Its sole failure was an incorrect
new expectation that a draft four-ball card was readable before frozen handicap
snapshots existed. The corrected test asserts existing `score_owner_not_eligible`
409 for both draft GETs, preserving partners-before-editability precedence.
Another case then added fresh partner-order/name coverage, making 17. A test-only
unused import was removed. Runtime source did not change for those corrections.

Owned disposable container `golf-a1h-20261014-pg` uses the repository's
`postgres:17-alpine` image, loopback port 55449 and tmpfs storage. SQLx tests own
isolated databases; `golf_a1h_browser` is independently migrated/seeded and holds
only synthetic data. The known native-collation portability case stays queued;
the ladder uses the repository's Alpine fixture. No production data or operator
infrastructure is used. Owned API/Vite processes were stopped and the disposable
PostgreSQL container removed after validation.

Commands from repository root, with disposable `DATABASE_URL` where applicable:

```bash
cargo test -p golf-api --features database-tests --test four_ball_read_sessions --test four_ball -- --test-threads=2
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build -p golf-api --bins
cargo test --workspace --all-targets --features database-tests -- --test-threads=2
cargo run -p golf-api --bin migrate
cargo run -p golf-api --bin seed
node docs/validation/four-ball-read-session-2026-10-14/browser-probe.mjs
```

Logs: `/tmp/golf-a1h-{baseline,baseline-membership,repaired,focused,focused-final,fmt,workspace,clippy,build,postgres,migrate,seed,browser}.log`.
The database-feature total includes ordinary tests and is not additive with 261.
The four ignored cases are one projection benchmark and three PostgreSQL profiling
probes, all pre-existing.
Frontend source and JSON contracts are unchanged; its unit/type/lint/build ladder
is unaffected and was not repeated. Public hosting, physical devices and
production load remain outside this step's acceptance.

## Browser evidence

The [probe](browser-probe.mjs) creates a fresh synthetic two-player tournament
through onboarding/invitation APIs, saves a manual 18-hole tee, assigns one side
and flight with long names, starts/opens it, then submits two tied numeric inputs
on hole one and a pickup with the other partner unentered on hole two. Fixture
writes precede read observers. Never run it against retained or production data.

At 320, 390 and 1280px, real `fourBallApi.read/scoring` decoders, both private 200
HTTP replies and the direct member/scoring pages pass. Each flow covers delayed
loading, populated long content with no horizontal overflow, injected 503 with
explicit read-only retry, empty 18-hole cards with 0/18 progress and injected 403
with no initially denied card retained. After logout both real GETs return private
401 and the scorer redirects to login. No score writes, page exceptions or
unexpected network failures occur; see [results](browser-results.json).

Empty/error/denied browser responses are deliberately injected; PostgreSQL/API
cases cover real expiry, permissions and natural empty cards. The unchanged
scorer revalidates reads after injected scoring 403 while synthetic writable
access still permits the side; the response counts include these read refreshes.
The denied flow unmounts before logout to isolate phases. This does not establish
a separate production defect or change that frontend behavior.

Visually inspected viewport screenshots:

- [320px member card: long side/partner names and tied winners](member-populated-320.png)
- [390px empty scoring card](scoring-empty-390.png)
- [1280px scoring error with read retry](scoring-error-1280.png)

Other viewport captures are `/tmp/golf-a1h-{member,scoring}-{loading,populated,error,empty,denied}-{320,390,1280}.png`.
These cover local Chrome, not physical-device behavior or every scroll position.

## Remaining scope

Eight P2 and three P3 routes from the historical private-read inventory remain.
Stableford cards, match detail/completion/table, invitation/final-visibility and
course reads, legacy write inventory, dependencies, recovery, admission and
measured team batching retain separate bounded steps in [PLANS.md](../../PLANS.md).
No queued implementation began.
