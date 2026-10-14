# Optional closest-to-pin and longest-drive awards

Tournament administrators can now record either optional award after a non-match
round is completed or locked. Each subtracts one from the winner's personal net
result; both may go to the same player for −2. A reason accompanies every change,
and replacements, removals and locked-round awards require explicit correction
acknowledgment. Neither award is needed to complete or lock a round.

For example, a player with net 72 who wins both competitions receives net 70.
In a team round their partner still receives 72 and the shared team result stays
72. Stableford instead shows a separate competition-point bonus: 36 native net
points become 38 competition points, with an overall equivalent of −2. Actual
strokes, native card points, handicap snapshots, gross results and all Fantasy
points—including placement—remain unchanged.

Schema 38 stores one current slot per round/type and immutable command receipts.
Removing an award preserves its revision/history. Parent-round locking, exact
admin/session checks, database guards and explicit revisions protect concurrent
corrections. Identical replay returns the original receipt without another
adjustment or live event. Net standings/history display base and adjusted values;
hidden final results conceal awards, and public sharing keeps its allowlist.

The management provider retains only drafts, exact uncertain requests and own
receipts across query-driven unmounts. A lost response offers exact replay even
when award reads fail. Acknowledged submissions followed by failed refresh offer
read-only retry. Account/session changes clear recovery. Optional award-read
failure cannot block lifecycle readiness. Read-only review found three recovery
and receipt-identity gaps; all were fixed with regression coverage.

Validation passed: 254 backend tests, 725 PostgreSQL-enabled tests, strict Rust
lint/format checks, clean and populated-schema upgrades, repeated seeds and the
restricted runtime login. Frontend validation passed 1,083 tests across 140 files,
typecheck, lint, build and strict browser TypeScript. Four new Chrome scenarios
and 12 retained scenarios passed, with mobile/desktop evidence at 320/390/1280px.
Three existing exclusive database performance probes remain ignored. See the
[acceptance report and limits](validation/round-awards-2026-10-14/README.md).

Deploy schema 38, refresh runtime permissions and use matching API/frontend builds
as described in the [deployment guide](deployment_guide.md#schema-38-optional-round-awards).
No hosted infrastructure was changed. The Fantasy ace/albatross scale remains
queued in [PLANS.md](PLANS.md); this step changes no Fantasy scoring rules.
