# Preserve unsaved scoring input through authority changes

H1 preserves failed device writes when the scoring screen is replaced by a lock,
access denial or metadata failure. It covers individual stroke play, scramble,
foursomes, four-ball and Stableford; match play retains its existing recovery path.

For example, a completed card has server score 4 and the user corrects it to 5.
If IndexedDB fails and an administrator then locks the round, 5 now remains in
local-only recovery. The user can explicitly discard it or save a device copy,
including offline. That copy requires fresh authorized canonical comparison and
an explicit choice before delivery. It cannot automatically alter the locked 4.

## Boundaries and decisions

An account-owned transient store sits above the scorer and the CSRF-keyed queue
runtime. It retains only target identity, hole/partner slot, entered numeric or
pickup value and the original conditional expectation/observed queue head.
Canonical cards, names, handicap values and results remain in TanStack Query and
are not displayed by recovery. Both four-ball partners retain independent intent
and stable ordinal labels even when the second partner was edited first.

Recovery starts on lock, denied scoring, failed required metadata, successful
removal of write access or a changed target. It remains active until the draft is
discarded or durably retained. A pending normal IndexedDB transaction is aborted
when recovery begins; durable recovery uses the existing conflict phase. It
rejects an existing operation or active confirmation lease rather than replacing
it. Normal retries keep the original conditional expectation and refuse an
unrelated queue head. A disappeared head still uses the original expectation,
letting server conflict checks decide rather than silently rebasing.

Navigation guards register separately, so an unmounting scorer cannot release
another active guard. Older save completion cannot erase newer local intent.
Same-account CSRF rotation preserves drafts. A real account change or authentication
teardown replaces the transient store; another account cannot see its contents.
Full reload/browser closure can still lose a draft that never became durable.
The unload guard warns about that boundary; it does not promise crash recovery.

No backend contract, database schema, stored queue protocol, immutable receipt,
match ledger, team assignment or handicap calculation changed. Locked rounds still
reject ordinary writes. Server delivery and conflict review remain authorized.

## Validation

Failing route regressions were captured before repair: failed local correction 5
was lost after lock, scoring 403 or metadata failure. The same cases now pass.
Focused storage/provider coverage includes atomic abort during a device write,
original expectation across canonical changes, other-tab collisions, disappeared
heads, late completion/newer input, confirmation leases, partner isolation,
same-account CSRF refresh, account changes and independent navigation guards.

- Complete frontend suite: **587 tests in 104 files passed**.
- H1 Chrome matrix: **6 passed**, covering all three legacy formats after external
  lock, four-ball numeric/pickup recovery after 403 and Stableford numeric/pickup
  recovery after metadata 500. Offline retention stayed in conflict and server
  scores remained unchanged. Explicit discard released navigation/logout guards.
- Recovery layout checked at **320x600, 390x844 and 1280x900**, with long account
  text, button trial clicks, at least 44px controls and no horizontal overflow.
  Expected fault-injection HTTP failures were distinguished from console and
  unexpected network errors. Mobile and desktop screenshots were inspected.
- Independent read-only review approved the final source boundaries and fixes.
  Every changed production source file remains below the 400-line limit.

Frontend type checking, lint and production build passed. The existing bundle
warning remains: JavaScript 777.41 kB (221.69 kB gzip), CSS 84.12 kB (14.31 kB gzip).
A separate phone clearance check passed with every recovery button entirely above
the fixed navigation after scrolling. The existing offline/match Chrome matrix
passed **30/30** (4.1 minutes), including lost acknowledgments, multiple tabs,
account isolation, blocked delivery, conflict review and match recovery. Together
with H1, **36 browser scenarios passed**, plus the separate phone clearance check.

Evidence logs are local, disposable artifacts: `/tmp/h1-red.log`,
`/tmp/h1-final-{tests,typecheck,lint,build}.log`, `/tmp/h1-browser-final.log` and
`/tmp/h1-legacy-match-browser.log`. Recovery screenshots use
`/tmp/golf-offline-h1-recovery-{320,390,1280}.png` and the scrolled phone-controls
capture `/tmp/golf-h1-recovery-controls-320.png`.

Browser scenarios used the local API and a fresh isolated PostgreSQL 17.11
container with all 32 existing migrations applied. The backend test/Clippy ladder,
database test/seed ladder and production deployment were not rerun: backend,
schema and deployment sources were unchanged. These browser checks are not a
production deployment certification.

## Remaining work

M1 (legacy session-expiry commit checks), M2 (denied private-result refresh) and
L1–L3 remain separate queued findings in [PLANS.md](PLANS.md). This iteration
addresses H1 only. The broader security review and performance measurement remain
later work.

**READY WITH KNOWN LIMITATIONS for H1.** The repair passed its affected checks and
review. Memory-only drafts still end at authentication teardown or forced reload,
and the existing bundle warning and separately queued findings remain unresolved.
