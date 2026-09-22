# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. The previous bounded investigation is complete; await approval before
starting the next candidate.

## Next candidate

Add a player-filtered full-card listing for selected-player match history.

**Goal:** avoid transferring and decoding other players' cards while preserving
current history results and full-card coherence validation, as supported by the
[measurement](performance/history-payload/README.md).

**Scope and behavior:** add a typed optional player filter to the round listing
read with explicit round/player response identity. Select matching IDs in existing
stable order before card construction; retain current membership, visibility,
frozen-handicap and independent writable checks. Return the same permitted full
cards and writable-ID intersection as filtering the current listing. A valid
absent/nonparticipant player yields an empty list. Preserve malformed/noncanonical
URL-filter behavior deliberately; do not silently normalize it into new results.

Route only selected-player results/history to a separate account+round+player
query key under the existing private read-list prefix. Validate target identity,
card membership and writable subsets before caching. Keep unfiltered GET,
assignment PUT, management, table, detail, scoring and recovery contracts and
behavior unchanged. Do not populate an unfiltered key from a filtered response.
No compact-summary DTO or authorization-query optimization is included.

**Invariants:** unchanged strict full-card evidence checks, nullable hidden
metadata and permitted early finishes, independent scoring authority, ordering,
readiness gate, account/round/player isolation, synchronous erasure, cancelled
late-response guards, 20-second freshness and required SSE/return refreshes.

**Dependencies and validation:** first establish an approved disposable PostgreSQL
environment. Add repository/API parity tests against filtering the original
listing for both opponent slots, roles and round states, membership loss, missing
players, final hiding/release/corrections and writable subsets; verify authenticated
private/no-store responses and malformed parameters. Run affected backend,
PostgreSQL and frontend ladders. Cover decoder target rejection, both cache variants,
player/account changes, empty/loading/error states, shared management, all denials,
held responses, readiness order and mutation/SSE/return invalidation. Validate actual
restricted-final payloads at 320/390/1280px and reproduce native byte/decoder savings.
Measure all-results→player and player→player navigation: separate filtered cache
keys can add a request where today's unfiltered cache is reused.

**Stop:** publish only this bounded repair after at-most-one-card-per-round
history, permitted-result/action parity, request/byte evidence and privacy checks
pass. If database validation is unavailable, record that blocker without claiming
completion. Do not change freshness or start broader database/security work.

## Later queue

1. **Database authorization measurements:** acquire approved disposable PostgreSQL
   measurements before proposing a list-authorization repair. Preserve authority
   refresh and fail-closed behavior; do not infer timing from browser fixtures.
2. **Security review:** perform the separately scoped wider application/operational
   review after agreed performance repairs.

No automatic opponents/byes/brackets, team match play, extra holes, new scoring
rules, public match sharing, cold offline launch or background sync is included.
