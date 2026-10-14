# Session-bound round readiness reads — 2026-10-14

Scope: A1e, P2 follow-up to the [application review](../application-review-2026-10-14/README.md).
Only `GET /api/rounds/{id}/pairing-validation` and
`GET /api/rounds/{id}/completion-validation` change their authority boundary.

## Original failure and repair

The [new regression on the original implementation](baseline-regression.txt)
held session and exact tournament-membership rows, started both GETs, observed a
real PostgreSQL wait after the nonlocking HTTP session lookup, revoked the
session and committed. Both returned **200** after committed revocation. The
same test now returns **401 unauthenticated**, with `private, no-store`.
No private response payload or credentials are retained in the failure evidence.

The handlers pass the actual session UUID to explicitly named `*_for_session`
repository entries. Both reuse the unchanged `session_read.rs`: begin one
repeatable-read attempt, lock/check active session and user, derive user identity,
require exact membership through the stored round parent, load the complete
readiness response, recheck active session against database wall-clock time,
then commit. Session/user and membership share locks remain through commit.
A revocation or credential replacement that wins authority denies the read;
a later writer waits behind an already authorized reader. Natural expiry during
membership or fact-loading waits denies success. This does not freeze expiry
during COMMIT or retract previously delivered data.

Pairing's final check follows base facts, pure validation and the additional
singles-assignment readiness query. Completion retries `validation_once` in full:
stored round/tournament context, exact role, owner facts, visible-hole query and
redacted projection. A direct or nested `40001` discards the entire attempt and
allows one fresh read; a second conflict returns generic `503 service_unavailable`.
Shared middleware applies `private, no-store` to every status on these two GETs.
Existing authorization/database errors retain their standard mapping.

All fact SQL, deterministic ordering, issue ordering, eligibility, fixed opening
snapshots and hidden-final redaction remain unchanged. Blocked readiness is
still typed HTTP 200. Singles completion-validation retains its existing 404;
its dedicated completion endpoint is outside this step. Unrestricted internal
validation helpers and open/complete/lock mutation bodies are unchanged. Existing
four-ball and Stableford privacy tests now supply their already available real
session to the renamed completion entry. No mutation gains a retry or SSE effect.
No schema, frontend source, dependencies or hosting configuration changed.
Each ordinary successful readiness read adds two session authority queries;
there is no batching or measured performance claim.

## Regression coverage

Sixteen new PostgreSQL/API tests cover:

- Revocation and credential-generation replacement before transactional authority
  reject both reads; later authority writes serialize behind valid readers.
- Membership removal causes a fresh attempt to deny access.
- An admin-to-viewer demotion also changes hidden-final context from explicitly
  released to hidden. The reply must reload both context and role and redact
  readiness, completeness, confirmations and hidden-state issue codes.
- All four tournament roles, global-admin non-bypass, missing/cross-tournament
  resources, invalid UUIDs and unauthenticated cache/error contracts.
- Twelve concurrent reads with full JSON parity against retained internal helpers,
  stable blocker ordering, database digests and no SSE.
- Ready, blocked, empty and singles responses, preserved final snapshot owners,
  all-role hidden-final projections, and ordinary completion/lock states.
- Late loading retains one coherent snapshot; fresh reads observe changed names.
- Cancellation releases session authority and subsequent reads succeed.
- Expiry during membership waits, ordinary late loading, empty results and the
  additional singles-assignment query rejects successful replies.

Expiry fixtures allow eight seconds to reach their controlled wait, assert the
session is still active after observing `pg_blocking_pids`, wait for database
wall-clock expiry, then release the gate. They use real database locks and no
production timing hooks. Configured pairing reads wait on `holes` near the end of
base loading; completion waits on `holes` in visible-hole loading after owner
facts. Tee-less empty pairing reads wait on `flight_memberships` instead; empty
completion still executes the visible-hole query. Singles pairing waits on
`singles_matches`, after base readiness loading.

Independent review caught a P2 test-evidence gap: the first context retry test
set visibility to its existing default true. The corrected fixture explicitly
releases it through the authorized workflow, asserts false, then changes it true
under valid workflow context before demoting membership in the same gated
transaction. Reusing either stale context or stale role now fails the test.
Production retry ownership was already correct. Independent read-only review found no remaining A1e source/test findings.

## Validation

| Check | Result |
| --- | --- |
| Original regression | Failed as expected, both GETs 200 after revocation |
| Repaired original regression | Passed, both private 401 |
| Focused readiness/lifecycle/completion/four-ball/Stableford suites | 73 passed before review fixture tightening |
| Final corrected regression | 30 passed: corrected readiness 16 and unchanged pairing 14 |
| Formatting | Passed after final fixture correction |
| Full backend all-target tests | 261 passed; 1 existing manual benchmark ignored |
| Strict all-feature/target Clippy | Passed after final fixture correction; 3 existing vendored warnings |
| API binary build | Passed |
| Full PostgreSQL-feature ladder | 813 passed; 4 existing manual performance probes ignored |
| Explicit migration and development seed | Passed, schema 39 |
| Local health/readiness | Passed |
| Chrome readiness UI at 320/390/1280px | Passed: both flows at all three widths |
| Independent read-only review | No remaining source/test findings |

Validation uses disposable PostgreSQL container `golf-a1e-20261014-pg` on
loopback port 55449. Initial focused/browser runs used Debian PostgreSQL 17.11;
the final PostgreSQL ladder uses the repository's `postgres:17-alpine` image
(PostgreSQL 17.10, musl), with disposable data on tmpfs. Both databases report
`en_US.utf8`, but their native text order differs. SQLx tests own isolated
databases; browser data used separate `golf_a1e_browser`. No production accounts
or operator infrastructure are used.
The owned API/Vite processes and PostgreSQL container were stopped/removed after validation.

Commands from repository root with disposable `DATABASE_URL`:

```bash
cargo test -p golf-api --features database-tests --test readiness_sessions --test round_lifecycle --test round_completion --test four_ball --test stableford -- --test-threads=2
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build -p golf-api --bins
cargo test --workspace --all-targets --features database-tests -- --test-threads=2
cargo run -p golf-api --bin migrate
cargo run -p golf-api --bin seed
node docs/validation/readiness-session-2026-10-14/browser-probe.mjs
```

Logs are `/tmp/golf-a1e-{baseline,repaired,focused-first,focused,fmt,backend,clippy,build,database,migrate,seed,browser}.log`.
Final correction logs are `/tmp/golf-a1e-{fmt-final,clippy-final,corrected}.log`.
The first full ladder is `/tmp/golf-a1e-database-first.log`; browser rerun diagnostics
are `/tmp/golf-a1e-browser-rerun.log`, with successful results retained here.
Frontend source and JSON shapes are unchanged, so its unit/type/lint/build ladder
is unaffected and not repeated. Existing vendored SQLx warnings remain unchanged.
Physical devices, public hosting and production load remain outside acceptance.
The previously deferred old leaderboard browser assertions and A1 expiry-test
window investigation remain separate, unchanged work. The PostgreSQL total includes ordinary backend tests and is not additive with 261. Four ignored cases are one projection benchmark and three existing PostgreSQL profiling probes. The unchanged old A1 score-loading expiry case passed in this final two-thread run.

## Existing collation-dependent test limit

The first full PostgreSQL run [stopped in an unchanged A1d test](pairing-collation-limit.txt),
`pairing_team_sessions::contracts::legacy_individual_groups_keep_identity_schedule_and_distinct_sorting`
at `backend/tests/pairing_team_sessions/contracts.rs:215`. It assumes
`B-group` precedes `b-group` under the team's existing native `ORDER BY name,id`.
Debian/glibc PostgreSQL 17.11 returns the lowercase name first; the repository's
Alpine/musl PostgreSQL 17.10 returns the uppercase name first. A direct
`VALUES ('b-group'),('B-group') ... ORDER BY v` comparison confirmed both orders.
Sorting with explicit `COLLATE "C"` on Debian also returns uppercase first.

This is a P3 portability limitation of the unchanged test expectation, not
new A1e SQL or an established product defect: the existing native ordering
remains explicit and deterministic within each database collation. Runtime
sorting and A1d tests are deliberately unchanged. The final ladder is rerun on
the image used by `docker-compose.yml` and `deploy/postgres/Dockerfile.production`.
A separate test-only follow-up should compare the native-order contract without
assuming one libc collation. Broader collation support is not claimed here.

## Browser evidence

The [probe](browser-probe.mjs) uses a fresh separately migrated/seeded synthetic
fixture, local API on 3000 in development mode and Vite on 5173. It starts the trip,
names a synthetic second-round team with long text through pairing PUT, then
opens that round once. Those fixture preparation mutations precede UI checks.
Do not run it against retained or production data.

At 320, 390 and 1280px, the existing real management UI exercises pairing readiness
on the draft first round and completion readiness on the open second round.
Both pass delayed loading, populated readiness, injected 503 with an explicit
read-only refresh, injected empty and 403 states, and disabled lifecycle actions
while pending or failed. Draft readiness also passes injected missing-flight
blockers with a long participant name; completion uses a real long team name.
No horizontal overflow occurs. Both actual endpoints return private 200 JSON and
private 401 after logout; management redirects to login. The six width/endpoint
flows emit no lifecycle POSTs, page exceptions or unexpected network failures.
The remaining three JSON entries record per-width logout and error checks.

The first probe run passed the 320px pairing flow, then its synthetic empty
completion payload was correctly rejected by the existing runtime decoder for
omitting `round_not_completed`. The corrected injection includes the complete
existing blocker set; the rerun passes all six flows. No frontend change was
needed. Empty, blocked, error and denied UI responses are injected deliberately;
PostgreSQL/API tests cover the real authority races and natural empty responses.

Representative screenshots are retained here after visual inspection.
- [Mobile completion with a long team name](mobile-completion.png)
- [Mobile blocked pairing with long content](mobile-pairing-blocked.png)
- [Desktop readiness error](desktop-error.png)

Text wraps and error refresh remains usable. These component screenshots capture
the existing fixed navigation in the viewport; full element captures do not
simulate physical-device scrolling. Other screenshots remain in
`/tmp/golf-a1e-{pairing,completion}-{loading,populated,error,empty}-{320,390,1280}.png`.

## Remaining scope

This closes only these two A1e GET boundaries. Remaining private read authority
inventory, legacy-write expiry inventory, dependency updates, restore checksums,
password-work admission and team batching retain their queued severity and
separate approval boundaries in [PLANS.md](../../PLANS.md).
