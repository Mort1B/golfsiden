# Latest iteration — invitation metadata session authority

A1l repairs only the invitation metadata GET. A real controlled wait at the
metadata query reproduced the original endpoint returning 200 after session
expiry. The repaired endpoint returns 401 with the existing `no-store` header.
The final check follows all metadata and redemption-count loading, including
empty results, while existing exclusive session/user and exact-admin membership
locks remain held.

The production change is four lines in the repository read: call the existing
exclusive active-session helper after `fetch_all` and before commit, retaining
its existing database/unauthenticated error mapping. Default `READ COMMITTED`,
stronger authority locks, SQL, metadata fields/order, cache behavior and all
mutation/shared-helper contracts remain. No retry was added. A role demotion
committed during a membership wait is observed by the existing locking read and
returns 403 directly; there is no repeatable-read conflict to retry.

An expired invitation remains visible history for an active administrator. An
expired requester session now receives 401 instead of that history. Rotated
predecessor/successor rows still share the same redemption count, and metadata
contains neither raw invitation tokens nor their hashes. No read path issues,
rotates, revokes or replays an invitation write.

Fourteen focused PostgreSQL/API cases passed: membership/metadata/count/empty
expiry, revocation/credential serialization, current exact-admin scope, history/
series/count/order parity, token secrecy, cancellation, unchanged single-attempt
errors and concurrent valid reads without writes or SSE. Independent read-only
source/test review found no actionable issues. Workspace tests, strict Clippy
and API binaries passed, as did all 1,133 frontend tests, typecheck, lint and build.
Chrome passed at 320, 390 and 1280 px: actual typed metadata/history, loading,
read-only error retry, empty, denied and logout states, without page errors,
unexpected network failures or invitation mutations during read observations.
The full PostgreSQL 17.10 Alpine ladder passed 914 tests with four existing
ignored probes/benchmarks; migrate and seed passed through unchanged schema 39.
See the [acceptance report](validation/invitation-read-session-2026-10-15/README.md).

No API source, frontend source, schema, dependency or hosting changes. Existing same-user
serialization remains; each ordinary success adds one session query without a
speedup claim. The final check does not freeze expiry through commit/delivery.
One P2 route and three P3 course routes remain in the historical read inventory.
