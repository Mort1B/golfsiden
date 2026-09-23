# Local private/public result-projection security assessment

Date: 2026-09-23. Assessed revision: `7fdb2e0`.

**One confirmed finding: SHARE-1 (P2/medium), late session expiry during public
result-link issuance.** No cross-tournament access or public field leak was
demonstrated. This is a bounded assessment, not proof that other defects do not
exist. Application source, tests, migrations and dependencies are unchanged;
the diagnostic programs retained here are assessment artifacts only.

The owner authorized source inspection and disposable local validation with
synthetic accounts. No production, external targets or real credentials were
accessed. Independent read-only reviews covered public capabilities/projections
and private results/membership boundaries; the public reviewer also checked the
reproducer and confirmed SHARE-1. The primary reviewed transport/frontend state,
ran validation and inspected representative Chrome screenshots.

## Confirmed finding: SHARE-1

**A request authorized while its session is active can commit a usable public
result link after that session expires during a later audit write wait.**

Source: [management repository](../../../backend/src/repositories/result_sharing/management.rs),
lines 68–82, authorizes before grant writes and commits immediately afterward.
The [schema guard and audit triggers](../../../migrations/0026_public_result_sharing.sql),
lines 52–78, check the current session in a BEFORE trigger, then insert the audit
row in an AFTER trigger. The latter write can block after authorization passed.
There is no final active-session check after that wait and before commit.

### Controlled reproduction and impact

The retained [probe](expiry-probe.rs) invokes the unchanged Rust API router using
Tower and real disposable PostgreSQL. It creates a synthetic user with exact
tournament administrator membership, a draft tournament and one empty round.
It submits the normal issue request with a valid session and CSRF token.

1. A separate transaction holds `LOCK TABLE tournament_result_share_audits IN SHARE MODE`.
2. The probe observes the issue request waiting on that exact table's
   `RowExclusiveLock`, identifies the expected blocker, and checks that the
   session is still active at this point.
3. It waits until PostgreSQL wall-clock time confirms session expiry, then
   releases the audit-table lock.
4. The API returns **201**. One live grant and one audit persist, and a matching
   tournament invalidation event is emitted. The same session now returns **401**,
   while an anonymous read using the returned capability returns **200**.
5. A control with the same wait but an unexpired session succeeds with **201**,
   a live grant/audit/event, session **200**, and anonymous **200**.

Both cases completed with successful assertions. Sanitized output is retained
in [results.txt](results.txt). The test uses a three-second fixture lifetime to
reach natural expiry; it does not imply a three-second production session policy.

Impact is a capability-issuing mutation completing outside the session's valid
lifetime. The capability is usable under the ordinary public projection rules.
The empty draft fixture proves usability, **not disclosure of populated scores**.
The table lock is a deliberately imposed, maintenance-style database wait; no
evidence shows that an anonymous caller can cause it. This is not an
expired-at-entry, CSRF, cross-tournament, logout or role-downgrade bypass. Earlier
grant-row waits remain covered by existing authorization and trigger checks.
These prerequisites constrain the proposed P2/medium severity.

### Recommended separate repair

Recheck the active session after all grant/audit writes and immediately before
commit. Preserve the existing lock order, exact tournament authority, expected
grant intent and atomic grant/audit behavior. Add controlled late-write expiry
tests for issue, replacement and revoke, plus valid-session controls. Detected
expiry should return 401, roll back all changes and emit no event. Do not claim
atomic expiry at the physical COMMIT instant.

Revoke has the same missing final-check pattern (management lines 94–103), but
that operation and replacement were **not independently reproduced** here.
No repair was implemented during this assessment.

## Evidence-backed controls

| Boundary | Source and exercised evidence |
| --- | --- |
| Private tournament access | `api/leaderboards.rs:37` derives identity from the session. `repositories/tournament_authorization.rs:100` locks exact membership through repeatable-read assembly. Private workspace and cross-tournament tests reject missing/wrong membership; global role does not substitute for tournament membership. |
| Private card/history scope | `repositories/scorecards.rs:104` rechecks session and membership; `scorecards/handicaps.rs:9` binds historical owners to the exact round. Actor-free scorecard projections omit submitter, revision and timestamp fields. Match reads authorize before loading exact-round history. Relevant private card and match-overall tests passed. |
| Private live events | `api/live.rs:33` scopes the stream to the tournament and reauthorizes before constant invalidation payloads. Private live-read tests passed; the public frontend does not subscribe to private SSE. |
| Public capability scope/lifetime | `repositories/result_sharing/public.rs:28` locks the grant through snapshot loading, resolves the tournament from the grant and checks the hash after acquiring the grant lock, and expiry both after that lock and after loading facts. Tests cover revocation/rotation serialization, grant/fact-wait expiry, stale intent and simultaneous replacement. |
| Public projection allowlist | `domain/result_sharing/projection.rs:79` builds a fixed summary DTO. It excludes global player/owner/account IDs, team/roster details, per-hole scores and contributions. Explicit public audience uses nonadministrator visibility even with administrator cookies. Allowlist, cookie-independence and hidden-final gross/net tests passed. |
| Format/embargo boundaries | Match-only overall results are unavailable and cannot issue a result link. Match notes/events are not loaded into public summaries. Stableford/four-ball equivalents and mixed match/overall behavior passed their focused tests; six embargo projection tests cover hidden final facts and fail-closed corruption handling. |
| Transport | Management requires session/CSRF. Public POST accepts its token in a strict, size-limited body and does not extract a session. Responses apply no-store/no-referrer/noindex and generic unavailable errors. Frontend public requests omit credentials and referrers and strictly decode fields. |
| Browser state | Public visits use separate query ownership, purge on return/offline/metric changes and clear on terminal errors/expiry. Public standings have no private drilldown links. Real Chrome issue/copy/replace/revoke and mocked lifecycle/UI states passed. Private denial tests check cache clearing and stale-response cancellation. |

Public result capabilities intentionally remain in the reusable URL fragment;
this differs from the single-use recovery form. The assessment does not claim
that result-link fragments are removed from browser history. Full browser and
offline persistence review remains separate.

## Unverified concerns and coverage gaps

- Ordinary leaderboard and actor-free scorecard suites do not deliberately hold
  membership deletion/demotion races. Source holds exact membership locks;
  existing match-listing tests exercise related locking, but are not a substitute
  for these specific result paths.
- Leaderboard repositories receive extracted user IDs; standard card reads check
  the session before assembly rather than after all later waits. Consistency of
  in-flight reads across logout/expiry needs an explicit read-boundary decision
  and reproducer. No unauthorized disclosure under that schedule was proved.
- Equivalent-format public tests check nested field values and selected
  exclusions, not the complete exact top-level/row allowlist used in legacy
  stroke coverage. Existing-grant reads after draft conversion to match-only
  are source-supported to fail closed but lack a dedicated scenario here.
- SHARE-1 replacement/revoke paths remain unprobed as noted above. Broader browser
  persistence/offline storage, production proxy/database privileges, recovery
  operations and dependency advisories remain outside this step.

## Validation

| Check | Result |
| --- | --- |
| PostgreSQL integration tests | 45 passed: main result/private/leaderboard/scope suites 34, Stableford public 2, match overall 3, embargo projections 6 |
| Result-sharing Rust unit test | 1 passed; unrelated 212 filtered out |
| Frontend result-sharing/private-result tests | 35 passed in 5 files |
| Frontend production build/type compilation | Passed |
| Installed Google Chrome 153.0.8010.36 | 2 scenarios passed |
| Browser layouts | 17 states at 320×600, 390×844 and 1280×900; 51 screenshots |
| SHARE-1 diagnostic | Expired case and unexpired control reproduced; assertions passed |

Commands executed against the assessed source:

```sh
cargo test --offline -p golf-api --lib result_sharing
cargo test --offline -p golf-api --features database-tests \
  --test result_sharing --test private_workspace_reads \
  --test private_scorecard_live_reads --test leaderboards --test tournament_scope_isolation
cargo test --offline -p golf-api --features database-tests --test stableford public
cargo test --offline -p golf-api --features database-tests --test singles_match overall
cargo test --offline -p golf-api --features database-tests --test embargo_read_projections

# From frontend/
npm run test -- src/features/resultSharing src/api/resultSharing.test.ts \
  src/api/privateResults.test.ts src/pages/PrivateResults.test.tsx
npm run build
GOLF_RESULT_SHARING_BROWSER=1 ./node_modules/.bin/playwright test \
  --config playwright.lifecycle.config.ts resultSharing.browser.ts \
  resultSharingStates.browser.ts --reporter=line \
  --output=/tmp/golf-results-assessment/browser-results
```

The initial database invocation named a nonexistent `private_results` target and
exited 101 before tests; it was corrected to `leaderboards`. A combined `public`
filter ran two Stableford tests but selected zero singles-match tests. The latter
was corrected to `overall`, which ran three tests; zero matches count as no
coverage. All reported passing runs exited zero, with no ignored tests.

The [real Chrome scenario](../../../frontend/e2e/resultSharing.browser.ts) exercises
admin issue/copy/replace/revoke, anonymous updates, cookie independence and hidden
final scores. The [mocked scenario](../../../frontend/e2e/resultSharingStates.browser.ts)
exercises loading/error/empty/long content, gross/net, polling, return/offline,
same-grant hash ownership and terminal states. Expected negative responses
(401/404/409/503) and specific canceled requests are allowed; unexpected monitored
errors fail the scenarios. Representative screenshots were visually inspected:
[live mobile](anonymous-live-390.png), [long names at 320px](public-longnames-320.png),
[desktop error](public-error-1280.png). Text wraps within the viewport and the
error state removes prior rows. These are not a full design/accessibility audit.

PostgreSQL 17.10 used a cached image with `--pull=never`, tmpfs data and loopback
port 55443. The [API harness](loopback-server.rs) and probe were built in a
temporary Cargo crate with a path dependency on `backend`, the checkout lockfile
and patched SQLx PostgreSQL crate; dependencies were axum 0.8, tokio 1, sqlx 0.8,
uuid 1, chrono 0.4, tower 0.5, serde_json 1 and http-body-util 0.1. Both reject any
database endpoint other than the designated synthetic loopback database. Vite
served the fresh build on 127.0.0.1:5173 and proxied to the API at 127.0.0.1:3000.
The probe was built offline with `CARGO_TARGET_DIR` pointing to this checkout's
`target` directory, then invoked as
`python3 /tmp/golf-results-assessment/run.py /home/morten/Prog/guttasgolfside/golfsiden/target/debug/results-review-expiry`.
The temporary wrapper supplied only synthetic database/development configuration;
its credential files were deleted during cleanup. To repeat, recreate a disposable
loopback database at the guarded endpoint, apply the checkout migrations, build
the retained probe as the `results-review-expiry` binary in that temporary crate,
and inject its synthetic `DATABASE_URL`; never substitute a persistent database.
No course provider was configured. Development throttling was disabled normally;
existing result-sharing tests explicitly exercise route throttling.

Loopback HTTP is not production TLS/proxy/runtime-role validation. Physical
Android Chrome and native 200% zoom remain unverified. Full all-target
backend/Clippy, full frontend test/lint and deployment ladders were not rerun:
this step changes no application implementation. Existing SQLx vendor and Node
color-environment warnings were observed; no external advisory lookup was made.

## Safeguards, cleanup and disposition

No platform cybersecurity safeguard blocked the review. A sandboxed local
container-version inspection returned this environment error:

```text
Failed to obtain podman configuration: set sticky bit on: chmod /run/user/1000/libpod: read-only file system
```

The same read-only inspection succeeded with local permission. No automatic
approval rejection or unresolved safeguard remains.

Task-owned API/Vite services and the disposable database were removed; synthetic
credential files were deleted and the three loopback ports verified closed.
Pre-existing containers were untouched. Final independent report review approved
the finding and qualifications without blockers. Details are in [results.txt](results.txt). Reports and
commits remain local without a push. The assessment is complete; deployment is
**NOT READY** with SHARE-1 unfixed and broader security/public-host/device gates
open. The next bounded candidate is the SHARE-1 repair above.
