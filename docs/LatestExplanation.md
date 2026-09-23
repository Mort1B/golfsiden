# Match notes survive a renewed session for the same account

PERSIST-1 is repaired. A numeric match note now lives in an account-owned transient
store above the session-keyed delivery providers. Renewing the same account's
session preserves the value, selected hole, failed-save error and original
conditional metadata. It remains labelled **Ikke lagret på enheten** until the
IndexedDB transaction commits. Logout/account change clears transient state and
aborts unfinished writes; completed queues keep their existing account ownership.

Recovery stays visible when card/round data is unavailable or access changes.
The navigation guard shares the account lifetime, so switching loading, recovery
and scoring views cannot temporarily release it. Router matching also covers
accepted trailing-slash and case variants. Original match revisions, locks,
explicit corrections and cross-tab queue generation checks remain enforced.

For example, a failed save of `7` on hole 3 remains `7` on hole 3 after same-account
session replacement, with its error and discard choice visible. It is not described
as device-saved until storage commits. A full reload still ends transient input.

The [validation report](validation/match-note-retention-2026-09-23/README.md) records
failing-first reproduction, 17 new unit/component tests, the full 768-test suite,
typecheck, lint, build and real Chrome checks at 320/390/1280px. Independent review
found no remaining actionable source defect after guard corrections.

The repair is **READY WITH KNOWN LIMITATIONS**: Chrome used synthetic API responses.
Docker socket permissions prevented repeating the PostgreSQL-backed browser
controls; no platform safeguard rejected the review or repair. There are no
backend, migration, dependency or production configuration changes.

Deployment remains **NOT READY** while PERSIST-2/PERSIST-3 and operational/device
acceptance remain open. PERSIST-2 is the next proposed bounded repair. This step
stops here, committed locally without a push.
