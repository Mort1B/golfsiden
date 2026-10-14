# Session-bound roster and round reads

Private tournament rosters and round list/detail now carry the session ID through
complete result loading. The new controlled regression showed all three routes
returning HTTP 200 after revocation committed while they waited on membership.
The same test now returns the established private 401 envelope on each route.

`repositories/session_read.rs` shares the existing A1b transaction authority:
one repeatable-read attempt, active session/user share locks before exact
membership, a final database wall-clock expiry check and one complete retry on
40001. The public `TournamentReadError` alias preserves A1b callers and behavior.
Roster and round SQL remain in their existing repositories, and a round's
membership scope comes from its stored parent. No user, role or payload from a
failed snapshot survives retry; a second conflict returns generic private 503.

For example, an otherwise valid roster that expires while loading its final
handicap-lock query returns no player data. Empty roster and round-list replies
must also pass the final check. Later revocation or credential writes wait behind
readers that already hold authority locks. The check does not freeze time during
COMMIT or retract previously delivered results.

Roster ordering, preserved entrant handicaps, first handicap-lock reason and all
round fields/order are unchanged. Each ordinary read adds two session queries,
with no per-player/per-round authorization loop or measured speedup claim. Shared
API mapping/cache support applies `private, no-store` to all selected GET replies.
Round POST creation and its existing preflight keep their behavior; no mutation
retry, sibling repair, schema, frontend, dependency or hosting change is included.

Thirteen new PostgreSQL/API cases cover revocation, credential replacement,
membership deletion/demotion, expiry during controlled waits, empty collections,
twelve concurrent valid reads without writes/SSE, cancellation, every role,
missing/cross-trip targets, payload parity and GET-only cache behavior. The empty
fixtures create a naturally empty tournament; real opening preserves
`snapshot_captured` as the first handicap-lock reason. Independent read-only
review found no remaining issues after those fixture assumptions were corrected.

Focused suites pass 43 tests and the shared retry helpers pass two. The full
backend suite passes 261 tests; the PostgreSQL-feature ladder passes 783 with
four existing manual performance probes ignored. Formatting, strict Clippy,
all API binary builds, explicit migration and development seed pass.

All nine Chrome flows pass across 320, 390 and 1280px with real API/PostgreSQL
payloads, delayed loading, long entrant/course/tee names, error/read-only retry,
empty roster/list or denied detail, and private 401s after logout. No page errors
or unexpected network failures occurred; representative mobile/desktop screenshots
were inspected. [The report](validation/roster-round-session-2026-10-14/README.md)
retains the original failure, browser evidence, exact gate semantics and limits.
Physical-device, public-host and production-load acceptance remain unchanged.

[PLANS.md](PLANS.md) selects pairing/team GET session binding as A1d next;
that work and the other severity-rated findings remain queued.
