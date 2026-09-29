# Final-round visibility ignores departed sessions' responses

**READY** for this bounded frontend repair. A pending final-round visibility save
could previously finish after logout, session renewal, account/target change or
navigation and still restore old cached visibility, display an old message or
refresh old queries. The control now gives each session and target its own
editor, checks current session ownership before dispatch and response effects,
and retires ownership when that editor leaves.

For example, if you renew your session while releasing the final nine holes, the
old response cannot replace the new session's status or interfere with a newer
save. Current-session release/hide, conflict refresh and explicit retry still
work, including on locked finals. Ignoring a response does not undo an accepted
server update; fresh authorized reads remain authoritative.

The original implementation failed 19 of the 23 final held-response regressions;
all 23 pass after repair. Full validation passed: 829 frontend tests across 121
files, frontend/browser TypeScript, ESLint and production build. Ten real Chrome
scenarios passed at 320/390/1280px, covering session replacement, departure,
loading, pending saves, success and error/retry. Screenshots and console/network
assertions were checked. Independent read-only review found no actionable issue.
The [report](validation/visibility-lifetime-2026-09-29/README.md) retains evidence
and explains the synthetic API boundary used for these client lifecycle checks.

Include rebuilt frontend assets in your next deployment. No API, migration or
configuration change is required by this repair, and gg26.no was untouched.
Continue with the [testing checklist](testing_checklist.md). Pairing and
tournament-start callback concerns remain unverified and separately queued;
this step does not claim they were repaired or require them before testing.
