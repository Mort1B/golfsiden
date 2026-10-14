# Session-bound Stableford card reads — 2026-10-14

A1i closes the two P2 [Stableford read boundaries](../private-read-inventory-2026-10-14/README.md#stableford):
`GET /api/rounds/{id}/stableford/scorecards/{player_id}` and its `/scoring` variant.

## Original failure and repair

The [original regression](baseline-regression.txt) populates and confirms a
synthetic card, holds `scorecard_confirmations`, starts each GET and observes its
real PostgreSQL wait while the session is still active. It waits for natural
expiry against the database clock, then releases the unchanged relation lock.
Both original GETs return **200 after expiry**. The same regression now returns
**401 unauthenticated** from both, with `private, no-store`, unchanged input/audit/
receipt/confirmation/handicap state and no SSE. Retained evidence contains only
synthetic paths/statuses/error codes, not private card payloads or credentials.

Each successful repeatable-read attempt now calls the central active-session
check after complete build: holes, player inputs, final confirmation query and
domain point/visibility assembly, immediately before commit. Empty cards use the
same path. Retained session/user and exact-membership locks still serialize later
authority writers. This does not freeze expiry through COMMIT, serialization or
network delivery, or retract already delivered data.

The public repository `get` wraps the original complete attempt in private
`get_once`. Only database SQLSTATE `40001` allows one fresh whole read. Stored
context, principal, exact membership, frozen player, scoring authorization, facts
and derived values are reloaded. Authorization database errors retain their
existing flattening. A second conflict becomes `ReadUnavailable`, mapped to
generic `503 service_unavailable` only by the two GET handlers. Other errors
retain their old mapping. Existing all-status `private, no-store` middleware and
mutation error mapping are unchanged.

Original order stays context, active session, exact membership, format, frozen
player snapshot, then scoring editability/permission. A correctly formatted draft
without snapshots still returns 404 before editability checks. No settings,
conditional delivery, confirmation or shared mutation helper changes; no mutation
retry or new SSE behavior. Fact SQL, ordering, payloads, formulas, frozen handicap
policy, 18-hole restriction and hidden front-nine projection remain unchanged.
No schema, dependency, frontend source or hosting configuration changes. Each
ordinary success adds one final session query; no measured speedup is claimed.

## Regression coverage

Seventeen new PostgreSQL/API cases reuse the existing Stableford fixture unchanged:

- Revocation/credential replacement after extraction but before transactional
  authority, later writers serialized behind authorized reads, and fresh denial
  after membership removal.
- Natural expiry during membership, late input and final confirmation waits for
  both GETs, including empty and populated/confirmed cards. Eight-second setup
  windows verify actual blocking with `pg_blocking_pids` and an active session
  before waiting for unchanged expiry. No production timing hooks are used.
- Fresh context and role after actual released-to-hidden final visibility plus
  admin-to-viewer demotion; member retry redacts and scoring retry denies.
- Fresh owner names after concurrent membership snapshot invalidation, with
  frozen handicap unchanged despite current global handicap edits.
- Persistent synthetic `40001` and unrelated `XX000`: nontransactional sequence
  counters prove two versus one attempts, private generic 503 versus 500, no
  payload/database-detail escape, unchanged mutation data and no SSE.
- Twelve concurrent full JSON comparisons across three players and both
  projections. Independent expectations distinguish 34 native points, +2 overall
  equivalent and 88 actual strokes; a retained 20-stroke input earns zero points.
  A pickup has zero points and absent strokes; an unentered hole remains null.
  Partial cards omit full stroke totals; signed -10 and +40 handicaps remain
  frozen after current handicap changes. Member entries stay actor/revision-free.
- Exact four-role permissions, own/same-flight/other-flight scoring boundaries,
  global-admin non-bypass, missing/invalid resources, format/state precedence,
  nine-hole rejection, completed/locked behavior and adjacent mutation/cache
  compatibility.
- Hidden back-nine numeric/pickup and confirmation changes leave the complete
  permitted member JSON unchanged, including nullable protected metadata.
- Late confirmation loading retains coherent old input/confirmation facts while
  a deliberate concurrent correction changes both; a fresh read sees the change.
  Cancellation releases retained authority and subsequent reads succeed.

Snapshots compare Stableford inputs, audits, immutable receipts, confirmations,
round handicaps and legacy score/audit tables; subscriptions assert no read SSE.
Independent read-only design and final source/test review found no actionable
findings, including error order, point/stroke separation and mutation boundaries.

## Validation

| Check | Result |
| --- | --- |
| Original final-confirmation regression | Expected failure: both GETs 200 after expiry |
| Same regression after repair | Passed: both private 401 |
| Final focused suites | 36 passed: 17 new and 19 existing Stableford tests |
| Full backend all-target tests | 261 passed; 1 existing benchmark ignored |
| Formatting / strict all-feature/target Clippy / binary build | Passed; 3 unchanged vendored SQLx warnings |
| Full PostgreSQL-feature ladder | 865 passed; 4 existing performance probes ignored |
| Explicit migration / seed | Passed, schema 39; separate browser fixture also passed |
| Local health / readiness | Both 200; PostgreSQL 17.10, schema 39, no failed migrations |
| Chrome at 320/390/1280px | All six endpoint/width flows passed |
| Independent read-only review | No actionable findings |

The first expanded run passed 16/17 new cases and all 19 existing cases. One new
fixture misspelled `individual_stableford` as `stableford_stroke_play`; PostgreSQL
correctly rejected the invalid enum. The correction also ensured all four
required player cards were confirmed before lifecycle completion. No runtime
change was needed for those test fixture corrections; the final focused run passes.

Disposable owned container `golf-a1i-20261014-pg` uses the repository's
`postgres:17-alpine` image, loopback port 55449 and tmpfs storage. SQLx owns isolated
test databases; separate `golf_a1i_browser` is migrated/seeded and holds synthetic
data only. The known native-collation test portability issue stays queued and
unchanged; the ladder uses the repository's Alpine fixture. No production accounts
or operator infrastructure are used. Owned API/Vite processes were stopped and
the disposable PostgreSQL container removed after validation.

Commands from repository root, with a disposable `DATABASE_URL` where applicable:

```bash
cargo test -p golf-api --features database-tests --test stableford_read_sessions --test stableford -- --test-threads=2
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build -p golf-api --bins
cargo test --workspace --all-targets --features database-tests -- --test-threads=2
cargo run -p golf-api --bin migrate
cargo run -p golf-api --bin seed
node docs/validation/stableford-read-session-2026-10-14/browser-probe.mjs
```

Logs are `/tmp/golf-a1i-{baseline,repaired,focused,focused-final,fmt,workspace,clippy,build,postgres,migrate,seed,browser}.log`.
The database-feature total includes ordinary tests and is not additive with 261.
Four existing ignored cases are one projection benchmark and three PostgreSQL
profiling probes.
Frontend code/JSON contracts are unchanged, so its unit/type/lint/build ladder is
unaffected and was not repeated. Physical devices, public hosting and production
load remain outside this step's acceptance.

## Browser evidence

The [probe](browser-probe.mjs) creates a fresh synthetic one-player Stableford
fixture through onboarding, manual 18-hole course configuration, flight assignment,
tournament start and round opening. It submits numeric 20, pickup and numeric 4
on three holes before read observers. Never run it on retained/production data.

At 320, 390 and 1280px, real `stablefordApi.read/scoring` decoders, private 200
responses and existing direct-member/scoring pages pass. Typed results retain
20 actual strokes with zero points, distinct native points/equivalents, a pickup,
an unentered hole and absent partial stroke totals. Both UI flows cover delayed
loading, long populated content without horizontal overflow, injected 503 and
explicit read-only retry, injected empty 18-hole/0-progress cards, and injected
403 without retaining initially denied cards. After logout both real GETs return
private 401 and the scorer redirects to login.

The [results](browser-results.json) record six flows plus three per-width logout,
no score writes, no page exceptions and no unexpected network failures. UI empty/
error/denied responses are deliberately injected; PostgreSQL/API cases cover real
authority races and natural empty cards. The unchanged scorer revalidates reads
after synthetic scoring 403 while injected writable access still permits the
player; response counts include these read refreshes. The flow unmounts before
logout to isolate phases. No separate production defect is established by that
synthetic inconsistency.

Visually inspected viewport screenshots:

- [320px member card with long name and point/equivalent values](member-populated-320.png)
- [390px empty scoring card](scoring-empty-390.png)
- [1280px scoring error with read retry](scoring-error-1280.png)

Other captures are `/tmp/golf-a1i-{member,scoring}-{loading,populated,error,empty,denied}-{320,390,1280}.png`.
They cover local Chrome, not physical-device behavior or every scroll position.

## Remaining scope

Six P2 and three P3 routes remain in the original private-read inventory: match
cards/completion/table, invitation/final-visibility metadata and course reads.
Legacy-write inventory, dependencies, restore checksums, password-work admission
and measured team batching retain separate bounded steps in [PLANS.md](../../PLANS.md).
No queued implementation began.
