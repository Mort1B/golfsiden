# A1r — course-configuration session expiry

Follow-up: [A1s mobile-label repair](../manual-hole-labels-2026-10-15/README.md)
closes the separate P3 presentation issue. The P3 section, source anchors and
screenshots below preserve A1r's original CSS snapshot; they are historical
evidence, not the repaired layout.

Base revision: `69482bc566ac20f799dbd47162923c42c32172fe`.
Scope: `PUT /api/rounds/{round_id}/course-configuration`, its final transaction,
PostgreSQL/API regressions, existing-editor browser acceptance and affected docs.
This repairs W3 in the [legacy-write inventory](../legacy-write-inventory-2026-10-15/README.md#w3--course-configuration-incomplete-p2).
Preflight redesign, provider/cache/quota work, frontend implementation, sporting
rules, retained helpers, schema/dependencies and hosted work remain separate.

## Result and authority boundary

The [configuration transaction](../../../backend/src/repositories/round_configuration.rs#L77)
keeps its stored-round lock, actual session/user and exact-admin membership
share locks. After inserting the complete course/tee/hole revision and awaiting
the round UPDATE/RETURNING and immediate triggers, it calls
[auth::lock_active_session](../../../backend/src/repositories/round_configuration.rs#L124)
for the same session UUID immediately before commit. Its database
`clock_timestamp()` catches natural expiry during preceding waits. The retained
authority locks already serialize revocation and credential/membership writers;
the check adds no new lock order or mutation retry.

Expiry uses the existing private/no-store `401 unauthenticated` envelope. It
rolls back all new hierarchy rows, the attachment and timestamp, returns no
updated round, and emits no SSE. Valid saves retain the same updated-round model
and one matching event after commit. CSRF, user-only preflight, external provider
fetch placement, draft/version/format checks, validation order, immutable
revision facts, error precedence and cache behavior are unchanged. Time inside
COMMIT, deferred constraints and network delivery remains outside the final check.

## Reproduction and regression coverage

Both [new API tests](../../../backend/tests/round_configuration/expiry.rs#L1)
ran against unchanged production source at the base revision. Each observed the
selected final-transaction wait while its session remained active, waited for
natural database-clock expiry, then released the gate. Both returned **200
instead of 401**: [original failures](baseline-regression.txt).

The handler buffers the request body before preflight. Preflight reads the round
without a row lock and holds membership authority only until its own commit.
The tests first hold an unchanged round `FOR UPDATE` and observe the final
`require_round_admin` query blocked on that exact backend PID. This proves
preflight finished before they acquire the selected gate and set session expiry.
Releasing the round lets final authorization acquire the session before the
second observed wait.

- Membership case: hold the unchanged exact-admin membership `FOR UPDATE`, then
  observe the final membership `FOR SHARE` query waiting on that blocker. No
  role, round version or other domain fact changes to construct the wait.
- Late case: a [test-only trigger](../../../backend/tests/round_configuration/late_gate.sql#L1)
  runs AFTER UPDATE OF course_id on only the selected fixture round. It verifies
  changed attachment IDs and timestamp, draft state, exact copied names/count,
  finalized manual provenance/import time, one complete tee and the exact two
  ordered hole/par/stroke-index/distance facts, then waits on an advisory lock.
  The request's UPDATE is observed blocked on that lock. Assertions run inside
  the transaction where pending rows are visible. Deferred completeness checks
  have not executed at this point and are not forced early by the test.
- Both cases require the exact private 401 envelope, complete equality of
  [stored course/tee/hole and round/history facts](../../../backend/tests/round_configuration/state.sql#L1),
  and no event while blocked or after rejection. Snapshotting every hierarchy
  row catches orphan courses, tees or holes, including rows without finalized
  provenance. Existing target attachments, presets, the other opened round,
  entrant/handicap snapshots and roster/scoring facts remain unchanged. Sessions
  are excluded because the test changes expiry.
- A fresh valid session traverses each gate using the original optimistic token
  and command. Each control commits exactly one new course, one tee and two
  holes, changes only the intended attachment fields/timestamp, returns private
  200 and publishes one matching event. Original hierarchies and all unrelated
  facts are preserved. The two-hole manual fixture exercises the ordinary stroke
  play path; retained tests cover restricted 18-hole formats and provider-tagged
  validated commands. Neither uses remote provider HTTP success.

The [focused run](focused-results.txt) passes **36 cases**: 18 configuration,
10 revision and eight course-read authorization cases. The two expiry cases
include their valid controls. SQLx isolates each database; fixture triggers
change no published migration or production schema.

## Browser acceptance and its limits

The [real Chrome probe](browser-probe.mjs) runs the existing course editor at
`/manage/tournaments/{id}?round={id}#courses`, at **320, 390 and 1280px**.
[Recorded results](browser-results.json) cover all widths with no unexpected
page/console/network errors, on a separately migrated/seeded synthetic database.

Each width covers delayed saved-course loading, live populated presets and a
table with the returned hole-row count; it does not compare every displayed
cell. Deliberately injected valid long preset names, private GET 503/retry and
typed empty arrays cover those states. A real manual save submits an 18-hole
layout, temporarily held to inspect disabled fields/radios and saving feedback.
Exactly one real PUT returns private 200, collapses the editor and focuses Endre.
Round GET and reload verify persisted course/tee names and hole count. Long names
wrap in the [desktop summary](desktop-summary.png), with no horizontal page
overflow. Backend tests above own exact persisted hierarchy facts.

The PUT 401 is **injected** while real GET authority remains valid. The editor
shows [its existing access feedback](mobile-denial.png), preserves the input
inside the same mounted manual form and re-enables fields/save after pending
ends. No automatic PUT retry occurs and the authoritative round DTO is unchanged.
This editor does not automatically invalidate reads, clear auth or provide a
pairing-style discard control on access errors. Changing mode, reload or actual
auth loss can unmount the local form; no retention across those transitions is
claimed. Actual logout plus forced reload separately verifies private round GET
401 and login redirection with no course editor. Late database expiry, complete
rollback/orphan checks and no-event proof belong to PostgreSQL/API tests.

The real bundled catalog remains unusable: fallback instructions and disabled
provider save are checked, with zero provider-detail requests. No usable IDs are
manufactured and no remote provider success or quota/cache behavior is claimed.
Initial probe runs used an exact label matcher that did not select the loaded
saved-course combobox. Switching to the established role/name selector fixed
the probe; application source was unchanged. The complete corrected run passes.

## Separate P3: mobile manual-hole labels

**P3 low, pre-existing usability defect.** Below 700px, sighted manual-entry users
see three numeric fields per hole without persistent Par, Slagindeks or Avstand
labels. Accessible names remain; no scoring or backend corruption was observed.
The [mobile screenshot](mobile-hole-labels.png) and
[desktop comparison](desktop-hole-labels.png) confirm the difference.
[CSS hides the column heading](../../../frontend/src/features/tournaments/course-configuration.css#L43)
and [visually hides row labels](../../../frontend/src/features/tournaments/course-configuration.css#L46),
then [restores column headings only at 700px](../../../frontend/src/features/tournaments/course-configuration.css#L61).
The smallest follow-up keeps compact visible column/row labels on mobile while
preserving accessible names, values, validation and save behavior. This step
records the finding and a separate candidate; it changes no frontend source.

## Validation

| Check | Result |
| --- | --- |
| New tests against original production | Both failed as expected: 200 instead of 401 |
| Focused configuration/revision/course-read suites | 36 passed |
| Rust formatting | Passed |
| Full backend all-target tests | 264 passed, zero failures; one existing manual probe ignored |
| Strict Clippy, all targets/features | Passed; three existing vendored SQLx warnings |
| API binaries and local health/readiness | Passed |
| Chrome editor, 320/390/1280px | Passed for the repair; separate P3 labels issue recorded above |
| Independent implementation/test review | No actionable findings |
| Independent browser-evidence review | No A1r findings; confirmed separate P3 labels issue |
| Full PostgreSQL-feature ladder, two threads | Complete rerun: 952 passed, zero failures; five existing probes ignored |
| Explicit migrate/seed ladder | Passed; all 39 migrations successful, eight players and five rounds |
| Independent documentation/next-plan review | No actionable findings |
| Final static/closure checks | Local links/source anchors, file scope/size, diff/EOF whitespace and fixture cleanup passed |

The [retained command results](validation-results.txt) include all 92 completed
targets, the five exact ignored probes and the incomplete first attempt. Both
new course-expiry regressions also pass in the complete ladder.

The disposable rootless Podman fixture was PostgreSQL **17.10**, image
`postgres:17-alpine`, container
`golf-a1r-20261015-pg`, on loopback port 55462. SQLx owned isolated databases;
browser data lived in separate `golf_a1r_browser`. Browser contexts and the owned
API/Vite services were closed before the full database ladder; ports were
confirmed released. After migration/seed acceptance, the owned container was
stopped and confirmed removed, and both private environment files were deleted.
The full PostgreSQL count includes ordinary backend tests and is not additive.

The first full-suite attempt ended with exit 143 after reporting 270 passing
unit cases and one existing ignored probe. No failing case was reported; the
interruption cause is unconfirmed. That attempt is not acceptance evidence.
The complete replacement run exited 0 with all 952 cases passing.

From the repository root with an approved disposable `DATABASE_URL` set privately:

```bash
cargo test --locked -p golf-api --features database-tests --test round_configuration expiry:: -- --test-threads=2
cargo test --locked -p golf-api --features database-tests --test round_configuration --test course_revisions --test course_read_sessions -- --test-threads=2
cargo fmt --all -- --check
cargo test --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo build --locked -p golf-api --bins
cargo test --locked --workspace --all-targets --features database-tests -- --test-threads=2
cargo run --locked -p golf-api --bin migrate
cargo run --locked -p golf-api --bin seed
node docs/validation/course-write-session-2026-10-15/browser-probe.mjs
```

Browser reproduction requires separately seeded data, the repaired API on 3000
in development mode with HTTP cookies, existing Vite on 5173, and
`GOLF_A1R_DEV_PASSWORD` set privately. No credentials, session cookies or database
URLs are retained here. Raw logs and additional captures remain under
`/tmp/golf-a1r-*`.

Frontend unit/type/lint/build ladders were not rerun because frontend source,
response shape, decoding and UI implementation are unchanged. Real-browser
acceptance covers the current user-facing error path and records the separate
existing mobile limit. No remote-provider acceptance, hosted check, dependency
scan or performance benchmark ran. This acceptance does not establish hosted
readiness.
