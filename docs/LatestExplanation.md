# Latest iteration — pairing-replacement session expiry

A1q repairs W2, aggregate `PUT /api/rounds/{round_id}/pairings`, from the
[legacy-write inventory](validation/legacy-write-inventory-2026-10-15/README.md).
After all roster mutations, timestamp updates and complete response assembly,
the repository rechecks the same active session with database wall-clock time
immediately before commit. Existing round/session/admin locks, validation,
response fields and error mapping remain intact. No mutation retry is added.

Both new API/PostgreSQL cases reproduced `200` instead of `401` on unchanged
production code. The membership test first observes the final transaction's
round wait, then holds the unchanged membership after preflight. The late test
verifies pending legacy-team/member and old-flight deletion, new flights/members
and a changed timestamp inside its isolated trigger before waiting. It covers a
late-write wait; a separate final-loader wait was not reproduced.

Both now return private/no-store `401 unauthenticated`, restore the complete
stored roster/round snapshot and publish no event. Fresh valid-session controls
reuse the original optimistic timestamp and identities, commit one replacement
and publish one matching event. All 29 focused pairing/team cases pass.
Independent read-only source/test review found no actionable findings.

Real Chrome editor checks pass at 320, 390 and 1280px: loading, populated/empty,
long-content layout, saving feedback, one real PUT and persisted reload, read
error/retry and no unexpected browser/network errors. A deliberately injected
PUT 401 leaves GET authority valid and checks existing access feedback, disabled
editing, current-draft retention, explicit discard/read recovery and no mutation
retry. This does not claim retained drafts across genuine auth-loss unmounts.
Actual logout plus forced reload separately confirms private 401/login gating.
An initial probe's same-fragment navigation assumption was corrected without
application changes; the complete final probe and focused screenshots pass.

Backend formatting, 264 all-target tests, strict Clippy and API binaries pass.
The complete PostgreSQL-feature ladder passes 950 tests with zero failures and
five existing ignored measurement probes. Explicit migration/seed checks pass
with all 39 migrations successful, eight players and five rounds. Independent
source/test, browser-evidence and documentation/next-plan reviews found no
actionable findings. Local links, source anchors, file scope/size and diff
whitespace checks pass. Evidence and remaining limits are in the
[acceptance report](validation/pairing-write-session-2026-10-15/README.md).
Time during COMMIT, deferred constraints and network delivery remains outside
the final-check guarantee. No frontend, schema, dependency, provider or hosted
change is included. Course configuration remains the separate P2 follow-up;
no adjacent implementation began.
