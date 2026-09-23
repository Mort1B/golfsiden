# Late Stableford settings responses respect session ownership

The confirmed PERSIST-3 Stableford path is repaired. The settings form now belongs
to one account, CSRF session, tournament and round. Before response-driven cache
writes or refetches, it checks that the session is still current and that the form
is still mounted. Conflict refresh continuations recheck before clearing drafts.
A replacement form has its own mutation state and cannot inherit the old receipt,
error or busy state.

For example, session A submits 50% and its response is held. The same account
renews its session and starts editing 70%. When the old response arrives, it cannot
overwrite the new input, show an old save receipt or trigger old round refetches.
The new session can save normally. Leaving the editor does not undo a request
already accepted by the server; current authorized reads recover actual settings.

The [validation report](validation/stableford-settings-lifetime-2026-09-23/README.md)
records 19 failing-first cases, 27 passing focused tests, **806 passing frontend
tests**, typecheck, lint, build and **nine passing Chrome scenarios** at mobile and
desktop widths. Independent source review found no actionable defect in this
bounded change. Browser API responses were synthetic; component tests inspect
the actual QueryClient and session transition implementation.

This Stableford repair is **READY**. Similar callbacks in pairing, tournament
start and final-round visibility remain source-supported, unverified concerns;
they were inspected read-only and are not claimed repaired. Final-round visibility
is the next proposed bounded follow-up. Deployment remains **NOT READY** pending
those concerns and operational/public-host/device acceptance. Completed validated
work is committed to main and pushed to origin/main as requested.
