# Hide private results after authoritative denial

M2 removes cached private result data after a refresh returns HTTP 401, 403 or
404. Direct scorecards, player history, round/tournament standings and delegated
read-only match views now hide denied names, scores and links. Recovery requires
fresh authorized reads, including after retry, remount or metric changes.

For example, a direct card displayed gross score 7. Previously a denied refresh
could leave that card visible beside a background-error message while SSE stayed
connected. The denied projection is now erased, its dependent reads are cancelled,
and only the error/retry state remains. A later 500 or offline error cannot revive
7. A successful authorized refresh can display the current card again.

## Boundaries and review repairs

A focused query wrapper catches typed authority errors before query-library retry
handling. Cleanup cancels the source and affected sibling reads with snapshot
reversion disabled, then clears their data and retains invalidated error state.
Cancelling only siblings was insufficient: review reproduced an observer unmount
restoring the source's old snapshot. The final implementation and a dedicated
regression cover that race as well.

Scope matching uses canonical account/target keys and decoded IDs. Weak query-owned
associations retain IDs only, so clearing metadata does not lose a known target.
Unidentifiable same-account round read projections are conservatively cleared;
unrelated reads with a usable round-to-tournament association remain intact.
Missing associations can cause extra clearing even for another tournament; those
reads recover through fresh authorization. Mutable query metadata is not
used for this boundary because other consumers can replace shared query options.
The tournament leaderboard loader separately protects its nested rounds request
and observes cancellation between fetch stages. Superseded responses cannot
repopulate denied data or clear a newer authorized response.

Read-only match rendering uses the same protections, including its round metadata.
Writable match/scoring queries, H1 transient intent, durable queues and server
contracts are unchanged. The existing match-only tournament-selector finding L3
is still queued. HTTP 500/network failures without a prior denial retain the
existing permitted-data behavior. No broad authentication or SSE redesign was
introduced, and no private projections were moved out of TanStack Query.

## Validation

Before repair, four route regressions failed for cached cards after 401/403/404
and denied round metadata; the transient 500/offline control passed. The focused
suite now passes **17 tests**, including history/results denial, read-only match
metadata, source-observer unmount, sibling metrics, pending and superseded reads,
missing metadata, nested-loader cancellation and account/known-trip isolation.

The complete frontend suite passed **602 tests in 106 files**. Type checking,
ESLint and production build passed. The existing bundle warning remains:
JavaScript 781.26 kB (222.30 kB gzip), CSS 84.12 kB (14.31 kB gzip).
Independent read-only source review found no remaining material issues.

The three M2 Chrome scenarios passed: direct-card 401, player-history 403 and
round-results 404, each with a real native SSE connection verified OPEN and no
error or reconnect during denial. They cover preceding transient 500 retention,
post-denial 500/retry suppression, erased names/links and successful recovery.
The first test iterations corrected endpoint URLs and waited for the real stream
to open after its initial keepalive; final assertions preserve the healthy-stream
requirement. Checks and inspected screenshots cover 320x600, 390x844 and 1280x900,
long content, no horizontal overflow and at least 44px usable controls.

The existing 44-case Chrome matrix initially passed. The final frozen-source run
passed **43/44**: the existing offline/frozen-page return test timed out waiting
five seconds for its read-only state. The unchanged exact test then passed
**3/3 isolated repeats**. This is recorded as an intermittent validation limitation,
not a clean final sweep. Review traced that fixture to `/score`, where the changed
M2 result components do not execute. Overlap between its online refresh and later
page-return event is a plausible existing ordering race, not a proven cause.
A separate follow-up is queued; no lifecycle code or assertion was weakened.

Browser fixtures use the current local API and a fresh isolated PostgreSQL 17.11
database with all 32 existing migrations applied. No production data is used.

Evidence logs are disposable local files: `/tmp/m2-red.log`,
`/tmp/m2-focus-final.log`, `/tmp/m2-test.log`, `/tmp/m2-types.log`,
`/tmp/m2-lint.log`, `/tmp/m2-build.log`, `/tmp/m2-browser4.log` and
`/tmp/m2-final-regression-browser.log`, `/tmp/m2-regression-browser.log` and
`/tmp/m2-return-repeat.log`. Screenshots use
`/tmp/golf-offline-m2-{denied,recovered}-{320,390,1280}.png`.
Committed tests preserve the regression scenarios.

Backend, PostgreSQL test/seed and production-deployment ladders were not rerun:
backend contracts, migrations and deployment sources are unchanged. The API was
built from the current checkout and existing migrations were applied for browser
validation. Frontend checks and actual API/browser evidence validate this scope.

## Remaining work

L1 (signed-minimum allocator), L2 (format-specific course errors) and L3
(match-only tournament selection) remain separate candidates in
[PLANS.md](PLANS.md). Performance measurement and the wider security review
remain later work.

**READY WITH KNOWN LIMITATIONS for M2.** Source and documentation review,
frontend checks and new privacy scenarios passed. The intermittent existing
frozen-page return check, conservative cache clearing and existing bundle warning
are recorded above. This is not a broader application-security certification.
