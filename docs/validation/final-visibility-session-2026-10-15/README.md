# Session-bound final-visibility metadata GET — 2026-10-15

A1m closes the P2 [final-visibility metadata boundary](../private-read-inventory-2026-10-14/README.md#visibility):
`GET /api/tournaments/{tournament}/final-round-visibility`.

## Original failure and repair

Baseline production source: `81830935463f825f2d88b176b256466dfcfda4d1`, with new
regressions applied before the production repair. The [original result](baseline.txt)
records **200 after natural session expiry, revocation and credential replacement**
during controlled database waits. The expiry case observes an actual membership
wait while the session is active, waits for database wall-clock expiry, then
releases the unchanged membership lock. Revocation/credential cases retain a
session-row gate plus membership gate, observe the post-extractor wait, commit
authority loss and release the gate. No metadata or session secrets are retained
in the baseline evidence.

The API now passes session identity to a new `get_for_session` repository entry.
Each complete attempt begins a repeatable-read transaction, loads stored
visibility metadata, derives the user from the active session with shared
session/user locks, locks exact-admin membership and checks active-session
validity again against `clock_timestamp()` before commit. Stored-resource lookup
still precedes repository authorization: missing tournament remains 404 even
for a missing repository session; the HTTP extractor still rejects missing or
inactive sessions before the repository is called. Invalid HTTP UUIDs retain
Axum's existing 400 rejection.

The final check follows the metadata and membership waits. It does not freeze
time through commit, JSON serialization or network delivery, or retract an
already delivered response. Revocation/credential replacement that wins the
initial authority race returns 401; later writers serialize behind an authorized
read. A membership change that invalidates the snapshot discards the complete
attempt. Existing `session_read` ownership permits one fresh whole read on
`40001`; a second conflict returns generic private 503. Nonserialization database
errors remain generic 500 without retry. No old user, role, metadata or version
survives a retry.

The GET method's response middleware applies exact `private, no-store` to all
statuses, including extractor rejections. PATCH is added after that method layer
and retains its independent error/cache contract: private success, existing
`no-store` on unauthenticated rejection and headerless stale 409. Its versioned,
CSRF-protected, database-guarded write path and post-commit notification behavior
are unchanged. The user-only `get_for_admin` helper remains unchanged for existing
internal callers; it is no longer this HTTP GET's authority owner.

The same three fields, boolean policy and full timestamp version are returned.
No score/result projection, sporting rule, handicap snapshot, schema, frontend
source, dependency or hosting configuration changed. Each ordinary successful
GET adds two active-session queries; there is no measured speedup claim, new
query loop, mutation retry or read side effect.

## Regression coverage

[Fourteen new PostgreSQL/API cases](focused-tests.txt), plus the four existing
visibility cases, pass with constrained synthetic fixtures:

- Natural expiry during initial metadata lookup and during exact membership
  locking. Both observe a real wait while active, then unchanged natural expiry.
- Post-extractor revocation and credential replacement return 401; later writers
  wait behind a valid reader, then make subsequent reads return 401.
- All four target roles, global-admin non-bypass, other-tournament admin isolation,
  missing sessions/resources, invalid IDs and resource-before-repository-authority
  precedence. GET cache headers cover 200/400/401/403/404/500/503.
- Twelve concurrent reads in each hidden/released/hidden state return exact
  three-field JSON equal to the established helper, preserving the complete
  timestamp token. Authorized setup changes occur before read observations.
- A real concurrent membership update invalidates the first repeatable-read
  snapshot while visibility changes. The successful retry returns the new flag
  and complete new version. Demotion/removal instead returns 403 after retry.
- Disposable failure views and a sequence prove exactly two attempts for
  persistent `40001`, one for `XX000`, generic error bodies and no metadata or
  database-detail escape. No production test hooks were added.
- Cancellation releases retained authority, permitting later revocation and a
  fresh successful read. Business-state snapshots and event subscriptions cover
  read-only behavior. The PATCH compatibility case separately proves no-op,
  changed-write notification, stale-token behavior and exact cache headers.

Independent read-only source/test review found no remaining actionable issues.
It identified the test's original incorrect assumption that PATCH 401 was
headerless; that assertion was corrected to the existing `no-store` contract and
the focused suite reran successfully. The reviewer ran no tests or database work.

## Validation

| Check | Result |
| --- | --- |
| Original regression | Expected failures: GET returned 200 after expiry, revocation and credential replacement |
| Focused PostgreSQL/API tests | 14 new and four existing passed |
| Formatting | Passed |
| Workspace tests | 261 passed, one existing ignored benchmark |
| Strict all-target/all-feature Clippy | Passed; three unchanged vendored SQLx warnings |
| API binaries | Built |
| Full PostgreSQL 17 Alpine suite | 928 passed, four existing ignored; exit 0, 764 seconds |
| Full migrate/seed | Passed; 39/39 migrations current, seed completed |
| Frontend unit tests | 147 files, 1,133 tests passed |
| Frontend typecheck / lint / build | Passed |
| Local Chrome | 320, 390 and 1280 px passed |
| Independent read-only source/test/documentation review | No remaining actionable findings |

The [validation summary](validation-results.txt) records commands and results.
Database-feature counts include ordinary tests and are not additive. Four
existing ignored cases are the Fantasy projection benchmark and three
PostgreSQL profiling probes; no check was newly skipped. No production source
or test changed after the successful complete ladders began.

The disposable database was PostgreSQL 17.10 Alpine/musl with `en_US.utf8`.
Existing collation portability remains separately queued. Production source
contains 90 API and 185 repository nonblank/noncomment lines, below the 400-line
limit. The owned local API/Vite services and disposable database were stopped
and removed after validation. No hosted environment was accessed.

## Browser acceptance

The retained [probe](browser-probe.mjs) creates a synthetic account/tournament,
configures a manual 18-hole final through the existing authorized workflow and
prepares hidden/released state before observing reads. Its
[results](browser-results.json) pass at 320, 390 and 1280 px:

- Actual 200 responses have exact `private, no-store`. The production
  `finalRoundVisibilityApi.get` decoder preserves the entire three-field JSON,
  including the complete version string. Both hidden and released flags render
  correctly without changing them through the control.
- A held real GET shows the loading message without a switch. Explicit injected
  500 and 503 responses show the existing error; **Prøv igjen** restores current
  server state using reads. Injected initial 403/404 shows no switch. Actual
  missing/invalid resources return private 404/400. A singleton metadata resource
  has no empty-collection state; no fabricated empty DTO was accepted.
- Long final/tournament names cause no horizontal page overflow. The visibility
  switch label remains at least 44 CSS pixels high. Actual logout produces a
  private 401 and protected navigation redirects to login.
- No visibility mutation, page error or unexpected network failure occurred
  during observations. The one fixture-release PATCH at 390 px and course
  configuration occurred before observers were installed.

Visually inspected captures: [error at 320 px](visibility-error-320.png),
[released at 390 px](visibility-released-390.png) and
[hidden at 1280 px](visibility-hidden-1280.png). These are selected viewport
captures, not exhaustive scrolled-control, physical-device or native-zoom claims.
The probe's first setup attempts exceeded the username limit and then omitted
required course configuration; only the corrected fixture's complete successful
run is retained as acceptance. These were harness setup errors, not product
findings. The retained probe now bounds the initial-request wait.

## Limits and remaining work

This closes this GET only. The original A1f inventory now has **zero remaining P2
routes and three P3 course GETs**; its manifest, classifications and source anchors
remain pinned to its original assessment revision. That count does not close the
other P2 findings in the [original review](../application-review-2026-10-14/README.md).
Legacy team/round write expiry remains separately unassessed. No other read or
mutation family was repaired or newly accepted here.

The regression harness is local and synthetic. Browser error/denial responses
are explicit UI injections; real PostgreSQL races separately establish session
and retry behavior. Physical-device, native zoom, load capacity and public-host
acceptance remain outside this step. Hosting belongs to the operator.
