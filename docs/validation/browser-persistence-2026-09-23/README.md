# Local browser and offline persistence assessment

Date: 2026-09-23. Assessed revision: `e0104af`.

**Three confirmed boundary defects, all P2/medium:** match input loss across
same-account session replacement, uncontrolled storage-failure retries, and
private cache restoration by a delayed administrator mutation. None demonstrates
a server authorization bypass or another account's UI displaying private data.
No repairs are included: application source, existing tests, migrations and
dependencies are unchanged. Diagnostic files here are assessment artifacts only.

The owner authorized read-only inspection and disposable local validation using
synthetic accounts. No production, external targets or real credentials were
accessed. Two independent read-only reviewers covered offline storage/runtime and
identity/cache/capability boundaries, then inspected the diagnostic evidence.
The primary ran validation and reviewed representative fresh Chrome screenshots.
The assessment is complete; deployment remains **NOT READY** with these findings
unfixed and broader operational/public-host/device gates still open.

Subsequent disposition: [PERSIST-1 was repaired](../match-note-retention-2026-09-23/README.md).
The findings and reproducer below describe the assessed revision; PERSIST-2 and
PERSIST-3 remain open.

## PERSIST-1 — Same-account session replacement loses unsaved match notes

**Confirmed in installed Chrome using real local authentication.**
[MatchNotes.tsx](../../../frontend/src/features/matchPlay/MatchNotes.tsx), lines 10–26,
keeps unsaved/failed-save input in component state. The private shell nests it
under session-keyed queue providers:
[AppShell.tsx](../../../frontend/src/ui/AppShell.tsx), line 21,
[ScoreQueueProvider.tsx](../../../frontend/src/features/scoring/offline/ScoreQueueProvider.tsx), lines 13–18,
and [MatchQueueProvider.tsx](../../../frontend/src/features/matchPlay/offline/MatchQueueProvider.tsx), lines 7–9.
A same-user login creates a new CSRF identity. Authority refresh then remounts
those descendants, dropping match input and its navigation guard without an
explicit save/discard decision.

The [browser probe](persistence.probe.ts) entered `7`, verified that logout was
guarded, and triggered page return without changing the session: input and guard
survived. It then performed an actual same-user login in the shared cookie jar,
asserted unchanged account/different CSRF, and triggered another persisted-page
return. The field became empty, the discard control disappeared, logout became
enabled, and the server still had no note. A second case reproduced the same
loss after a deliberately failed IndexedDB save. The retained
[before](match-before-390.png) and [after](match-after-390.png) screenshots show it.

**Control:** a successfully persisted offline match note survived the same
session replacement and subsequently reached the server with value `7`.
This affects transient match input, not the demonstrated durable queue path.
The direct login API models a second-tab login sharing cookies; the probe did
not automate the second tab's login form or navigate away from the dirty page.
The failed-save variant injects a synthetic `matches` object-store write error.

**Impact/recommendation:** avoid silent loss of entered match notes. Retain match
intent in an account-owned store above session-keyed descendants, preserve it
across same-account replacement, and clear it on account teardown. Keep it
explicitly nondurable until IndexedDB commits, with existing recovery/discard
and navigation guards. Test both failed and unsaved writes, unchanged-session
return, durable queue controls, account changes and late callbacks. This is the
next proposed bounded repair; it must not broaden into match scoring rules.

## PERSIST-2 — Storage failures can continuously reschedule queue work

[QueueRuntime](../../../frontend/src/features/scoring/offline/runtime.ts), lines
55–85, retains `snapshot.items` when IndexedDB listing fails, then schedules
another microtask whenever that stale snapshot contains eligible queued work.
An immediately rejected storage access therefore retries without a timer/backoff
boundary. This shared runtime serves legacy, four-ball and Stableford scoring.

**Confirmed mechanism in a bounded Chrome probe:** persist one queued edit while
offline; replace the IndexedDB getter with one that throws exactly 50 times;
simulate `navigator.onLine=true` and an online event while retaining the real
browser's offline isolation. A zero-delay timer runs only after **52 storage
accesses**, after the probe restores storage. The queue row remains present.
The source loop has no failure/progress condition that would stop repetition
while the fault continues.

**Impact/recommendation:** a sustained immediate storage failure can starve
browser tasks; other asynchronous storage failures can cause uncontrolled retry
load. Drain immediately only after successful storage access/progress, retain
pending data and the error, and let a bounded timer/manual wake retry failures.
Add a regression proving bounded attempts, task responsiveness and queue retention.

The outage is synthetic. The probe deliberately restores storage; it did not
leave the browser hung, measure sustained CPU use, reproduce a spontaneous
Chrome storage fault or show permanent queue loss. Severity concerns availability
under a storage fault, not attacker-controlled denial of service.

## PERSIST-3 — Delayed administrator success restores cleared private cache

[StablefordSettings.tsx](../../../frontend/src/features/tournaments/StablefordSettings.tsx),
lines 37–41, writes its successful response into the submitting account's query
key without a current-session or component-lifetime check.
[Session transition](../../../frontend/src/features/auth/sessionTransition.ts),
lines 21–34, clears private queries before publishing logout/account changes,
but that cannot stop an outstanding mutation continuation.

The [component probe](cache.probe.test.tsx) uses the actual settings component,
QueryClient and session-transition implementation with a held mocked successful
API response. After logout or switching to account B and unmounting the control,
it verifies the private workspace is empty. Releasing success recreates account
A's round while authentication remains null/B. Both cases and an unchanged-session
success control passed.

**Impact/recommendation:** cache erasure after logout/account change is not
lasting. Fence response-driven writes/refetches to the submitting user and CSRF
identity; suppress component-local callbacks after unmount. Add held-response
regressions for logout, different account and same-account replacement.

The restored key remains account A's. This is a private **in-memory** retention
failure; no account B UI disclosure, persistent-storage write or server-side
unauthorized operation was demonstrated. The API response is mocked, so the
probe establishes frontend continuation behavior rather than network ordering.
Similar source patterns exist in
[usePairingEditor.ts](../../../frontend/src/features/tournaments/pairings/usePairingEditor.ts), lines 109–112,
[TournamentStartPanel.tsx](../../../frontend/src/features/tournaments/TournamentStartPanel.tsx), lines 89–93,
and [FinalRoundVisibilityControl.tsx](../../../frontend/src/features/tournaments/FinalRoundVisibilityControl.tsx), lines 48–53.
Those paths were not independently reproduced and remain source-supported concerns.
Existing SavedCoursePicker and CountedRoundsEditor tests demonstrate the intended
guarded pattern; this assessment does not expand into repairs of all callbacks.

## Verified controls and explicit limits

- Durable ordinary inputs use `golf-pending-scores-v1`; match notes use the
  separate `golf-match-notes-v1`. Account-indexed reads, validated keys/targets,
  immutable request heads, conditional revisions, atomic leases and exact
  generations isolate queue work and conflict/discard actions. Session/CSRF
  runtime ownership fences dispatch and late acknowledgements. Broadcasts contain
  only notifications. Existing tests cover other-account isolation and late
  acknowledgements; real Chrome verifies logout retention, different-account
  hiding and resumption with a new session.
- Keeping same-account queued edits after logout is intentional and disclosed in
  the UI. They contain local inputs, IDs and conditional metadata, not full
  authoritative cards, passwords or session/CSRF secrets. They remain readable
  to someone controlling the same browser profile; account indexing is an
  application boundary, not encryption against that person.
- Private server queries live in account-rooted memory caches. Account transitions
  remove queries; private-result denials cancel before erasure, reject late
  results and scope invalidation. The PERSIST-3 mutation exception above remains.
  Return-order tests exercise held responses and Chrome page freezing at three
  widths. Full-card cold offline launch and background sync are unsupported.
- Source contains no service worker registration or persistent query-cache plugin.
  A fresh real Chrome control observed zero localStorage/sessionStorage entries,
  CacheStorage caches and service worker registrations. This inventory is scoped
  to that synthetic profile/flow, not proof about every installed browser profile.
- Public result visits own isolated in-memory query clients; reusable result
  tokens intentionally remain in URL fragments. Recovery removes its fragment
  and uses transient token/password state. Strict decoders, scoped receipts and
  response ownership have existing tests. Source inspection and the capability
  Chrome scenarios support these controls; history/profile access to a reusable
  share URL remains capability possession by design.
- Server conditional-score and match tests verify current session/tournament
  authority, stale revisions, replay semantics and locked-round rejection. A
  client queue grants no server authority. Membership-loss and blocked delivery
  are also exercised by frontend tests; these do not prove every cross-tab
  cookie/membership schedule.

Remaining gaps: malformed persisted-record recovery has limited explicit negative
coverage; match late-acknowledgement/account-departure coverage is narrower than
legacy score coverage; prolonged quota/corruption/device-loss behavior was not
stress-tested. The synthetic page-return events are not proof of every real
back/forward-cache path. Physical Android Chrome, native 200% zoom, production
TLS/proxy/database roles, recovery operations and advisories remain separate.

## Validation and reproducibility

| Check | Result |
| --- | --- |
| Full frontend suite | 751 passed in 116 files |
| Typecheck, lint and production build | Passed |
| PostgreSQL score/match suites | 99 passed, none ignored |
| Existing Chrome scenarios | 29 distinct scenarios passed across runs; harness correction below |
| Targeted Chrome diagnostics | Four cases passed after test-hook correction |
| Actual-component / mocked-API diagnostic | Three cases passed |
| Independent read-only source/probe/report review | No substantive objections; qualifications retained |

Final counts and cleanup are in [results.txt](results.txt). Existing tests are
unchanged. Passing diagnostic assertions confirm the defects/controls; they do
not mean the defective behavior is acceptable.

```sh
# frontend/
npm run test
npm run typecheck
npm run lint
npm run build
GOLF_OFFLINE_BROWSER=1 GOLF_MATCH_BROWSER=1 GOLF_STABLEFORD_BROWSER=1 \
  ./node_modules/.bin/playwright test --config playwright.lifecycle.config.ts \
  offlineLifecycle.browser.ts offlineScoring.browser.ts stablefordOffline.browser.ts \
  matchPlay.browser.ts returnOrdering.browser.ts --reporter=line
GOLF_FOUR_BALL_BROWSER=1 GOLF_RESULT_SHARING_BROWSER=1 GOLF_RECOVERY_BROWSER=1 \
  ./node_modules/.bin/playwright test --config playwright.lifecycle.config.ts \
  fourBall.browser.ts resultSharing.browser.ts resultSharingStates.browser.ts \
  passwordRecovery.browser.ts --reporter=line

# repository root; disposable synthetic DATABASE_URL injected privately
RUST_TEST_THREADS=4 cargo test --offline -p golf-api --features database-tests \
  --test scorecards --test four_ball --test stableford --test singles_match
```

The local PostgreSQL 17.10 container used a cached image with `--pull=never`, tmpfs
storage and 127.0.0.1:55443. The existing temporary API harness was built against
this unchanged source and bound 127.0.0.1:3000; Vite served the fresh frontend at
127.0.0.1:5173. It uses development cookies/throttling and no external provider,
not the production deployment stack. The initial recovery scenario could not
issue a receipt because this temporary harness lacked a recovery origin. A
rebuilt temporary harness supplied the explicit loopback origin
`http://127.0.0.1:5173`; both recovery scenarios then passed. This harness correction
did not modify production code or configuration. Seven other scenarios in the
initial additional run passed; the duplicate recovery mock rerun is counted once
in the 29 distinct existing scenarios. Backend score/match tests exercise their
own explicit authorization; this run makes no production configuration claim.
Full backend/Clippy/deployment ladders were not rerun for this assessment-only step.

To repeat diagnostics, copy the retained probes and their
[Playwright](playwright.config.ts) / [Vitest](vitest.config.ts) configurations to
`/tmp/golf-persistence-assessment`, with `node_modules` pointing to this checkout's
frontend installation. Paths are intentionally fixed to the assessed checkout
and loopback harness. Run Playwright using that config and Vitest with
`--config /tmp/golf-persistence-assessment/vitest.config.ts`. Never substitute a
persistent database. No dependency install or external connection is required.

The initial failed-save probe targeted the nonexistent `drafts` store and failed
its alert assertion; it provided no failed-save evidence. Correcting the diagnostic
hook to `matches` produced both successful reproduction cases. The bounded
retry probe also passed again. No production source/test fix was made.

## Safeguards and disposition

No platform cybersecurity safeguard or automatic approval rejection blocked this
step. Loopback services/tests used the authorized local permission path. No
unresolved safeguard restriction remains. The API harnesses and Vite were stopped, the disposable database removed,
synthetic credential files deleted and task ports verified closed. Pre-existing
containers were untouched. Final independent reviews approved the source/probe
claims and report qualifications. Details are recorded in [results.txt](results.txt). Work and commits remain local without a
push. Fixes are separate next steps; no queued implementation started here.
