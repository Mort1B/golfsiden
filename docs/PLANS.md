# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. The previous bounded step is complete; await approval before starting the
next candidate.

## Next candidate

Investigate full-card payloads used by match-result history.

**Goal:** determine whether returning full round match cards for player-filtered
history creates a material remaining cost after the lifecycle repairs.

**Scope and behavior:** trace current history/list consumers, response shape,
decoding and filtering. Use representative synthetic populated data to separate
transfer, decoding and rendering cost from required authority refreshes. Document
baseline evidence and, only if supported, propose one bounded follow-up. This is
an investigation: do not change production queries, API contracts or payloads.

**Invariants:** preserve private-result authority, synchronous erasure, account
isolation, restricted-final projections, cancellation, freshness, writable intent
and scoring rules. Do not infer server/database savings from browser fixtures.

**Validation:** retain reproducible inputs, source/build provenance, request and
payload counts, browser evidence at mobile/desktop widths and explicit limitations.
Check affected consumers before proposing a narrower response contract. Run checks
appropriate to any investigation harness or documentation changes.

**Stop:** publish findings and a supported bounded proposal, or explicitly record
insufficient evidence. Do not implement the proposal or start database/security work.

## Later queue

1. **Database authorization measurements:** acquire an approved disposable
   PostgreSQL environment and measurements before proposing a list-authorization
   repair. Preserve authority refresh and fail-closed behavior.
2. **Security review:** perform the separately scoped wider application/operational
   review after agreed performance repairs.

No automatic opponents/byes/brackets, team match play, extra holes, new scoring
rules, public match sharing, cold offline launch or background sync is included.
