# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. Database authorization measurement is complete; see the
[report](performance/database-authorization/README.md) and
[latest explanation](LatestExplanation.md).

## Next candidate

**Listing-only shared authorization context (awaiting approval).** Resolve eligible
player owners once inside the existing match-list transaction, then require both
opponents to be eligible for writable discovery. Reduce repeated owner-set queries
and authorization lookups for full and filtered listings without changing card
construction, stable ordering, visibility, frozen handicaps, locked/draft writable
rules or API/cache contracts. Do not route through the separately transactional
`writable_owners` helper or change scoring/mutation authorization paths.

Preserve live-session/membership locking, linked-player and flight/snapshot scope,
fail-closed errors and expiry after waits/before commit. Define the expiry recheck
policy explicitly; removing repeated wall-clock checks must not extend authority.
Validate exact full/filtered result parity, roles, empty/absent players, hidden and
released finals, expiry/revocation/membership races and locked rounds against
PostgreSQL. Run affected validation ladders and repeat the retained release
measurement. Stop after reviewed, measured listing-only improvement with unchanged
privacy/authority behavior; do not include bulk card loading or wider auth refactors.

## Later queue

1. **Security review:** perform the separately scoped wider application/operational
   review after agreed performance repairs.

No automatic opponents/byes/brackets, team match play, extra holes, new scoring
rules, public match sharing, cold offline launch or background sync is included.
