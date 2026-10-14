# Application logic, performance and security review — 2026-10-14

Reviewed revision: `a2554f17c0534de987798cefc80fdc7eff219947`.
Follow-up: the [private leaderboard repair](../leaderboard-session-2026-10-14/README.md)
addresses A1's four leaderboard endpoints, and the
[round lifecycle repair](../round-lifecycle-session-2026-10-14/README.md) addresses
A2's open/complete/lock expiry gap. The
[identity ordering repair](../identity-ordering-2026-10-14/README.md) addresses A3's
same-document cookie/continuation ordering. The
[tournament read repair](../tournament-read-session-2026-10-14/README.md) addresses
A1b's list, own-membership list and detail routes. The
[roster and round read repair](../roster-round-session-2026-10-14/README.md)
addresses A1c's roster and round list/detail routes. Remaining sibling private
read/write families and the other findings stay queued. The assessment below describes the reviewed revision.

This is a documentation-only assessment of current application source, database
contracts, browser state, production configuration and dependency advisories.
No application, dependency, migration or hosted configuration was changed.

## Assessment

No P0/critical or P1/high application finding was confirmed. Three P2/medium
session defects were reproduced, and four further P2 findings concern recovery,
dependencies and capacity. One P3/low query-scaling issue is source-confirmed.
The reviewed sporting rules are internally consistent, but the application
cannot receive an unconditional security or optimization sign-off while these
findings and deployment acceptance limits remain.

Severity reflects this application's exposure, rather than copying scanner
labels: P0 means immediate critical compromise/data loss; P1 means a severe,
readily reachable production failure; P2 means a material defect or actionable
conditional risk; P3 means lower-impact inefficiency or hardening. Evidence
strength is stated separately. Remediation is proposed, not implemented.

| ID | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| A1 | P2 medium | In-flight private reads survive completed session revocation | PostgreSQL/API reproduction |
| A2 | P2 medium | Round opening can commit after session expiry | PostgreSQL/API reproduction |
| A3 | P2 medium | Departed onboarding can replace a newer login | Chrome reproduction |
| A4 | P2 medium | Active Rust TLS dependency has a published advisory | Fresh audit and normal dependency tree |
| A5 | P2 medium | Restore silently accepts an absent checksum sidecar | Controlled script reproduction |
| A6 | P2 medium | Password work limits execution but leaves admission waiting uncapped | Source-supported capacity risk |
| A7 | P2 medium | Development dependencies retain current advisories | Fresh npm audit; production exclusion checked |
| A8 | P3 low | Team listing performs one member query per team | Source-confirmed; latency unmeasured |

## Findings and acceptance criteria

### A1 — P2: revoked sessions can finish private reads

`backend/src/api/leaderboards.rs:42-64,102-106` discards the extracted session ID
and passes only the user ID. The read transactions in
`backend/src/repositories/leaderboards/mod.rs:86-110` and
`leaderboards/response.rs:25-50` lock membership without locking or checking the
session/user. `tournament_authorization.rs:100-117` cannot check revocation from
its user-only input. Similar user-only setup reads exist for tournaments, rounds,
rosters, pairings and teams; those sibling endpoints were source-reviewed, not
individually race-tested.

Reproduction: hold the synthetic admin's membership `FOR UPDATE`; start a round
net-leaderboard GET; observe its membership `FOR SHARE` lock wait; revoke its
session in another connection and commit; release membership unchanged. The
waiting GET returned **200**, with the same **2,169-byte populated private
response** as the baseline. A fresh GET returned **401**. See
[the captured result](private-read-race-populated.json). The defect affects an
already authenticated in-flight request; it does not let a revoked token start
arbitrary new requests.

First bounded fix: carry session identity into round/tournament leaderboard
transactions, authorize session/user and exact membership together, retain locks
through assembly, and check natural expiry before completion. Define lock order
and deliberate handling of repeatable-read serialization failures. Do not weaken
final-round visibility, membership checks or result snapshot consistency.
Acceptance: contention tests for revocation and credential-generation replacement
before transactional authorization, expiry during waits, ordinary valid reads,
nonmembers and privacy. Revocation after the read acquires its authority locks
may serialize behind that read; no claim of retracting an already sent response
is intended. Follow with a separately bounded pass over the named sibling reads.

### A2 — P2: lifecycle authorization expires before commit

`backend/src/repositories/tournament_authorization.rs:133-148` checks session
validity before waiting for the admin membership lock.
`round_lifecycle/mod.rs:78-81` and `round_completion.rs:216-219` commit afterward
without a final active-session check. Session locks prevent concurrent revocation
from overtaking the transaction, but do not stop database wall-clock expiry.

Reproduction: start the synthetic seeded tournament normally, set the test
session to expire in three seconds, hold its membership row, start round opening,
observe the membership wait, wait until the database reports expiry, then release
the unchanged membership. Opening returned **200**, persisted status **open** and
**eight handicap snapshots**. See [the result](expiry-race.json). Completion/lock
share the source pattern but were not separately reproduced. Legacy authorized
team/round creation also warrants a follow-up inventory.

Bounded fix: recheck active session with database wall-clock time immediately
before commit in open/complete/lock, rolling back all status/snapshot effects and
emitting no SSE on rejection. Acceptance: an expiry-after-membership-wait test for
each transition, plus retained lifecycle, snapshot and locking regressions. This
does not promise to freeze time during COMMIT itself.

### A3 — P2: late account creation replaces newer authentication

`frontend/src/features/onboarding/useOnboardingSubmission.ts:46-57` publishes the
returned session and private cache without mounted-lifetime or canonical-identity
checks. Invitation registration has the same continuation at
`features/invitations/JoinExperience.tsx:104-108`.
`features/auth/AuthProvider.tsx:31-43` provides no shared coordination of concurrent
identity-changing operations.

Chrome reproduction: enter anonymous tournament creation; submit; allow the real
API to commit while delaying browser delivery; navigate Back in the SPA; log in
as an existing synthetic account; deliver the departed creation response. The
visible account and browser cookie both changed to the original creator. There
were no page exceptions. The scenario reproduced at 390px and 1280px; see
[the mobile result](onboarding-race.json) and
[the desktop result](onboarding-race-desktop.json). Invitation registration is
source-confirmed only. Diagnostic routing delayed a real response; no malicious
payload or production account was involved.

Bounded fix: coordinate identity-changing HTTP operations across route lifetimes,
fence publication by operation/lifetime/canonical identity, and reconcile the
authoritative cookie session. Ignoring a React callback alone is insufficient:
these HTTP responses set cookies. Acceptance: reverse-response-order browser
tests for anonymous creation and invitation registration, navigation, a newer
login/logout and same-user session renewal. Assert cookie/canonical identity,
cache ownership and navigation, and preserve existing immutable write recovery.

### A4 — P2: reachable Rust TLS advisory

`Cargo.lock:1561-1562` pins `rustls 0.23.43`. Fresh RustSec reports
[RUSTSEC-2026-0285](https://rustsec.org/advisories/RUSTSEC-2026-0285.html), patched
in `>=0.23.45`. `cargo tree --edges normal -i rustls` confirms normal paths through
Reqwest and SQLx. This concerns accepting TLS handshake messages at an incorrect
encryption level. The authenticated transcript remains intact; the upstream
advisory does **not** establish an authentication bypass or arbitrary MITM.
The service's public TLS termination is Caddy, while Rustls is a client-side
dependency here. Actual hostile-peer exploitation was not attempted.

Bounded fix: update the minimum compatible locked dependency, preserve the local
SQLx transaction-cancellation patch, rerun current audit and backend/PostgreSQL
checks, and exercise provider TLS plus the supported database TLS configuration.
The fresh Rust audit also flags three Quinn advisories and RSA, plus a yanked
ChaCha20 version. Those packages have no normal dependency-tree path in this
build; do not describe them as reachable application vulnerabilities. Recheck
features/targets when dependencies change.

### A5 — P2: checksum requirement is optional in the restore script

`scripts/restore-production.sh:26-32` checks the sidecar only when it exists,
although `docs/Documentation.md` describes a valid checksum as required.
A fake-Docker harness, which touched no database, verified that a missing sidecar
returns exit **0** and reaches `pg_restore`; a mismatched existing sidecar returns
exit **1** without restoration. See [the result](restore-probe.json).

An archive copied without its sidecar bypasses the documented transfer-integrity
check. PostgreSQL's archive parsing, single-transaction restore and empty-schema
guard remain useful safeguards, but are not verification against the saved dump
digest. A checksum is also not proof of authenticity against an attacker who can
replace both files.

Bounded fix: require a readable checksum sidecar, verify it binds the requested
archive, and fail before stopping services or contacting restoration tools.
Acceptance: missing, empty, mismatched, incorrectly targeted and valid manifests;
preserve the empty-target and transactional-failure tests against disposable
PostgreSQL. No hosted restore is authorized by this assessment.

### A6 — P2: password-work admission has no explicit queue bound

`backend/src/auth/password.rs:9-11,46-55` limits active blocking Argon2 work to
four tasks, but every additional caller awaits `acquire_owned()` without an
admission cap or deadline. Pending closures retain their inputs. Unknown-user
login deliberately performs dummy verification (`api/auth.rs:119-133`) and shares
capacity with registration/recovery.

Production per-client/resource rate limits constrain arrival rates, and the
shared bucket store itself is bounded. They do not define an acceptable number
or duration of pending password operations across admitted clients. Sustained
multi-client arrival above service capacity can grow waiting work and delay
legitimate account operations. This is a source-supported capacity risk, **not a
measured production denial of service**; no public load test was performed.

Bounded fix: retain four active, cancellation-safe workers while bounding
active-plus-waiting admission and promptly rejecting overload through the stable
generic error contract. Acceptance: saturation, cancellation, permit release,
dummy-user parity, no secrets in responses/logs and legitimate work recovery.
Keep Argon2 parameters and credential-generation checks intact.

### A7 — P2: development-tool advisories need a separate upgrade

Fresh `npm audit --package-lock-only --ignore-scripts` reports six affected
packages: `@vitest/mocker`, `vitest`, `tinypool`, `brace-expansion`, `js-yaml` and
`source-map-js`. Scanner labels are two critical, three high and one moderate;
Vitest inherits transitive findings, so this is not six independent exploits.
Versions, advisory URLs and counts are captured in
[the audit summary](dependency-audit-summary.json).

All affected installed paths are marked development-only in the lockfile. A
separate fresh `npm audit --omit=dev --package-lock-only --ignore-scripts` reports
zero production dependency findings. This is an advisory scan, not a proof that
all production dependencies are defect-free. The
production frontend image copies only built static files into Caddy. The
[Tinypool advisory](https://github.com/tinylibs/tinypool/security/advisories/GHSA-85c8-ppgw-ccpr)
requires a prototype-pollution precondition and affected worker options; the
[Vitest advisory](https://github.com/vitest-dev/vitest/security/advisories/GHSA-82fw-gwwq-j7x9)
concerns particular development mock-server registrations. Neither exploit chain
was found in current application source. Build/test environments still deserve
maintenance, especially when processing untrusted files or exposing dev servers.

Bounded fix: select compatible patched tool versions deliberately, split any
major Vitest migration from simple transitive updates, and rerun the complete
frontend ladder and both production/development audits. Do not run a blind
force-update or treat the scanner's critical label as production remote execution.

### A8 — P3: team member queries grow with team count

`backend/src/repositories/teams.rs:46-51` loads teams once and fetches members
sequentially for every team. The authenticated route remains exposed at
`api/teams.rs:20-28`. Member-loading work is **1+N SELECTs**, in addition to
authorization and transaction overhead. No current production frontend caller
was found, and neither a latency regression nor a production budget breach was
measured.

Bounded fix after correctness work: fetch round memberships once, group by team,
and preserve team/member ordering, empty teams and transactional authority.
Acceptance: constant member-loading query count for one and many teams, exact
response parity and representative end-to-end measurement. Do not weaken session
checks or shorten locks without separate evidence.

## Performance conclusions and remaining measurements

Recent R1–R3 predeployment fixes are present and are not reopened here. Scoped
Fantasy round projection, route splitting and batched sporting leaderboard loads
remain in place. The fresh production build reports main JS **467.50 kB / 144.40
kB gzip**, lazy Fantasy **62.52 / 19.94 kB**, and tournament management **141.31 /
39.36 kB**. These are build sizes, not complete route transfers or load timings.

Two future measurement candidates, rather than additional confirmed defects:
Fantasy still loads/locks all tournament facts for scoped result reads, and its
batch actions can overlap broad invalidation/SSE reads; the two private offline
runtimes also poll every two seconds even with empty queues
(`scoring/offline/runtime.ts:31-44`, `matchPlay/offline/runtime.ts:19-30`). Measure
completed request counts, query/lock time, idle React commits and foreground/
background CPU before selecting a bounded optimization. Preserve authority,
freshness, exact replay and read-after-write reconciliation.

## Validation and limits

All data was synthetic. PostgreSQL 17 ran in the new disposable rootless Podman
container `golf-assessment-20261014-pg`, bound to local port 55449. The local API
and Vite served only the review fixture. No hosted service was administered.

| Check | Fresh result |
| --- | --- |
| Rust formatting | Passed |
| Backend all-target tests without database feature | 258 passed; 1 manual benchmark ignored |
| Clippy, all targets/features, warnings denied | Passed; 3 existing vendored SQLx warnings |
| Backend binary build | Passed |
| Full PostgreSQL-feature test ladder | 736 passed; 4 manual performance probes ignored |
| Clean forward migration and development seed | Passed, schema 39 |
| Frontend tests | 1,113 passed across 143 files |
| Frontend typecheck, lint and production build | Passed |
| Revoked-session populated read race | Reproduced A1; baseline and in-flight 200, subsequent 401 |
| Expired-session opening race | Reproduced A2; 200 and persisted snapshots |
| Chrome onboarding response race | Reproduced A3 at mobile 390px and desktop 1280px |
| Restore fake-tool harness and shell syntax | A5 reproduced; syntax passed |
| RustSec and npm audits | Completed, findings retained above; production-only npm audit reports 0 |
| Independent read-only security/domain/frontend review | Consolidated into this report |

The 736 total includes the non-database tests; totals are not additive. The four
ignored cases are the explicit projection benchmark and three existing
PostgreSQL profiling probes. They were not run in this assessment. No production
load benchmark, complete browser regression rerun, physical-device run or
public-host TLS/DNS/container-image scan was performed. Only the focused A3
browser scenario was newly exercised. Existing physical iPhone/Android, native
zoom and hosting acceptance limits remain.

Initial sandbox runs could not create mock HTTP sockets, reach npm, write the
default RustSec cache, or access Podman's runtime directory. Checks were rerun
with authorized local/network access and a temporary advisory database; these
were environment restrictions, not application failures. The browser probe's
first two diagnostic attempts had incorrect accessible-label selectors and a
missing base URL; corrected mobile and desktop runs completed.

Commands: the affected ladders in `docs/AGENT_WORKFLOW.md`, plus `cargo audit --db
/tmp/golf-review-20261014-advisory-db --json`, `cargo tree --edges normal -i
<package>`, `npm audit --package-lock-only --ignore-scripts --json`, `sh -n
scripts/backup-production.sh scripts/restore-production.sh`, and the isolated
Python/Playwright race probes described above. Local logs/probe sources use
`/tmp/golf-review-20261014-*` and `/tmp/golf-review-*-race.*`; durable sanitized
outcomes are linked in this report. Advisory database commit:
`6aed84668c23d7f535077a9fe918bf1fa0f6a5bc` (updated 2026-10-13); advisories and
package reachability must be rechecked when implementing upgrades.

## Plan

The review selected A1's leaderboard boundary first; its
[repair evidence](../leaderboard-session-2026-10-14/README.md) is now available.
[PLANS.md](../../PLANS.md) owns the current next candidate and remaining work.
Address A2 and A3 as separate correctness steps, followed by A4, A5, A6 and A7
in separately validated changes. A8 and further performance measurements follow
the correctness/security work. Equal P2 ratings do not imply identical scope or
identical exploitability. Each implementation step requires its own authorization.
