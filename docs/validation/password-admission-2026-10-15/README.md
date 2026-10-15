# Bounded password-work admission — 2026-10-15

A6 addresses the P2 [password-admission finding](../application-review-2026-10-14/README.md#a6--p2-password-work-admission-has-no-explicit-queue-bound).
Production baseline: `81689e68f07d0b3ac7b6600de39bbc1d0feb65e6`.

## Failure and repair

The [baseline regression](baseline.txt) fills the original four worker slots with
blocked synthetic operations, then explicitly polls four more operations into
waiting. A ninth operation does not reject within 200 ms. The test opens the gate
and joins the remaining work before its expected failing assertion. This is a
controlled helper reproduction, not a production load or denial-of-service test.

The repair adds a separate eight-operation admission semaphore around the
existing four-worker boundary. Admission uses `try_acquire_owned`, so excess work
returns the existing `CapacityUnavailable` error without waiting or spawning a
blocking task. Hashing and real/dummy verification use the same process-local
limiter. No production caller, Argon2 parameter, credential/session recheck,
request limiter, HTTP error mapping, schema or dependency changes.

Waiting cancellation drops admission. After worker acquisition, both owned
permits move into the blocking closure and remain held through execution and
Tokio blocking-pool queuing, including after its request is cancelled. Completion,
operation errors and panic unwinding release capacity. Worker slots count work
dispatched to the blocking pool, including queued closures; they do not promise
four running OS threads. The synchronous seed hash helper is unchanged.

The eight-operation policy bounds admitted password-helper work, not all HTTP
requests, pre-helper database lookups, retained request bodies or process memory.
It does not establish a safe arrival rate, production throughput or latency.
No overload deadline, new response status or automatic retry is introduced.

## Deterministic unit acceptance

The [seven focused checks](focused-results.txt) live in
[the private test module](../../../backend/src/auth/password/tests.rs):

- Four blocked operations consume all worker slots; four explicitly polled
  futures consume the remaining admission. The ninth operation rejects in one
  poll without running its closure. All admitted work later completes and fresh
  work succeeds.
- Dropping a waiting future returns admission without running its operation;
  replacement work can queue and complete.
- Aborting an active request retains both permits until its blocked closure
  finishes, then legitimate work recovers.
- A dedicated runtime with one occupied blocking thread proves dispatched but
  pool-queued work retains both permits after request cancellation.
- Operation error, worker panic and closed-semaphore errors return capacity;
  normal work succeeds again after recoverable failures.
- Real entry points share admission: both hashing and verification reject in one
  poll under the same held budget; hashing succeeds after release.
- Argon2id defaults, successful verification, incorrect-password rejection and
  malformed-hash rejection retain their behavior.

A shared async test lock coordinates all unit tests touching the production
static. Scheduling tests use isolated local limiters. Gates open on drop so a
failed assertion does not leave blocking threads parked indefinitely.

## PostgreSQL/API acceptance

The private database unit test
[exercises the actual router](../../../backend/src/auth/password/api_tests.rs)
without exposing a production test hook. It holds the shared admission budget
while making real requests against an isolated migrated PostgreSQL database:

- Known-user and unknown-user dummy login both promptly return the unchanged
  generic 500 `internal_error` JSON and no `Set-Cookie`.
- A valid creator-onboarding request fails with the same generic response.
- Counts across ten account/session/tournament-related tables remain unchanged.
- Releasing admission permits real login and creator onboarding; known wrong
  passwords and unknown accounts retain identical generic 401 responses, no
  cookie and `no-store`. Exactly the two successful operations create sessions.

This verifies overload behavior independently from route rate limits; the test's
ordinary development state disables that separate limiter. It does not assert
that password saturation returns 429 or refunds already charged route quotas.

The [retained local ladder](postgres_ladder.py) owns a unique PostgreSQL container
and an ephemeral loopback-only port. It records the container ID before startup,
uses synthetic data and removes only that ID in `finally`. Local raw logs contain
no request payloads or configuration dumps; retained results contain no database
URLs, encoded password hashes or session secrets.

## Validation

| Check | Result |
| --- | --- |
| Original helper regression | Expected failure: ninth work item waited rather than rejecting |
| Focused helper suite | Seven passed |
| Focused PostgreSQL/API saturation case | One passed |
| Workspace tests | 264 passed, one existing ignored benchmark |
| Formatting | Passed |
| Strict all-target/all-feature Clippy | Passed; three unchanged vendored SQLx warnings |
| Controlled fixture startup/removal reporting | Both cases passed with fake Podman only |
| Complete PostgreSQL suite | 932 passed, four existing ignored; exit 0, 818.72 seconds |
| Migrate/seed | Passed; all 39 migrations current |
| Owned live fixture removal | Confirmed by harness result and separate container inventory |
| Independent read-only review | No remaining source/test/harness/documentation findings |

[Backend results](backend-results.json) record successful command timings. The
[first sandboxed workspace run](sandbox-listener-limit.txt) failed eight
unchanged provider tests because local-listener binding returned `PermissionDenied`. The rerun with local socket
permissions passed the complete suite. This was a fixture permission limit;
no source repair or skipped test was used. Strict Clippy retains the same three
warnings in unchanged vendored SQLx code.

The reviewer found one harness reporting defect: it could print successful
cleanup after a failed removal. The retained harness now reports success only
when removal succeeds and retains its exact owned container ID in results on
failure. [Two controlled startup/removal cases](cleanup-controls.json) verify
both outcomes without invoking real Podman. The live successful ladder started
before this reporting-only edit; its removal result is true only after successful
`podman rm`. A separate final container inventory confirmed no A6 fixture remains,
and its temporary fixture directory was removed.

[PostgreSQL results](postgres-results.json) retain exact commands, timings,
version, schema count and removal confirmation. Database-enabled totals include
ordinary tests and must not be added to the backend count. The four unchanged
ignored cases are the Fantasy projection benchmark and the three match profiling
probes (`hundred_matches`, `twelve_matches`, `twenty_four_matches`). No new ignored
test was added. The full ladder used PostgreSQL 17.10 Alpine/musl, two test threads
and only its own disposable data. Raw local logs remain outside the repository;
the [validation summary](validation-results.txt) records the accepted results.
No production source changed after the successful validation began.

## Limits and remaining work

The runtime repair is confined to `backend/src/auth/password.rs`; tests are split
into private modules. API handlers, credential generations, transaction ordering,
SQLx cancellation override, migrations, Cargo lockfile, frontend source and
protocols remain unchanged. There is no scoring, handicap or locked-round change.

This is local helper/API/database evidence. It does not validate the hosted
service, multi-instance global capacity, browser UI or production image/deploy
behavior. Frontend/browser ladders are unaffected and are not rerun. Rebuild the
API to adopt the change; no migration or environment setting is required.

Development dependency advisories remain a separately selected candidate in
[the plan](../../PLANS.md). No queued dependency or query optimization is included.
