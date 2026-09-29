# Ready for continued functional testing

**READY WITH KNOWN LIMITATIONS** for continued application testing. The main
workflows passed against a fresh local PostgreSQL database and real Chrome:
organizer/player login, tournament creation and invitations, saved-course setup,
manual teams/flights, start/open, score entry/editing, results, confirmation,
corrections and round locking. Team and individual scorecards and gross/net
results remained exactly unchanged after restarting PostgreSQL, the API and the
frontend and signing in from a fresh browser session.

No application or schema changes were needed. Two browser tests were corrected:
one now waits for confirmation cleanup before hard navigation, and one correctly
expects the second opponent's preserved local-note slot. Existing format checks
also passed for Stableford, four-ball and singles match play. The practical
journey covers 320px, 390px and 1280px widths; lifecycle checks also use 1440px.

Validation passed: 215 ordinary backend tests; 615 database-enabled tests
(including ordinary tests); 806 frontend tests; formatting, strict Clippy,
frontend typecheck/lint/build, browser typecheck, and 19 final passing existing
browser cases plus the before/after restart journey. The
[report](validation/test-ready-2026-09-29/README.md) distinguishes real API flows,
injected error states, initial failures, repeat runs and remaining limits.

One earlier journey recorded a transient scorecard-read HTTP 500 caused by a
PostgreSQL concurrency conflict. Saved data remained correct and the repeat and
restart checks passed. The precise overlapping action remains unconfirmed;
this is recorded as an unresolved limitation, not claimed fixed.

Use the [short checklist](testing_checklist.md) to continue testing. The user owns
hosting at gg26.no; this step did not access or change it. Physical phone and
hosted configuration acceptance remain unverified here. Further work follows
concrete testing feedback; broader assessment and speculative repairs are deferred.
