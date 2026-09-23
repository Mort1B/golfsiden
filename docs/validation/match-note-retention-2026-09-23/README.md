# PERSIST-1: same-account match-note retention

Date: 2026-09-23. Starting revision: `4cca221`.

**Repair ready with known validation limitations.** Unsaved and failed-save match
notes survive same-account CSRF replacement, including their hole, raw value,
error and original conditional expectation. They stay explicitly nondurable until
IndexedDB commits. Account departure clears transient state; committed queues
remain account-scoped. Deployment remains **NOT READY** because PERSIST-2,
PERSIST-3 and operational/public-host/device gates remain open.

## Boundary and behavior

- [MatchNoteProvider](../../../frontend/src/features/matchPlay/drafts/MatchNoteProvider.tsx)
  owns transient notes above both CSRF-keyed queue providers. The store contains
  input and target/conditional metadata, never canonical cards, names or authority.
- [MatchNoteStore](../../../frontend/src/features/matchPlay/drafts/store.ts)
  retains exact intentions while a device write settles, even after its old runtime
  stops. A committed write clears only that sequence. A failed write keeps its
  error and releases saving state. Original revision/generation/old value remain
  unchanged; only a proven local append advances matching sibling generations.
- [IndexedDB retention guard](../../../frontend/src/features/matchPlay/offline/database.ts)
  aborts unfinished writes when the account-owned store closes. Late completion
  cannot mutate a new account's store. There is no database schema/API change.
- [MatchPage](../../../frontend/src/pages/MatchPage.tsx) renders minimal input
  recovery when round/card data is absent, membership is denied or a match becomes
  terminal/locked. Actions still require current authority. Stable route, logout
  and unload protection spans loading/recovery/scoring transitions. `useMatch`
  preserves React Router's accepted trailing slash and case variants.

Example: type `7` on hole 3, encounter a failed device save, and renew the same
account's session. The field remains `7` on hole 3, the error remains visible and
logout/navigation stay guarded. Explicit discard removes it. Full reload or
account departure still ends this memory-only copy.

## Validation

- [Failing-first nested-provider tests](baseline-failure.log) reproduced both original loss cases; the
  account-departure control passed. After repair, 11 lifecycle tests pass with
  React StrictMode: unchanged/replaced session, account change/logout, pending
  success/failure and fresh-cache round loading/error, missing card, membership
  denial, terminal and locked recovery. These assert route/logout/unload guards.
- Six store tests cover original conditional metadata, unrelated generation
  rejection, sequential sibling appends, delayed commit/error after runtime stop,
  actual IndexedDB transaction abortion and isolation from late old-account work.
- Full frontend: **768 tests across 118 files**, typecheck, lint and production
  build passed. See [unit output](tests.log), [typecheck](typecheck.log),
  [lint](lint.log) and [build](build.log).
- Installed Google Chrome, production build, synthetic API responses and loopback
  SSE: **19 scenarios passed**, recorded in [Chrome output](chrome.log). Ten new scenarios cover both
  input states at 320/390/1280px, same-session and same-account return, denial,
  session expiry/re-login, durable queue retention and alternate URL guards.
  Six existing match-navigation scenarios and three frozen-page return races
  exercise adjacent match/ordinary-score boundaries.
- Chrome assertions inspect page/console errors, failed requests and HTTP failures;
  only deliberately induced 401/503 responses are accepted in relevant cases.
  Checks include horizontal overflow, scoring/navigation target height and
  unobstructed click trials. Reviewed screenshots: [failed save at 320px](failed-320.png),
  [retained input at 1280px](retained-1280.png), [denied recovery at 390px](denied-390.png).
- Independent read-only review found an alternate-URL blocker gap during iteration;
  it was repaired with router matching and covered in Chrome. Final source review
  found no remaining actionable defect. Fresh-cache tests also caught a branch
  transition blocker gap, fixed by stable account-owned guard placement.

## Limits and exact blockers

The attempted disposable PostgreSQL container did not start. Docker returned:

```text
permission denied while trying to connect to the docker API at unix:///var/run/docker.sock
```

The noninteractive permission check returned:

```text
sudo: a password is required
```

These were local OS permission errors, **not a platform safeguard rejection**.
No safeguard rejection notice was received. The eight existing database-backed
match browser scenarios and the actual-login replacement probe were therefore
not rerun here. The prior assessment contains the original actual-login failure
and durable delivery control; this repair's Chrome API boundary is synthetic.
Backend/PostgreSQL ladders were not rerun: there are no backend/migration changes,
and the planned disposable database was unavailable. No substitute production
or external database was accessed. Device faults are injected, not physical disk
exhaustion; physical Android Chrome and native 200% zoom remain unverified.

Initial validation retries corrected fixture issues: a select locator, expired
session modeled as null/200 instead of 401, and an omitted online event. They
are not application defects. A lint-only unused callback parameter was corrected.
An early browser run was stopped while the final build was still being produced.
No tests or safeguards were weakened.

PERSIST-2 retry scheduling and PERSIST-3 administrator callbacks are unchanged.
Temporary synthetic config files and the loopback preview are removed at closeout;
no database/container was created. Work is committed locally without a push.
