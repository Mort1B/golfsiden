# Latest iteration — retained leaderboard browser-test maintenance

A1x repairs the retained live leaderboard browser test. The original case fails
at its stale server-status assertion: the component renders `Lagret på serveren`,
while the test accepts `Synkronisert` or `Lagret`. Both stale assertions now match
only `Synkronisert` or `Lagret på serveren`. Queued/local/failed delivery labels
remain excluded. No application UI/API/SQL, schema, dependency or scoring policy
changes.

Source review also found the negative fixture targeting a retired endpoint/error
UI. It now faults the actual conditional PUT, proves an attempted 503 delivery,
requires the current failed durable-queue status and unchanged server/standings,
then uses explicit two-step local discard. The fault stays installed until the
queue is empty and the acknowledged server value returns, plus one bounded
2200ms heartbeat check before restoring normal delivery. Subsequent real saves
must still pass. This is necessary test support, not a product repair.

The complete real Chrome case now reaches all later scenarios: three numeric
formats and gross/net updates, selection exclusions, historical correction,
locked-score rejection, actual API crash/offline projection clearing/reconnect,
and member final-front-nine visibility with release/re-hide. Existing precise
refresh requests, canonical JSON and UI assertions remain. Its final unexpected
page/console/network failure list is empty under explicit fault/outage allowances.

All 1,133 frontend tests, TypeScript, ESLint and production build pass. Both phases
use fresh migrated/seeded PostgreSQL and the locked API build. Twenty-one fresh
screenshots cover seven states at 320/390/1280px with horizontal-width checks;
representative mobile populated, desktop failed-delivery and member re-hidden
screenshots were inspected. Independent test review finds no material issues.
Owned services, database and temporary credentials are removed. Exact receipts,
review and limits are in the
[acceptance report](validation/leaderboard-browser-maintenance-2026-10-15/README.md).

This is one combined desktop/mobile case, not separate complete runs at every
width. The late-retry check covers one current heartbeat; physical devices,
every timer interleaving, production proxy/TLS and broad security/performance
sign-off remain outside acceptance. Backend ladders are not repeated because
application source, schema and dependencies are unchanged. The next candidate
is the retained expiry-test timing investigation, which has not started.
