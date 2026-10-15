# Latest iteration — round-creation session expiry

A1p repairs direct `POST /api/tournaments/{tournament_id}/rounds`, W1 in the
[legacy-write inventory](validation/legacy-write-inventory-2026-10-15/README.md).
After INSERT/RETURNING and immediate triggers, the repository rechecks the same
session with database wall-clock time immediately before commit. It retains the
existing session/user and exact-admin membership locks and error mapping.

An initially active session could previously expire during the membership wait
or insert work, then still create a round. Both new controlled API tests
reproduced `201` instead of `401` against unchanged production code. They now
return `401 unauthenticated` / `no-store`, roll back the complete round and
conditional Fantasy window, preserve parent/game/overall-guard facts and publish
no event. The late test verifies those pending round/child rows inside its
isolated AFTER INSERT trigger before waiting.

Fresh valid-session controls traverse the same gates, create exactly one draft
round and child, return `201` and publish one matching event. All six focused
round-creation tests pass, including the four retained admission/preflight/
authorization/commit cases. No mutation retry is added, and the final check does
not freeze time inside COMMIT, deferred constraints or network delivery.

Backend formatting, 264 all-target tests and strict Clippy pass. Eight existing
provider-unit tests first hit the sandbox's loopback-bind restriction; the full
rerun with socket permission passed without an external provider call.
Independent read-only authorization/test review found no actionable findings.
The full PostgreSQL-feature ladder passes 948 tests with zero failures and five
existing manual probes ignored, including a repeat of all six round-creation
cases. Migrate and seed pass with all 39 migrations. The owned local PostgreSQL
container and private environment files were removed. Independent documentation/
next-plan review and source-size, link, anchor and diff checks also pass. The
[acceptance report](validation/round-creation-session-2026-10-15/README.md)
records baseline failures, focused/full results, commands and limits.

No production UI consumer calls this direct POST; creation screens use excluded
aggregate APIs. No frontend, schema, provider or dependency change is included.
Pairing replacement and course configuration remain separate P2 repairs, as do
provisional preflight observations and retained helper cleanup. No adjacent
implementation began. The [plan](PLANS.md) closes A1p and proposes pairing
replacement alone as the next bounded repair.
