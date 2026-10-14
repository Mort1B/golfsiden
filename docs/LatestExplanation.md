# Session-bound pairing and team reads

The private pairing-layout and team-list GETs now carry the session ID through
complete payload assembly. A controlled regression showed both previously
returning HTTP 200 after session revocation committed while they waited on
membership. The same test now returns the existing private 401 body on both routes.

Both repositories reuse the unchanged `session_read.rs` boundary: one
repeatable-read attempt, initial session/user share locks before exact membership,
a final database wall-clock expiry check, and one full retry on snapshot
invalidation. Round ownership comes from its stored tournament relation. All
entrant, group and member queries finish before the final check, including empty
layouts/lists. Revocation or credential replacement that wins authority denies
the read; a later writer waits behind an already authorized reader. The clock is
not frozen during COMMIT, and already delivered responses are not retracted.

For example, a pairing request that expires while loading final flight members
returns no layout. Membership changes that invalidate an attempt discard its
user, authority and payload; a second snapshot conflict returns generic private
503. Pairing GET uses shared error mapping while PUT/preflight keep their existing
error and mutation boundaries. Existing pairing cache middleware remains in place;
team GET adds `private, no-store` for every status. No mutation gains a retry.

Payload loading is unchanged: flat team JSON, active/inactive entrants, schedules,
timestamps, distinct group identities, legacy individual groups and the endpoints'
different name/member ordering are preserved. Each ordinary read adds two session
queries. The existing member query per team remains A8; no batching or measured
speedup is claimed. Readiness, scoring, schema, frontend source, dependencies and
hosting configuration remain outside this repair.

Fourteen new PostgreSQL/API cases cover revocation, credential generation,
later-write serialization, membership changes with fresh payloads, coherent
multi-query snapshots, expiry during membership/data loading, empty results,
twelve concurrent valid reads, cancellation, all roles, missing/cross-trip targets
and legacy contracts. Two fixture assertions were corrected to match the existing
flat team JSON. Independent read-only review found no remaining findings.

Focused suites pass 31 tests; shared retry helpers pass two. The full backend
suite passes 261 tests, and the PostgreSQL-feature ladder passes 797 with four
existing manual performance probes ignored, including retained A1b/A1c and pairing
PUT coverage. Formatting, strict Clippy, all API builds, migration and seed pass.

Chrome passes at 320, 390 and 1280px. The actual pairing UI covers loading,
populated long group names, error/read-only retry, empty and denied states.
`/teams` has no current UI consumer; its actual typed API/decoder and HTTP/cache
contracts were tested separately inside the browser. Both routes return private
401s after logout. No page errors or unexpected network failures occurred, and
representative mobile/desktop screenshots were inspected. [The report](validation/pairing-team-session-2026-10-14/README.md)
retains the original failure, browser results, exact wait gates and limits.
Physical-device, public-host and production-load acceptance remain unchanged.

[PLANS.md](PLANS.md) selects A1e's pairing/completion readiness GETs next;
that work and the remaining severity-rated findings stay queued.
