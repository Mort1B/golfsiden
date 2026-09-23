# Local recovery capability and credential-revocation assessment

Date: 2026-09-23. Assessed revision: `ce72e45`.

**No confirmed security defect was found in this bounded assessment.** This is
not proof that none exists. Issuance, preview, redemption, expiry, revocation,
single-use behavior, session invalidation and browser transport were assessed.
The owner authorized source inspection and disposable local validation with
synthetic accounts. No production systems, external targets or real credentials
were accessed. Application source, tests, migrations and dependencies are unchanged.

Two independent read-only reviewers covered repository authorization/concurrency
and schema/operator boundaries. The primary review covered API and frontend
transport, ran validation and inspected representative browser screenshots.
The assessment is complete with the coverage gaps below; overall deployment
remains **NOT READY** because the wider security and deployment gates remain open.

## Evidence-backed controls

| Boundary | Source and exercised evidence |
| --- | --- |
| Issuance/revocation authority | `api/password_recovery/admin.rs` requires an active mutation session, CSRF and current-password confirmation. `repositories/password_recovery/authorization.rs` resolves the target from the exact tournament player and verifies current credentials, exact admin membership and ordinary-account eligibility. Rules/API tests cover self, privileged, unlinked, inactive, missing-membership and wrong-tournament denials. |
| Capability strength and storage | `domain/password_recovery.rs:11` uses independent 256-bit OS randomness, SHA-256 storage and constant-time comparison. Schema 24 constrains token hashes, lifetime, immutable grant identity and one unterminated grant per account. Unit/schema tests passed. |
| Expiry and authority changes | `repositories/password_recovery/public.rs:18` locks accounts in a consistent order before rereading the grant and checking token, generation, outcome, eligibility and the authority-change ledger. Admin mutations recheck session validity after waits and before commit. Contention/race tests cover clock expiry and authority changes away and back. |
| Single use and credential invalidation | `repositories/password_recovery/public.rs:91` changes the password and terminates the grant in one transaction, with a final wall-clock expiry check after writes. Credential generation invalidates old sessions and stale verified logins. Parallel redemption has one winner; replacement/revocation checks and revoke/password-change races passed. |
| Operator privilege/audit | `migrations/0024_password_recovery.sql:86` and repository checks require the actual recovery-table owner or superuser for operator provenance. Runtime startup rejects membership in that owner role. Synthetic restricted-role tests reject forged operator issuance/audits. CLI tests verify successful exact-account issue/revoke and private output. |
| Secret transport | Configured recovery origin is exact HTTPS, except development loopback HTTP; the server never derives it from Host. Secrets are issued in a URL fragment, sent to preview/redeem in POST bodies, and omitted from ordinary metadata. All recovery responses use private/no-store and no-referrer; request decoding/body limits and generic invalid-link errors are tested. |
| Browser identity and state | `ResetPasswordExperience.tsx` removes the fragment from history and retains the capability in component memory; API calls suppress referrers. Query keys omit secrets; short-lived mutation state is reset. `refreshRecoverySession.ts` avoids overwriting a concurrent/new or unrelated identity. Unit and real Chrome flows passed. |

Primary source directories:
[API](../../../backend/src/api/password_recovery/mod.rs),
[repository](../../../backend/src/repositories/password_recovery/mod.rs),
[token](../../../backend/src/domain/password_recovery.rs),
[schema](../../../migrations/0024_password_recovery.sql),
[operator CLI](../../../backend/src/bin/password-recovery.rs),
[frontend API](../../../frontend/src/api/passwordRecovery.ts),
[public form](../../../frontend/src/features/recovery/ResetPasswordExperience.tsx).

The operator is explicitly trusted to identify the exact account and deliver a
capability privately. The browser administrator flow requires identity verification
through a known contact channel. Code checks do not prove those human procedures
are followed. Owner/superuser database compromise is not contained by runtime
provenance checks.

## Unverified concerns and coverage gaps — not confirmed findings

1. **Operator output-failure compensation.** The CLI attempts exact-grant revocation
   if writing or syncing the private output file fails (`password-recovery.rs`,
   lines 111–126). Source scopes compensation to both account and grant ID, but
   this run did not inject output failures or a concurrent replacement grant.
   A focused failure-injection test would strengthen confidence; no erroneous
   revocation or leaked live capability was demonstrated.
2. **Configured runtime CLI refusal.** The existing schema test exercises a
   synthetic broadly granted runtime role against provenance/audit guards. The
   CLI test runs as a disposable owner. This is not a fresh invocation of the CLI
   under the deployment's actual configured runtime login; source checks support
   rejection, but production least privilege was not revalidated here.
3. **Late write-wait expiry.** Source rechecks capability expiry after password and
   audit writes. Existing contention tests exercise expiry during initial account
   acquisition, not a deliberately blocked later write. The final check is present;
   this narrower schedule remains untested. Time can also advance during COMMIT;
   the implementation does not claim atomic expiry at durability time.
4. **Transport/deployment limits.** Chrome used loopback HTTP and development cookie
   settings, with normal development throttling disabled. Backend tests explicitly
   exercise recovery route throttles. These results do not establish public TLS,
   proxy/logging configuration, multi-instance limits, physical Android behavior
   or native 200% zoom acceptance. No external advisory lookup was performed.

No remediation was implemented or inferred from these gaps. The next assessment
can proceed to private/public projections, with browser persistence assessed
separately afterward, without treating this report as whole-application sign-off.

## Validation

| Check | Result |
| --- | --- |
| Recovery token/origin Rust unit tests | 2 passed |
| PostgreSQL suites | 29 passed: recovery 20, authentication 6, credential concurrency 3 |
| Frontend recovery/auth/cache tests | 26 passed in 7 files |
| Frontend production build/type compilation | Passed |
| Installed Google Chrome | 2 scenarios passed; Chrome 153.0.8010.36 |
| Browser layout samples | 12 states at 320×600, 390×844 and 1280×900; 36 screenshots |
| Independent source review | No confirmed findings; gaps recorded above |

Commands executed against the assessed source:

```sh
cargo test --offline -p golf-api --lib recovery
cargo test --offline -p golf-api --features database-tests \
  --test password_recovery --test profile_concurrency --test auth

# From frontend/
npm run test -- src/features/recovery src/api/passwordRecovery.test.ts \
  src/features/auth/AuthProvider.test.ts src/api/privateWorkspace.test.ts
npm run build
GOLF_RECOVERY_BROWSER=1 ./node_modules/.bin/playwright test \
  --config playwright.lifecycle.config.ts passwordRecovery.browser.ts \
  --reporter=line --output=/tmp/golf-recovery-assessment/browser-results
```

The existing [Chrome tests](../../../frontend/e2e/passwordRecovery.browser.ts)
exercise administrator issue/copy/redeem/relogin, unrelated-session preservation,
target-session invalidation, lost-link revoke, missing/revoked/reused links,
password validation, loading/retry and success. The second scenario mocks public
preview responses for UI states; real capability expiry and concurrency are
covered by PostgreSQL tests. The first scenario uses real local API calls and
synthetic accounts. Browser assertions check horizontal overflow, recovery-control
44px height and enabled-control hit testing at each width. Unexpected page/console
errors and request failures fail the tests; expected 401/409/503 responses and
specific canceled requests are explicitly allowed by the test.

Representative screenshots were inspected and retained:
[administrator at 320px](admin-longname-320.png),
[public form at 390px](public-form-390.png),
[success at 1280px](public-success-1280.png).
The long tournament heading wraps aggressively at 320px; it does not overflow.
These samples are not a complete application design or keyboard-navigation audit.

PostgreSQL 17.10 ran from a cached image with `--pull=never`, tmpfs storage and
loopback binding at port 55443. The temporary [loopback harness](loopback-server.rs)
uses the unchanged `api::router` with local AuthConfig and explicit recovery origin,
binds only `127.0.0.1:3000`, and rejects any other database endpoint. It was built
as a separate temporary Cargo crate with a path dependency on `backend`, axum 0.8,
tokio 1 (macros/rt-multi-thread/net), sqlx 0.8 (runtime-tokio-rustls/postgres/migrate)
and this checkout's patched SQLx PostgreSQL crate. Vite preview served the freshly
built assets at `127.0.0.1:5173` and proxied only to that local API. No provider was
configured. This was not the production binary/configuration or deployment stack.

Full all-target backend/Clippy, full frontend test/lint and deployment ladders
were not rerun: this step changes no application implementation. The scoped
checks above establish this report's evidence, not a new full-release verdict.
Existing SQLx vendor warnings and Node color-environment warnings were observed.

### Safeguards and cleanup

No platform cybersecurity safeguard blocked an action. Initial local port
inspection with `ss` was denied by the default sandbox. Exact notice:

```text
Cannot open netlink socket: Operation not permitted
```

The same read-only inspection succeeded with local permission. This did not block
source review or validation. No unresolved approval or safeguard restriction remains.
The task-owned API and Vite preview were stopped; the disposable container and
its tmpfs data were removed. Synthetic credential configuration was deleted.
Pre-existing local services and application files were left untouched.

Sanitized totals are retained in [results.txt](results.txt). Reports and evidence
remain local; no push or external publication is part of this assessment.
