# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. The player-filtered full-card history step is complete; see the
[latest explanation](LatestExplanation.md).

## Next candidate

**Database authorization measurements (awaiting approval).** Use an approved
disposable PostgreSQL database to measure current unfiltered and player-filtered
listing authorization query counts/latency under representative roles and sizes.
Define reproducible fixtures, warm/cold conditions and semantic parity before
proposing any repair. Preserve live membership, independent writable authority,
restricted-final projection and fail-closed behavior. Browser fixture timings are
not database evidence. Stop after a bounded measurement report and a concrete
candidate; no authorization-query implementation is included without approval.

## Later queue

1. **Security review:** perform the separately scoped wider application/operational
   review after agreed performance repairs.

No automatic opponents/byes/brackets, team match play, extra holes, new scoring
rules, public match sharing, cold offline launch or background sync is included.
