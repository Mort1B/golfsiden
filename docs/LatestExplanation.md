# Latest iteration — leaderboard expiry-test timing

A1y repairs a P3 timing exposure in one retained leaderboard integration test.
Its two-second session deadline previously started before connection acquisition,
blocker lookup and scores-gate locking. The original historical timeout did not
reproduce in three fresh default-concurrency module runs or the two-thread
control; its precise cause remains unproven.

A controlled 2250ms pre-gate delay reproduced the original wait timeout. The
instrumented same-order control proved that the session had expired and returned
an early 401 without reaching scores. Arming expiry after gate acquisition passed
the same delay for all four routes. This demonstrates a fixture exposure and its
repair, without attributing the old failure to an unobserved historical delay.

The final test retains the two-second lifetime, five-second waits and all
401/no-store/unauthenticated assertions. It now races early completion against
an exact ungranted scores lock blocked by the owned gate, requires positive
database-clock lifetime there, then waits for natural expiry before release.
An early 401 fails setup; a late successful response still fails. No retry,
artificial delay, production source, API, schema, dependency or sporting change
remains. Its relative-time diagnostics contain no credentials, identities or
payloads. `query_wait_ms` describes query age, not isolated lock-wait cost.

Three repaired default module repetitions and the two-thread control pass all
10 tests each. All 16 selected route requests reached scores while active,
expired while blocked and returned the expected private denial. Formatting,
264 ordinary Rust tests and all-feature Clippy pass. Independent review found
no weakened race or source defect; a P3 evidence-runner acceptance gap was fixed
with exact counts, target execution, four-board ordering and cleanup checks.

The full default-concurrency PostgreSQL run passed the selected leaderboard case
but stopped in the unchanged match-lifecycle late-expiry test: 477 passed,
1 failed, 1 ignored. Only the failed name/counts were retained, so its exact cause
is unknown. This step does not repair that separate case. The complete two-thread
continuation passes 953 tests with five manual probes ignored; migration and seed
pass. All seven receipts and final source fingerprints pass the offline audit,
which explicitly labels the default run as blocked. Owned databases, raw logs and
temporary credentials are removed. Exact commands, review and limits are in the
[acceptance report](validation/leaderboard-expiry-timing-2026-10-15/README.md).
The next candidate is investigation of the separate lifecycle failure.
Frontend/browser and release-build checks are unaffected by this test-only scope.
Local fixture evidence establishes no universal flake elimination, load capacity
or hosted security readiness.
