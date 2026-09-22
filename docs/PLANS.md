# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. The listing-only shared authorization repair is complete with a recorded
transaction-start limitation; see the [latest explanation](LatestExplanation.md).

## Next candidate

**Cancellation-safe transaction initialization and pooled connection reuse
(awaiting approval).** Diagnose and repair the independently reproduced SQLx 0.8.6
transaction-start cancellation defect, and trace the desktop return failure that
is consistent with it. The retained [probe and evidence](performance/listing-authorization/README.md#known-transaction-start-cancellation-issue)
reproduce SQLSTATE 25001 without application authorization code by widening the
BEGIN/cancellation window. Establish a controlled ordinary-transaction cancellation
reproducer before choosing a dependency or application lifecycle fix.

Scope: interrupted transaction initialization and safe pool reuse only. Prove that
cancelled begins/requests are rolled back or their connections retired, preserving
isolation level, clean cross-request state, active session/membership authority and
private-result freshness. Do not suppress the 500, disable cancellation/refreshes,
relax repeatable-read semantics or broaden scoring/security behavior.

Validate deterministic cancellation and connection-reuse cases, cross-account and
cross-request isolation, rollback/lock release, existing authorization expiry races,
and real mobile/desktop return flows. Run affected ladders and measure any pool or
query-count overhead. Stop after the scoped transaction-lifecycle repair is
reproduced, reviewed and validated; broader security work remains separate.

## Later queue

1. **Security review:** perform the separately scoped wider application/operational
   review after the transaction repair and agreed performance work.

No automatic opponents/byes/brackets, team match play, extra holes, new scoring
rules, public match sharing, cold offline launch or background sync is included.
