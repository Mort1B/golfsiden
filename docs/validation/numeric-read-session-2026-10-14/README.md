# Session-bound numeric score reads — 2026-10-14

A1g closes three P2 boundaries from the [private-read inventory](../private-read-inventory-2026-10-14/README.md#numeric):

- `GET /api/rounds/{id}/score-access`
- `GET /api/rounds/{id}/scorecards/{owner_type}/{owner_id}`
- The same card GET with `/scoring` appended.

## Original failure and repair

The [regression against the original implementation](baseline-regression.txt)
holds exact membership, starts each GET, observes its real PostgreSQL wait while
the session remains active, lets the unchanged session naturally expire against
database wall-clock time, then releases the row without changing membership.
All three returned **200 after expiry**. The same regression now returns
**401 unauthenticated**, with non-storeable responses and no score event.
Retained failure evidence contains only synthetic paths/statuses/error codes.

Existing session/user and exact-membership locks already serialize revocation
and credential replacement. The repair adds a final central active-session check
after complete successful assembly, immediately before commit. Member cards check
after confirmation loading and read projection; scoring cards check after the
full summary. Access checks after owner resolution and four-ball partner-to-side
mapping, including empty viewer/unlinked-player and singles paths. Natural expiry
can deny success after these waits. This does not freeze time through COMMIT,
serialization or network delivery, or retract delivered data.

All three retain repeatable read. The existing complete scoring-card `40001`
retry is shared privately with the member-card read. Access has its own complete
attempt and read-specific error wrapper; the common authorization error and
mutation helpers are unchanged. Each discards the entire failed transaction and
allows one fresh attempt with fresh session, role, context and facts. A second
serialization conflict yields generic `503 service_unavailable` without data;
other errors retain their mapping and are not retried.

Selected GET middleware fills missing `Cache-Control` with `private, no-store`.
Existing `no-store` denials stay unchanged. This covers extraction, validation,
missing resources and database errors as well as success. The pre-existing
missing-card 404 test now expects this deliberate header change; its status,
error-code and cross-target authorization assertions are retained. Adjacent save,
confirmation and conditional-delivery routes retain their cache contracts.

Fact SQL, error ordering, format validation, owner identity, deterministic order,
flight permissions, fixed handicaps, hidden-final redaction and JSON shapes are
unchanged. Unrestricted card helpers and score mutations are unchanged. Dedicated
four-ball, Stableford and match card routes remain outside this step, although
the selected shared access GET still maps four-ball partner authority to teams.
No schema, dependency, frontend source or hosting configuration changes. An
ordinary success adds one final session query; no batching or speedup is claimed.

## Regression coverage

Eighteen new PostgreSQL/API tests cover:

- Revocation/credential replacement before transactional authority, and later
  authority writers serialized behind valid readers.
- Membership removal, exact four-role behavior, global-admin non-bypass, linked
  player flight permissions and member-versus-scoring access distinctions.
- Expiry during membership waits on all three routes, final confirmation loading
  on both cards, late owner loading, empty access and final four-ball side mapping.
- Fresh retries after admin-to-viewer demotion with an actual released-to-hidden
  final context change; fresh owner ordering after a concurrent name change.
- Synthetic failures with nontransactional attempt counts: `40001` makes exactly
  two attempts and yields private 503; `XX000` makes one and yields generic 500.
  No discarded payload, database detail, score write or SSE escapes.
- Twelve concurrent reads with full JSON parity against existing internal
  projections, unchanged score/audit/confirmation/handicap state and no SSE;
  cancellation releases authority and connections remain usable.
- Late confirmation loading preserves one coherent old score/confirmation
  snapshot; a fresh read sees changed scores and removed confirmation.
- Nine-hole foursomes with preserved team handicaps after current handicaps
  change, scramble/individual cards, singles format/error precedence, invalid
  owners/resources and adjacent mutation error/cache compatibility.

Expiry tests provide eight seconds to reach the gate, assert actual blocking
with `pg_blocking_pids` and verify the session remains active before waiting for
natural expiry. They do not modify expiry after the read starts. Card loading
gates `scorecard_confirmations`, the final fact query after scores/handicaps/holes;
access gates `team_memberships`. Four-ball individual owner resolution does not
read that table, so its gate occurs in final side mapping. No production timing
hooks or runtime fault injection are added.

Independent read-only review found no actionable source/test findings. It also
confirmed the existing 404 cache assertion adjustment preserves the approved
contract and leaves adjacent 403 `no-store` responses intact.

## Validation

| Check | Result |
| --- | --- |
| Original expiry regression | Expected failure: all three GETs 200 after expiry |
| Repaired original regression | Passed: all three non-storeable 401 |
| Corrected focused suites | 58 passed: 18 new, 3 authority, 37 card tests |
| Full backend all-target tests | 261 passed; 1 existing manual benchmark ignored |
| Formatting and strict all-feature/target Clippy | Passed; 3 existing vendored SQLx warnings |
| API binary build | Passed |
| Full PostgreSQL-feature ladder | 831 passed; 4 existing manual performance probes ignored |
| Explicit migration and development seed | Passed, schema 39; separate browser fixture also passed |
| Health/readiness and schema | Both 200; schema 39, no failed migrations |
| Chrome at 320/390/1280px | All nine endpoint/width flows passed |
| Independent read-only review | No actionable findings |

The first expanded focused run passed 16/18. Its failures were fixture-only:
an adjacent conditional route already returned `private, no-store` rather than
literal `no-store`, and the synthetic nine-hole tee omitted its required category.
Those expectations/data were corrected without runtime changes. An earlier
compile failure came from an async test assertion borrowing a non-Sync response
in a spawned task; the assertion is now synchronous. The first full database run
then stopped on the existing missing-card 404 cache assertion described above.
The complete final run includes that compatibility update.

Validation uses owned disposable container `golf-a1g-20261014-pg`, the repository's
`postgres:17-alpine` image (PostgreSQL 17.10), loopback port 55449 and tmpfs data.
SQLx owns isolated test databases; `golf_a1g_browser` is independently migrated and seeded with
synthetic accounts. No production data or infrastructure is used. The known
Debian/Alpine native-collation test portability issue stays queued and unchanged;
the final ladder uses the repository's Alpine fixture. Owned API/Vite processes
were stopped and the disposable PostgreSQL container removed after validation.

Commands from repository root, with disposable `DATABASE_URL` for database work:

```bash
cargo test -p golf-api --features database-tests --test numeric_read_sessions --test scorecards --test score_authorization -- --test-threads=2
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build -p golf-api --bins
cargo test --workspace --all-targets --features database-tests -- --test-threads=2
cargo run -p golf-api --bin migrate
cargo run -p golf-api --bin seed
node docs/validation/numeric-read-session-2026-10-14/browser-probe.mjs
```

Logs: `/tmp/golf-a1g-{baseline,repaired,focused,focused-corrected,workspace,clippy,build,postgres,postgres-final,migrate,seed}.log`;
final formatting/Clippy logs use `{fmt-final,clippy-final}`. The PostgreSQL total
includes ordinary tests and is not additive with 261. The four ignored cases are
one projection benchmark and three existing PostgreSQL profiling probes. Frontend code and runtime
JSON contracts are unchanged, so its unit/type/lint/build ladder is unaffected
and was not repeated. No production-load or physical-device claim is made.
Deferred test maintenance and operator-owned deployment acceptance stay separate.

## Browser evidence

The [probe](browser-probe.mjs) uses the existing app, current local API build and
Vite. It starts the synthetic trip, renames the exact team with long text, opens
the first round and saves one hole through existing mutation APIs before attaching
read observers. Do not run it against retained/production data.

At each width it exercises all three real typed client methods and private 200
responses, then the scoring workspace for access/scoring and the direct member
card page. Every flow passes delayed loading; populated 1/18 holes; long names
without horizontal overflow; injected 503 with explicit read-only retry; injected
empty and 403 responses, with no card retained on initial denial. Empty access
uses the existing read-only fallback; empty card injections preserve 18 configured
holes with no scores and 0/18 progress. After logout all three real GETs return
non-storeable 401 and the scorer redirects to login.

The [results](browser-results.json) record nine flows and per-width logout,
no score writes, no page exceptions and no unexpected network failures. Error,
denied and empty UI responses are deliberately injected; real expiry, permission
races and natural empty branches are covered by PostgreSQL/API tests. The existing
scorer revalidates read queries after injected scoring 403; injected-response
counts include those read retries. No score mutation is emitted.

The first probe completed the 320px flows but counted expected post-logout 401s
from the still-mounted denied scorer as unexpected failures. The corrected probe
unmounts that flow before logout; all nine flows then pass. This changes only
probe phase isolation. Runtime frontend code is unchanged.

Visually inspected screenshots:

- [320px member card with long name](member-populated-320.png)
- [390px empty writable list using read-only fallback](access-empty-390.png)
- [320px card with no entered scores](scoring-empty-320.png)
- [1280px scoring error and read retry](scoring-error-1280.png)

Full-page captures include existing fixed navigation at the capture viewport;
they do not simulate physical-device scrolling. Remaining captures are under
`/tmp/golf-a1g-{access,member,scoring}-{loading,populated,error,empty,denied}-{320,390,1280}.png`.

## Remaining scope

This closes only the three selected A1g GET boundaries. Ten P2 and three P3 routes
from the original inventory remain. Dedicated format-card repairs, legacy write
expiry inventory, dependencies, restore checksums, password-work admission and
measured team batching retain their severity and separate bounded steps in
[PLANS.md](../../PLANS.md). No queued implementation began.
