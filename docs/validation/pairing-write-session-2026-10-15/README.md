# A1q — pairing-replacement session expiry

Base revision: `cc611b6c30c91f180d5ab4c1b1450322454ee41a`.
Scope: `PUT /api/rounds/{round_id}/pairings`, its replacement transaction,
PostgreSQL/API regressions, browser acceptance and affected documentation.
This repairs W2 in the [legacy-write inventory](../legacy-write-inventory-2026-10-15/README.md#w2--aggregate-pairing-replacement-incomplete-p2).
Course configuration (W3), provisional preflights, retained helpers, frontend
implementation, schema/dependencies and hosted work remain separate.

## Result and authority boundary

The [replacement transaction](../../../backend/src/repositories/round_pairings/replace/mod.rs#L17)
retains its stored-round lock, actual session/user and exact-admin membership
share locks. After all validation, roster mutations, the timestamp update and
complete `load::model` response assembly, it calls
[auth::lock_active_session](../../../backend/src/repositories/round_pairings/replace/mod.rs#L48)
for the same session UUID immediately before commit. Database
`clock_timestamp()` catches natural expiry during preceding waits. The query
reuses already held authority locks and introduces no new lock order or retry.

Expiry returns the existing `401 unauthenticated` with `private, no-store`.
The transaction rolls back groups, memberships and round timestamps, returning
no layout or mutation acknowledgment and publishing no event. Valid requests
retain their 200 model and one matching round event after commit. Existing CSRF,
provisional preflight, draft/version checks, validation precedence, legacy
identity/schedule/order/timestamp preservation and durable references are
unchanged. No provider work, authority cache or mutation retry is added. Time
inside COMMIT, deferred constraints and network delivery remains outside the
final-check guarantee.

## Reproduction and regression coverage

Both [new API tests](../../../backend/tests/round_pairings/expiry.rs#L1) ran
against unchanged production source at the base revision. Each observed its
targeted final-transaction wait, verified the session was still active there,
then waited for database-clock expiry and released the gate. Both returned
**200 instead of 401**: [original failures](baseline-regression.txt).

The handler buffers the request body before preflight, which reads the round
without a row lock. The tests therefore first hold an unchanged round `FOR UPDATE` and observe
the final `require_round_admin` query waiting there. This proves preflight has
finished while the final session query has not run. Only then do they acquire
the selected work gate and set expiry; releasing the round admits the final
session check before the second observed wait.

- Membership case: an unchanged exact-admin membership is held `FOR UPDATE` in
  another transaction. The test observes the final membership query blocked on
  that exact blocker PID, then allows natural expiry. Neither round nor role is
  changed merely to construct the wait, so no stale-token/role rejection is used.
- Late-write case: a test-only `AFTER UPDATE OF updated_at` trigger verifies the
  changed timestamp, deleted legacy team/members and old flight, and the two new
  flights/three members with the preserved legacy schedule before taking an
  advisory lock. These assertions run inside the transaction; other connections
  cannot observe pending rows. This is a late-write gate, not a separately
  reproduced final-loader wait. The source's final check follows the subsequent
  complete response loading as well.
- Both rejected requests require the exact private 401 envelope, complete
  equality of [stored round/roster/parent/entrant/score facts](../../../backend/tests/round_pairings/state.sql#L1),
  including group/member timestamps, and no event while blocked or after
  rejection. Session rows are excluded because the test changes their expiry.
- A fresh valid session traverses each gate, reusing the original optimistic
  timestamp and conversion/group IDs. It must commit exactly one replacement,
  return the correct private model, remove the old identities, preserve unrelated
  parent/player/scoring facts and publish one matching event. There is no
  automatic replay of an expired write.

The [focused run](focused-results.txt) passes 29 cases: 15 pairing mutations,
including the two new tests and their valid controls, plus 14 retained private
pairing/team read cases. The new tests use isolated SQLx databases; their
triggers do not modify a published migration or production schema.

## Browser acceptance and its limits

The [real Chrome probe](browser-probe.mjs) exercises the management editor at
`/manage/tournaments/{id}?round={id}#pairings`, at **320, 390 and 1280px**.
Its [results](browser-results.json) record all three successful widths, with no
unexpected page/console/network errors. A separately migrated/seeded browser
database contains only synthetic fixtures; one participant has a deliberately
long display name.

Each width covers delayed GET loading, populated groups and long names without
horizontal overflow, reachable save/recovery controls at least 44px high, and a
real UI PUT held briefly to inspect saving feedback. Exactly one real PUT must
return private 200; subsequent GET and reload retain the new group names. GET
503 plus explicit retry and a typed empty editor are deliberately injected.
Inspected samples: [mobile group](mobile-long-group.png),
[mobile recovery](mobile-recovery.png), [desktop group](desktop-long-group.png).

The private PUT 401 is also **injected**, while follow-up GETs remain genuinely
authorized. It verifies existing access feedback, disabled editing/save, a
retained current draft, explicit discard/read recovery, unchanged server state
and no automatic PUT retry. It does not reproduce late database expiry in Chrome
or establish draft retention across actual auth-loss unmounts: genuine expiry
also denies refreshed GETs, which can remove the editor. Actual late expiry,
rollback and no-event evidence belongs to the PostgreSQL/API cases above.
Real logout followed by forced reload separately verifies private GET 401 and
login redirection without a private editor.

An initial probe reused the current fragment URL after API logout and therefore
did not force canonical auth refresh. The probe now explicitly reloads. The
corrected complete run passes at all widths; application code did not change.
Focused screenshots were added because full management-page captures are too
tall for useful close visual inspection; the final probe with those captures
also passes all three widths.

## Validation

| Check | Result |
| --- | --- |
| New tests against original production | Both failed as expected: 200 instead of 401 |
| Focused pairing/team suites | 29 passed |
| Rust formatting | Passed |
| Full backend all-target tests | 264 passed, zero failures; one existing manual probe ignored |
| Strict Clippy, all targets/features | Passed; three existing vendored SQLx warnings |
| API binaries | Built |
| Local API health/readiness | Passed |
| Chrome editor, 320/390/1280px | Passed; real saves plus clearly identified injected states |
| Full PostgreSQL-feature ladder, two threads | 950 passed, zero failures; five existing measurement probes ignored |
| Explicit migrate/seed ladder | Passed; all 39 migrations successful, eight players and five rounds |
| Independent implementation/test review | No actionable findings |
| Independent browser-evidence/documentation/next-plan reviews | No actionable findings |
| Final static checks | Local links/source anchors, file scope/size and diff/EOF whitespace passed |

The [retained command results](validation-results.txt) include all 92 target
results and the exact five existing ignored probes. The full ladder also
passes both new pairing-expiry cases.

The local PostgreSQL fixture was version **17.10**, rootless Podman
`postgres:17-alpine`, container
`golf-a1q-20261015-pg`, on loopback port 55461. SQLx owned isolated databases;
browser data lived in separate `golf_a1q_browser`. The owned local API/Vite
processes were stopped before the full database ladder to avoid concurrent
browser load during timing-sensitive tests. After migration/seed acceptance,
the disposable container was stopped and confirmed removed; both owned private
environment files were deleted. Browser contexts were closed.

From the repository root with an approved disposable `DATABASE_URL` set privately:

```bash
cargo test --locked -p golf-api --features database-tests --test round_pairings --test pairing_team_sessions -- --test-threads=2
cargo fmt --all -- --check
cargo test --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo build --locked -p golf-api --bins
cargo test --locked --workspace --all-targets --features database-tests -- --test-threads=2
cargo run --locked -p golf-api --bin migrate
cargo run --locked -p golf-api --bin seed
node docs/validation/pairing-write-session-2026-10-15/browser-probe.mjs
```

Browser reproduction needs separately seeded data, the repaired API on 3000
with development mode and HTTP cookies, existing Vite on 5173, and
`GOLF_A1Q_DEV_PASSWORD` set privately. The probe uses only synthetic data and
does not log credentials or mutation payloads. Logs and full-page screenshots
remain under `/tmp/golf-a1q-*`; environment values are not retained here.
The full PostgreSQL count includes ordinary backend tests and is not additive.

Frontend unit/type/lint/build ladders are unaffected and were not rerun: no
frontend source, response shape, decoding or UI implementation changed. The
approved user-facing gate is the real-browser acceptance above. No external
provider, hosted environment, dependency scanner or performance benchmark was
used. This acceptance does not close W3 or establish hosted readiness.
