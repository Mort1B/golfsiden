# SHARE-1: reject result-link mutations after late session expiry

Date: 2026-09-23. Parent revision: `ef79897`.

The [confirmed assessment](../result-projection-security-2026-09-23/README.md)
identified a public-link issue request that committed after its session expired
while waiting on the audit table. This step repairs that bounded mutation
boundary. It adds no features, migrations, dependencies or frontend changes.
Only disposable loopback PostgreSQL, synthetic accounts and local browsers were
used; no production, external targets or real credentials were accessed.

## Change and invariants

The [management repository](../../../backend/src/repositories/result_sharing/management.rs)
reuses the existing active-session predicate after grant/audit writes immediately
before commit. Replacement also rechecks after terminating the old grant, before
inserting the new one: otherwise expiry at that first audit is rejected by the
new grant's database guard as an internal error rather than unauthenticated.

Session/user and exact membership locks were already acquired and remain held.
The added queries reacquire only the same session/user share locks and use
PostgreSQL wall-clock expiry and credential generation. No lock order, membership
rule, expected-grant intent, public projection, token lifetime or token storage
changes. The existing 401 `unauthenticated` API mapping applies. Failure rolls
back every grant/audit change; handlers emit an invalidation only after a
successful repository commit. Failed replacement/revoke preserves the original grant unchanged; an otherwise
valid capability remains usable. Valid mutations retain their existing 201/204 responses.
This boundary does not freeze time or promise atomic expiry during COMMIT itself.

## Regression evidence

The new [PostgreSQL API tests](../../../backend/tests/result_sharing/session_expiry.rs)
exercise eight cases: expired and valid sessions for issue, revoke, replacement
at the old grant's audit and replacement at the new grant's audit.

| Expiry point | Before repair | After repair |
| --- | --- | --- |
| Issue audit | 201 (incorrect success) | 401, no persisted changes/event |
| Replacement old audit | 500 (new grant guard) | 401, original grant/audit unchanged |
| Replacement new audit | 201 (incorrect success) | 401, both writes rolled back |
| Revoke audit | 204 (incorrect success) | 401, original grant/audit unchanged |
| All four unexpired controls | Passed | Passed, expected audit counts and exactly one event |

Before the source repair, all four expiry assertions failed and all four valid
controls passed. After repair, all eight passed, as did all 14 existing sharing
tests. Tests compare relevant persisted grant fields and complete audit rows,
check absence of failure events, validate the exact successful tournament event,
and read old/new capabilities anonymously to verify survival/invalidation.

A separate transaction holds a maintenance-style audit-table lock for issue,
revoke and the first replacement audit. A test-only conditional trigger pauses
only the new issuance audit on an advisory lock for the second replacement wait;
this trigger exists solely in that test's disposable database. Tests observe the
exact lock and blocker while the session is still active, then wait for database
wall-clock expiry before unlocking. Three-second lifetimes are synthetic fixtures,
not production session settings. This proves controlled late-wait behavior, not
an anonymous ability to cause a production lock or delay.

## Validation

Final totals and cleanup are recorded in [results.txt](results.txt).

| Check | Result |
| --- | --- |
| Focused sharing integration suite | 22 passed, including eight new cases |
| Format / full backend / Clippy | Passed; 215 backend tests |
| Complete PostgreSQL-enabled rerun | 615 passed, three existing benchmarks ignored |
| Migrate and seed commands | Passed on disposable PostgreSQL 17.10 |
| Frontend focused tests / production build | 35 passed; build/type compilation passed |
| Independent read-only review | No findings |

The first full database run stopped on unchanged
`round_configuration::configuration_first_commits_before_waiting_open_rechecks_readiness`
(409 versus expected 200). Its focused rerun passed, followed by the complete
`--no-fail-fast` rerun with all 615 tests passing. The test uses fixed 100ms waits
rather than observing lock acquisition; this is a timing-test concern, not a
confirmed product defect. No unrelated code/test repair was made. These totals
include backend unit tests. The three ignored match-listing benchmarks require
an exclusive database with `pg_stat_statements`; no new tests were ignored.

Chrome 153.0.8010.36 passed both scenarios (55.3 seconds), with 17 layout states
at three widths (51 screenshots). Representative fresh screenshots were inspected:
[live mobile](anonymous-live-390.png), [320px long content](public-longnames-320.png),
[desktop error](public-error-1280.png). Content stays within the viewport; the
error state removes previous rows. This is scoped sharing-flow validation, not a
new application-wide design audit.

```sh
# Before repair: four failed expiry cases, four successful controls.
cargo test --offline -p golf-api --features database-tests \
  --test result_sharing session_expiry
# After repair: all 22 sharing tests passed.
cargo test --offline -p golf-api --features database-tests --test result_sharing
cargo fmt --all -- --check
cargo test --offline --workspace --all-targets
cargo clippy --offline --workspace --all-targets --all-features -- -D warnings
cargo test --offline --workspace --all-targets --features database-tests --no-fail-fast
cargo run --offline -p golf-api --bin migrate
cargo run --offline -p golf-api --bin seed

# From frontend/
npm run test -- src/features/resultSharing src/api/resultSharing.test.ts \
  src/api/privateResults.test.ts src/pages/PrivateResults.test.tsx
npm run build
GOLF_RESULT_SHARING_BROWSER=1 ./node_modules/.bin/playwright test \
  --config playwright.lifecycle.config.ts resultSharing.browser.ts \
  resultSharingStates.browser.ts --reporter=line \
  --output=/tmp/golf-share-repair/browser-results
```

The full Rust/PostgreSQL ladder used `RUST_TEST_THREADS=4`. Database credentials
were injected by a temporary local wrapper and never printed. PostgreSQL used a
cached image with `--pull=never`, tmpfs data and loopback port 55443. The Chrome
harness was rebuilt against the repaired backend; it uses the unchanged API router
with development configuration at 127.0.0.1:3000, with Vite preview proxying from
127.0.0.1:5173. Development throttling is normally disabled; existing backend
sharing tests explicitly exercise throttles. No external provider was configured.

The browser scenarios check real issue/copy/replace/revoke and anonymous results,
plus mocked loading, error, empty, long-content and return/offline states at
320×600, 390×844 and 1280×900. Browser tests supplement the deterministic database
expiry schedule; they do not reproduce a three-second session expiry in Chrome.
Expected negative responses and specific aborted requests are explicitly allowed;
unexpected monitored console/page/network errors fail the scenarios.

No frontend source changed. Its focused tests and production type/build check
plus browser scenarios supplement the affected complete backend/database ladder;
the full frontend unit/lint ladder was not rerun. Public TLS/proxy/database-role
acceptance, physical Android Chrome and native 200% zoom are not established by
loopback browser checks. Existing vendored SQLx warnings remain unchanged.

## Review and safeguards

Independent read-only review found no code or regression-test issues. The
reviewer checked session predicates, already-held locks, both replacement writes,
rollback, event timing and control cases; they did not independently run tests.

No platform cybersecurity safeguard or automatic approval rejection occurred.
The first sandboxed test attempt could not connect to the disposable database:

```text
failed to connect to setup test database: Io(Os { code: 1, kind: PermissionDenied, message: "Operation not permitted" })
```

That attempt establishes no regression result. The same offline tests then ran
with local permission against the authorized loopback service and reproduced the
four failures reported above. There is no unresolved safeguard block.

The task-owned API and Vite were stopped; the container and temporary database
were removed, synthetic credential files deleted, and task ports verified closed.
Pre-existing local containers were untouched.

SHARE-1 is repaired within this boundary. Broader persistence, operational and
public-host/device gates remain open; overall deployment is still **NOT READY**.
Work and commits remain local without a push. No queued work is included.
