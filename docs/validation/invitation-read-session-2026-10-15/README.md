# Session-bound invitation metadata GET — 2026-10-15

A1l closes the P2 [invitation metadata boundary](../private-read-inventory-2026-10-14/README.md#invitations):
`GET /api/tournaments/{tournament}/invitations`.

## Original failure and repair

Baseline production source: `8e23d043d7c2162c88dcaa06c91156f03e26200f`, with the new
regression applied before the four-line repair.

The [original regression](baseline.txt) holds `tournament_invitations`, starts an
admin metadata GET and observes its real database wait while the session is
active. It waits for natural expiry against database wall-clock time, then
releases the unchanged relation lock. The original GET returns **200 after
expiry**. The same regression now returns **401 unauthenticated** with the exact
existing `no-store` header, unchanged business state and no SSE. Retained evidence
contains only a synthetic path/status/error code, with no metadata or secrets.

The production diff is four lines in `invitations::admin::list`: call the existing
`lock_active_session_exclusive` after `fetch_all`, map the existing database and
unauthenticated errors, then commit. Initial exclusive session/user and exact
admin membership locks remain held. Empty results still cross that final check.
The check covers expiry during earlier waits; it does not freeze time through
commit, serialization or delivery, or retract an already delivered response.

Default `READ COMMITTED` and all stronger `FOR UPDATE` authority locks are
unchanged. Metadata and series redemption counts still come from one SQL
statement snapshot; this is not whole-transaction repeatable-read behavior.
Membership demotion/removal committed while the locking read waits is observed
without a retry. No retry was added. Synthetic `40001` and unrelated `XX000`
retain the existing single-attempt generic 500 mapping. Lookup order remains
active session, tournament existence, exact-admin membership, metadata. Invalid
management UUIDs retain 404. All statuses retain `no-store`.

Metadata fields, deterministic `created_at DESC, id DESC` ordering, nullable
policy, revocation provenance, rotation series/predecessor identity and shared
redemption counts are unchanged. Expired/revoked invitations are still history
rows for an active administrator; expired requester sessions instead receive 401.
The GET returns neither raw token nor token hash and never creates, rotates,
revokes, redeems or replays a write. API/shared helpers, mutations, other reads,
frontend source, schema, dependencies and hosting are unchanged. Existing
same-user read serialization remains. Each ordinary success adds one session
query; no measured speedup is claimed.

## Regression coverage

Fourteen new PostgreSQL/API cases ([focused result](focused-tests.txt)) use a
small constrained synthetic fixture, with no production timing/failure hooks:

- Natural expiry at exact membership, final metadata, redemption-count and
  empty-list waits. Eight-second setup windows prove observed blocking and an
  active session before waiting for unchanged natural expiry.
- Revocation/credential replacement after extraction but before transactional
  authority; later revocation and credential writers blocked behind an authorized
  reader; current membership demotion/removal returns 403 under `READ COMMITTED`.
- All four tournament roles, other-tournament admin and global-admin non-bypass,
  missing/invalid resources, session-before-resource ordering, exact error/cache
  shape and adjacent issue authentication behavior.
- A registration followed by rotation creates genuine shared series usage. Full
  JSON parity covers active successor, revoked predecessor, an older revoked row
  and an expired row. Independent assertions check 1/1 versus 0/0 series counts,
  predecessor identity, revocation actor fields, nullable limits and equal-time
  UUID ordering. Exact 12-key metadata assertions exclude raw/hash token fields;
  neither one-time raw secret appears in the response.
- Twelve concurrently submitted valid reads return complete identical payloads.
  The existing exclusive user locks intentionally serialize same-user requests;
  this is correctness evidence, not a throughput benchmark.
- Cancellation releases exclusive authority; a later revocation and fresh read
  succeed. Synthetic `40001` and `XX000` views use a sequence counter to prove one
  attempt, generic 500 and no database detail/payload escape.
- Snapshots compare invitation metadata, series/redemption rows, safe user facts,
  memberships, entrants and scores. Subscriptions assert no read SSE. Intentionally
  prepared session expiry or credential changes stay outside business snapshots.

Independent read-only design and final source/test review found no actionable
issues. Review executed no tests or database commands; runtime evidence belongs
to the primary agent.

## Validation

| Check | Result |
| --- | --- |
| Original final-metadata regression | Expected failure: GET returned 200 after expiry |
| Focused PostgreSQL/API suite | 14 passed |
| Formatting | Passed |
| Workspace tests | 261 passed, one existing ignored benchmark |
| Strict all-target/all-feature Clippy | Passed; three unchanged vendored SQLx warnings |
| API binaries | Built |
| Full PostgreSQL 17 Alpine suite | 914 passed, four existing ignored; exit 0 |
| Full migrate/seed | Passed; 39 migrations current, seed completed |
| Frontend unit tests | 147 files, 1,133 tests passed |
| Frontend typecheck / lint / build | Passed |
| Local Chrome | 320, 390 and 1280 px passed |
| Read-only design/source/test review | No actionable findings |

The [validation summary](validation-results.txt) records complete commands and
results. Database-feature totals include ordinary tests; the counts are not
additive. Four existing ignored cases are the projection benchmark and three
PostgreSQL profiling probes. No test was newly skipped.

Frontend source and schemas are unchanged; the full frontend ladder still ran.
The disposable PostgreSQL is 17.10 Alpine/musl with `en_US.utf8`. Existing test
collation portability remains separately queued; no collation or unrelated
contract changed.

## Browser acceptance

The [probe](browser-probe.mjs) prepares synthetic invitation history through
existing authorized onboarding, issue, registration and revocation flows before
observing reads. The [results](browser-results.json) cover three widths:

- Actual metadata 200s keep exact `no-store`; the real `listInvitations` decoder
  accepts all 12 metadata fields. Raw responses contain no token/hash keys.
  The page shows active, exhausted and revoked history with the real usage count.
- Loading shows no list. Injected no-store 500 shows the existing error, and
  **Prøv igjen** restores the populated list through reads. Empty injection shows
  the no-invitations message; initial 403 shows the error without a list.
- Long tournament/creator names and populated/empty states have no horizontal
  page overflow. No one-time link is exposed by metadata loading.
- Actual logout makes the GET return 401 with `no-store` and the page redirects
  to login. No invitation mutations, page errors or unexpected network failures
  occurred during the read observations.

Visually inspected viewport captures:
[error at 320 px](metadata-error-320.png),
[populated metadata at 390 px](metadata-populated-390.png),
[empty metadata at 1280 px](metadata-empty-1280.png).
These cover selected viewports, not every scrolled control, physical-device
geometry or native zoom. The expired-invitation case is proved by PostgreSQL;
browser populated history uses active, exhausted and revoked rows. Error/empty/
denied injections keep the surrounding synthetic fixture real.

## Limits and remaining work

This closes this GET only. One P2 route remains in the historical inventory:
user-only final-visibility metadata. Three P3 course GETs remain. The historical
A1f manifest/source anchors keep their original revision; those remaining
findings have no new race reproduction here. Legacy write expiry is separately
queued and invitation mutations received no new assessment or repair.

Browser fixtures are synthetic local data. Error/empty/denied UI responses are
explicit injections; PostgreSQL regressions separately prove natural expiry and
failure mapping. No physical-device, load-capacity or public-host readiness claim
is made. Broader findings remain in the
[original review](../application-review-2026-10-14/README.md).
