# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None.

## Next candidate

### A1 — P2: bind private leaderboard reads to the active session

Evidence and the complete severity-rated assessment:
[application review](validation/application-review-2026-10-14/README.md).
This candidate is planned, not authorized for implementation by the review request.

- Goal: prevent a private leaderboard request from using authority from a session
  already revoked or replaced before its result transaction authorizes access.
- Scope: round/tournament gross/net leaderboard handlers and repositories,
  narrowly scoped session-aware authorization support, PostgreSQL/API regressions
  and affected documentation. No frontend, scoring, schema or deployment change.
- Behavior: pass session identity into the result transaction, lock/check the
  session/user and exact membership with a deliberate lock order, retain authority
  locks through assembly and recheck natural expiry before completion. Define
  deliberate handling for repeatable-read serialization failures. Preserve API
  envelopes, cache headers, visibility and valid-session response parity.
- Validation: reproduce the captured membership-wait/revocation sequence and
  credential-generation replacement; cover expiry, valid callers, nonmembers,
  privacy, transaction cancellation and concurrent reads. Run the full backend
  and PostgreSQL ladders, including migration/seed checks; read-only review.
- Invariants: preserve membership/session isolation, final-round embargo,
  handicap snapshots, sporting data, deterministic standings and SSE semantics.
- Stop: publish the validated leaderboard repair only. Sibling setup reads and
  A2–A8 remain separate candidates.

## Later queue

### Severity-ordered review follow-ups

All findings and individual acceptance criteria are in the
[review report](validation/application-review-2026-10-14/README.md#findings-and-acceptance-criteria).
Select one bounded step at a time; none is implemented by this plan.

1. **P2 / A2:** recheck expiry before committing round open/complete/lock; assert
   rejection rolls back status/snapshots and emits no invalidation.
2. **P2 / A3:** coordinate anonymous onboarding/invitation authentication across
   route lifetimes, including cookie identity and reverse response order.
3. **P2 / A1 follow-up:** inventory and repair the remaining user-only private
   tournament/round/roster/pairing/team read families in bounded slices; inventory
   analogous expiry gaps in legacy team/round writes alongside A2.
4. **P2 / A4:** update reachable Rustls to a compatible patched release, retain
   the SQLx cancellation patch and recheck TLS behavior/advisory reachability.
5. **P2 / A5:** require and bind the restore checksum before invoking restore;
   preserve empty-target and single-transaction protections.
6. **P2 / A6:** cap password-work admission while retaining four active Argon2
   workers and cancellation-safe capacity release. Capacity impact is unmeasured.
7. **P2 / A7:** upgrade affected development tools in compatible, validated
   dependency steps. Scanner critical labels do not establish production RCE.
8. **P3 / A8:** batch team memberships, preserving ordering and authority; measure
   query count and end-to-end cost. No current frontend latency impact is proven.

Only after these, consider measurement-only steps for remaining Fantasy
invalidation/fact-loading cost and empty-queue polling. No speculative optimization
or load claim is authorized.

### Previously deferred work

- Optional offline preparation for browser closure/eviction: requires a separately
  scoped app-shell and private scorecard-retention/reauthorization design before
  implementation. Current in-memory return and durable queued edits remain distinct.

- Operator-owned security acceptance remains for the actual public proxy/TLS,
  runtime database privileges, image advisories and backup/restore environment.
  The local application assessment does not establish hosted readiness.

### Remaining deployment acceptance

Physical iPhone acceptance remains for Chrome and Safari separately: iPhone 15
Pro and a newer/larger model, both orientations, browser toolbars, software
keyboard, safe areas and native zoom. Local Chrome/WebKit coverage does not
replace those checks; see [the evidence and device checklist](validation/iphone-layout-2026-10-14/README.md#outstanding-physical-device-acceptance).
No further mobile implementation step is selected. Duo/foldables remain excluded.

The remaining PERSIST-1 same-account/actual-login scenarios may be revisited if
testing feedback warrants them; see the original
[validation limits](validation/match-note-retention-2026-09-23/README.md). Local
PostgreSQL validation is now available through rootless Podman. The functional
checks cover real database-backed match recovery after locking, but do not claim
every separate session-replacement scenario.

Public-host acceptance still needs a Docker Engine environment and public DNS/TLS
validation. Native 200% browser zoom and physical Android Chrome remain unverified.
Hosting and deployment checks belong to the user. These historical acceptance
limits do not expand the application-readiness step into server administration
or another assessment. Broader device coverage and polish can follow feedback.

No further sporting scoring rules are planned. No automatic opponents/byes/brackets, team match play,
extra holes, public match sharing, cold offline launch or background sync is included.
Fantasy remains separate from sporting scores and completion rules.
