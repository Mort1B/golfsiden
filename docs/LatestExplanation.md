# Active rate limits survive capacity pressure

AUTH-1 is repaired. The limiter checks existing quotas before allocating buckets
and rejects new admissions when storage is full. It no longer forgets an
unexpired counter to make room, including when rejected requests vary their
resource keys on another route.

At capacity, existing keys retain their remaining quota. New client/resource
buckets receive the established 429 response until space expires. This can
temporarily deny new identities across routes; `Retry-After` is the earliest expiry
hint, not guaranteed admission. Quotas, client identity, fixed windows and the
8,192-bucket bound are unchanged.

Four regressions failed against the previous code and passed after the fix. The
final 11 limiter tests cover cross-route churn, production capacity, atomic
admission, mixed windows and expiry recovery. A real-router test also verifies the
HTTP contract and preserves the existing account's remaining quota.

Formatting, the 215-test backend ladder, Clippy, migration and seed checks passed.
The complete database-enabled run passed 601 tests (including those 215), with
three existing performance measurements ignored because their PostgreSQL
`pg_stat_statements` configuration was not enabled. Independent read-only source
and documentation review completed. The task-owned disposable database was removed.
Details are in the [repair report](validation/rate-limit-capacity-2026-09-23/README.md).

The AUTH-1 repair is **READY** with its documented capacity tradeoff. Deployment
remains **NOT READY**: AUTH-2's session-expiry boundary, the wider security review
and existing public-host/device gates remain open. Frontend/browser checks were
not rerun for this backend-only repair. Work remains local, without a push, under
the user's existing scope. AUTH-2 is the next proposed bounded step.
