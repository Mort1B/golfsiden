# Latest iteration — course-configuration session expiry

A1r repairs W3, `PUT /api/rounds/{round_id}/course-configuration`, from the
[legacy-write inventory](validation/legacy-write-inventory-2026-10-15/README.md).
After inserting the complete immutable revision and awaiting round
UPDATE/RETURNING, the repository rechecks the same session with database
wall-clock time immediately before commit. Stored-round/session/admin locks,
CSRF, preflight/provider placement, validation and response contracts remain
unchanged. No mutation retry or repeated provider fetch is added.

Both new PostgreSQL/API cases reproduced `200` instead of `401` against unchanged
production source. The first observes a final round wait before holding an
unchanged membership, proving preflight completed. The second verifies complete
pending course/tee/hole facts and changed attachment/timestamp inside a targeted
AFTER UPDATE trigger before its advisory wait. Deferred constraints are not
forced early.

Both now return private/no-store `401 unauthenticated`, roll back all hierarchy
and attachment changes and publish no event. Full snapshots catch orphan rows
and preserve the other opened round, original courses and handicap/history
facts. Fresh-session controls reuse the original optimistic timestamp and
command, create exactly one hierarchy/attachment and publish one matching event. All 36 focused cases
pass, and independent read-only implementation/test review found no findings.

Real Chrome checks pass at 320, 390 and 1280px: saved-picker loading/populated,
injected long names/error/retry/empty, manual saving feedback, one real PUT,
receipt/collapse/focus and persisted round names/hole count after reload. A
clearly injected PUT 401 retains inputs in the same mounted form, shows existing
access feedback and re-enables fields/save without an automatic retry. Server
round data is unchanged. Retention across mode changes/reload/auth-loss unmounts
is not claimed. Actual logout plus reload verifies private round GET 401 and
login gating. The real unusable catalog shows its manual fallback; no provider
detail request or remote provider success is claimed. An initial probe selector
was corrected without application changes before the complete successful run.

Browser inspection separately confirms a pre-existing **P3 mobile labeling
issue**: below 700px, manual hole rows lack visible Par/Slagindeks/Avstand labels,
although accessible input names remain. Desktop headings are visible. This is
recorded with screenshots and proposed as A1s; no frontend implementation began.
Independent browser-evidence review confirms that distinction.

Backend formatting, 264 all-target tests, strict Clippy and API binaries pass.
The complete PostgreSQL rerun passes 952 tests with zero failures and five
existing ignored probes. The first full attempt ended with exit 143 after its
unit target; no failing case was reported and its cause remains unconfirmed.
That incomplete attempt is recorded separately. Explicit migration/seed checks
pass with all 39 migrations successful, eight players and five rounds. The
owned container and private environment files were removed. Independent
source/test, browser-evidence and documentation/next-plan reviews are resolved;
local links/source anchors, file scope/size and diff whitespace checks pass.
The [acceptance report](validation/course-write-session-2026-10-15/README.md)
retains exact results, the separate P3 finding and validation limits.
Time during COMMIT, deferred constraints and delivery remains outside the final
check. No schema, dependency, provider or hosted change is included.
