# A1n — P3 course-read session authority, 2026-10-15

This closes the three remaining P3 GETs from the
[private-read inventory](../private-read-inventory-2026-10-14/README.md#courses).
Baseline source: `1ea62d678b92e91b534226e82f34718a2fc0dd72`.
The data is global bundled/registered course facts. No private-score disclosure,
reachable provider-quota abuse or hosted incident is established by this repair.

## Read boundary

| GET suffix under `/api/tournaments/{id}` | Current owner and final check |
| --- | --- |
| `/course-catalog` | `course_reads::local_for_session`; query validation and synchronous bundled search precede the final session check in one transaction |
| `/course-presets` | `course_presets::list_for_session`; unchanged ordered SQL/projection precedes the final session check, including empty results |
| `/course-provider/courses/{provider_id}` | API preflight commits, one unchanged readiness/cache/provider future runs, then separate postflight reauthorizes the same session/tournament before either facts or errors are exposed |

`repositories/course_reads.rs` uses the existing `session_read` transaction and
retry owner. It derives the user from locked active session/user rows, preserves
tournament-existence 404 before exact-admin 403, and retains exact membership
share locks until a fresh database wall-clock session check and commit. A global
administrator has no cross-tournament bypass. The helper runs synchronous local
projection and never awaits a provider future inside its transaction.
Projection errors remain caller-owned and authorization still precedes query
validation. A direct/nested `40001` discards the complete local attempt and allows
one retry; exhaustion returns generic `503 service_unavailable`. Other database
errors remain generic 500 without retry. Expiry is checked before commit, not
frozen through commit, serialization, network delivery or previously returned data.

Provider preflight/postflight own separate local attempts. The fetch is `FnOnce`
and sits outside both retry boundaries: no database locks span provider/cache
work, and retry never repeats quota consumption, fetching or cache population.
Postflight runs after both success and error. A fetch legitimately admitted by
preflight can still consume quota/populate the bounded global course cache if
authority is lost later; its response is denied. No course/tee/hole/round rows
are written by these GETs and no SSE is emitted.

All statuses on the three selected routes now use `private, no-store`, including
session and typed-path extractor failures. Existing cache middleware/error
mapping is reused. Course JSON, ordering, query validation, provider readiness,
ID normalization and provider-error envelopes are preserved for authorized users.
`fetch_course` is byte-identical and its round-configuration mutation caller is
untouched. The historical/internal user-only preset helper remains separate,
sharing its original data SQL/projection and preserving schema-20 upgrade tests;
HTTP uses only the session-bound entry point. No schema, frontend, catalog,
credential, dependency, provider-client or deployment configuration changes.

## Baseline regressions and focused acceptance

The new real PostgreSQL/Axum tests ran before production changes. Seven tests
failed against original source; one unrelated-database-error case passed. The
runner checks the exact seven failing identities, rather than accepting any
cargo exit 101. Both confirmed-baseline and current fixtures use PostgreSQL
17.10 Alpine/musl and synthetic data on ephemeral loopback ports.

| Controlled race or failure | Original result | Repaired result |
| --- | --- | --- |
| Revocation after extraction, before authority | catalog/presets 200; provider 409 incomplete | all three 401, no requested facts |
| Credential replacement in that window | catalog/presets 200; provider 409 incomplete | all three 401 |
| Natural expiry while waiting on membership | catalog/presets 200; provider 409 incomplete | all three 401 |
| Expiry in the final preset-loading wait | populated presets 200 | populated/empty presets 401 |
| Membership demotion invalidates preset snapshot | 500 | fresh exact-admin check returns 403 |
| Repeated synthetic local `40001` | 500 | two attempts, private generic 503 |
| Authentication extractor denial cache | `no-store` | `private, no-store` |

Tests prove a real database wait while the session is still active before
allowing wall-clock expiry. They cover missing/invalid IDs, global-admin foreign
absence, all non-admin membership roles, invalid/empty catalog queries, empty
preset lists, complete payload parity, concurrent pure reads with unchanged
database digests/no SSE, preset-fact snapshot coherence, fresh facts after a
retained-admin snapshot retry and cancellation releasing session authority.

The unchanged bundled catalog still has **zero usable entries**. Actual provider
GET tests preserve its 409 incomplete/404 unknown gates and existing local mock
confirms no upstream request. No production gate or configuration was weakened
to manufacture a usable entry. Conditional provider success is **API-helper-only
coverage**, through the same private orchestration called by the real handler:

- A real loopback provider request remains held while session revocation,
  credential replacement or membership demotion completes. Both successful and
  failed provider results are suppressed after authority loss; valid authority
  preserves success and provider-error mappings.
- Postflight conflict retries once or exhausts at two local attempts, while
  fetch invocation and upstream call counts both stay one; the normalized cache
  entry survives without another request.
- Fresh and cached responses preserve complete normalized JSON; a cache hit
  still requires postflight authority. Cancellation leaves session/membership
  writers available and does not replay work.
- Natural expiry is tested during a held normalized-result future, separately
  from HTTP's unchanged five-second timeout. No external provider is contacted.

## Validation and limits

- Confirmed original regression: one passed, seven expected failures, zero
  ignored. The first runner expected six failures; the additional preset
  membership-conflict 500 was inspected and added to the strict expected set.
  The confirmed rerun accepted exactly seven original failures. Both fixtures
  were removed; no production source changed before that rerun.
- Initial repaired GET suite: eight passed. Expanded snapshot/no-write assertions
  were added afterwards; the final focused run passes the same eight tests with
  those added assertions. [Focused results](focused-results.json) also record
  five provider-helper and six existing course contract tests passing again.
- Conditional provider helper: five passed; existing course contract suites:
  six passed, including historical schema upgrades and independent preset saves.
- `cargo fmt --all -- --check`: passed.
- `cargo test --locked --workspace --all-targets`: 264 passed, one existing
  manual projection benchmark ignored.
- `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`:
  passed. The same three pre-existing vendored SQLx warnings remain.
- `cargo test --locked --workspace --all-targets --features database-tests --
  --test-threads=2`: 946 passed, zero failed, five existing manual/exclusive
  probes ignored. No new ignored test was added. Additional assertions added
  during that run were separately compiled/exercised by the final focused run.
- `cargo run --locked -p golf-api --bin migrate` and `--bin seed`: passed;
  39 successful migrations. Both current full/final-focused owned containers
  were removed, as were both original-regression fixtures.
- Independent read-only design/source, expanded-test and final evidence/docs/
  plan reviews found no issues. Final review independently confirmed all 13
  production/test hashes, preserved fetch/projection/DTOs, test totals, fixture
  cleanup and bounded closure. The next assessment remains unapproved.

The API payload and frontend request/response types are unchanged. Frontend and
mobile/desktop browser ladders are unaffected and were not rerun. This does not
establish physical-device or hosted acceptance. Provider-success behavior is
conditional/helper-only, not a live successful GET with this catalog. No
throughput/latency measurement or capacity claim was made. The inventory closure
applies only to its selected GET routes, not unassessed legacy writes or an
application-wide security guarantee.

## Reproduction and retained evidence

[postgres_ladder.py](postgres_ladder.py) creates a unique container and records
its CID before start, binds an ephemeral loopback port and removes only its own
recorded CID in `finally`, including on failure. It records cleanup as confirmed
only after successful removal. Raw logs stay in the chosen local output path;
JSON contains no database URL or session token. Existing containers are untouched.

```sh
python3 docs/validation/course-read-session-2026-10-15/postgres_ladder.py \
  --phase current --output /tmp/golf-a1n-current
python3 docs/validation/course-read-session-2026-10-15/postgres_ladder.py \
  --phase current --focused-only --output /tmp/golf-a1n-focused-final
```

`--phase baseline` runs only the real-GET regression suite and requires exactly
its seven original failure names. Run it in a separate original-revision
checkout/archive with only the new regression test modules and runner copied in;
use distinct Cargo target directories. Do not roll back the current checkout or
reuse another revision's test executable. Current/focused commands require zero
failures. The full current command also runs conditional-provider and existing
course suites, the whole two-thread PostgreSQL ladder, migrate and seed.

The [confirmed baseline results](baseline-results.json) and
[sanitized original failure output](baseline-regression.txt) retain the seven
failures. [Backend results](backend-results.json) and [final focused
results](focused-results.json) retain exact commands/counts and fixture cleanup.
[Full PostgreSQL results](postgres-results.json) retain the full ladder and
migration/seed outcomes. [Source checks](source-checks.json) record the current
production/test hashes, bounded production file sizes, unchanged provider fetch,
preset SQL/projection/DTO fields and untouched round-configuration mutation caller.
