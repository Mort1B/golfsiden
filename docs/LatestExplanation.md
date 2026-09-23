# Result-projection assessment confirms a late session-expiry defect

The bounded [result-projection assessment](validation/result-projection-security-2026-09-23/README.md)
confirmed SHARE-1 (P2/medium). A public-link issue request starts with a valid
administrator session and CSRF token, waits during its audit write, then commits
a usable grant after the session expires. The local probe observed the precise
database wait and expiry, a 201 issue response, persisted grant/audit and event,
then session 401 and anonymous capability 200. An unexpired control succeeded.

The deliberate blocker was a maintenance-style audit-table lock; no anonymous
ability to cause it was demonstrated. The empty synthetic tournament proves
capability usability, not populated-score disclosure. No expired-at-entry,
cross-tournament, CSRF or public-field bypass was established. Revoke has a
similar source pattern but was not independently reproduced. Recommended repair:
recheck the session after grant/audit writes immediately before commit, with
rollback/no-event expiry tests for issue, replacement and revoke.

Application source, tests, migrations and dependencies are unchanged. The report
separates confirmed evidence from private-read lifecycle and format-coverage gaps.
Two independent read-only reviews covered public and private boundaries; the
public reviewer also checked the diagnostic reproduction and final report.

Validation passed 45 PostgreSQL tests, one Rust unit test, 35 frontend tests and
a production frontend build. Both result-sharing scenarios passed in installed
Chrome 153 at 320, 390 and 1280px, producing 51 layout samples. Representative
mobile live results, long names and desktop error screenshots were inspected.
The real flow exercised issue/copy/replace/revoke; mocked states covered loading,
errors, empty/long results and return/offline behavior.

The assessment is complete, but deployment remains **NOT READY** with SHARE-1
unfixed and broader persistence, operational and public-host/device gates open.
No platform cybersecurity safeguard blocked the review; a sandboxed Podman
inspection needed local permission and then succeeded. Cleanup and sanitized
results are recorded with the report. Work remains local without a push.
The next proposed bounded step repairs SHARE-1; implementation remains separate.
