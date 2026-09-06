# Latest iteration: Tournament completion UI

Exact tournament administrators can now finish the tournament from the management
lifecycle section. The panel lists unlocked rounds with direct management links
and requires every configured round, not only those counted in standings, to be
locked. Draft, loading, invalid-plan, error and terminal states are explicit.

Completion is deliberate: a focused confirmation explains that joining closes,
existing members retain access, reopening is unavailable, and final-nine visibility
does not change. Escape cancels; authority/read refresh permanently expires the
confirmation. The existing versioned backend action remains the authority.

## Safety and boundaries

- The existing exact-tournament administrator gate controls access. A platform
  administrator with only player/scorer/viewer membership receives no controls.
- The API decoder verifies tournament identity and completed status. A synchronous
  guard prevents duplicate submissions; mutations are not automatically retried.
- Success, conflict and uncertain failure all reconcile authoritative user-scoped
  reads. Failed reconciliation blocks completion until an explicit refresh works.
  Newer live refreshes may supersede those reads without false failure receipts.
- Late responses do not insert private data after an account switch or unmount.
  No score, team, handicap, snapshot, final visibility, backend or schema changes
  are included. Archiving and reopening remain unavailable.

## Validation and review

- All 304 frontend tests passed across 50 files, including 19 new focused API,
  readiness and component tests. Coverage includes incorrect completion responses,
  full-plan readiness, keyboard focus, expired confirmation, conflicts, lost
  successful responses, failed reconciliation, duplicate submission, unmount and
  both pre-commit and post-commit live-refetch race orderings. Management-gate
  assertions also cover absent completion controls after revocation/account change.
- Type checking, lint and production build passed. Vite retains its non-blocking
  warning for a minified JavaScript chunk above 500 kB; splitting is outside scope.
- Fresh PostgreSQL 17 migration and seed succeeded. Real Chrome completed the
  entire seeded five-round workflow against that backend, used a second admin
  session to lock the final round and submit completion, and observed live removal
  of an already-open confirmation in the first session. Existing member reads and
  hidden final-nine progress remained intact; member management controls were absent.
- The Chrome suite passed at 320, 390 and 1280 pixels for draft, blocked, confirmation,
  completed, loading, empty, error/retry and long-content states. It checked document
  overflow and minimum 44-pixel buttons. Mobile and desktop screenshots were visually
  inspected. The real workflow produced no collected console, page or failed-network
  diagnostics; deliberate edge-state errors were injected afterwards.
- Final independent read-only review found no concrete source/test issues. A
  documentation clarification makes clear that terminal read-only behavior applies
  to the completion panel, not to independent visibility/revocation controls.

The full backend unit/integration/Clippy ladder was not repeated because backend
and migrations are unchanged; the real browser used the previously implemented
backend contract. Production deployment, recovery drills and exhaustive testing of
other browser engines were not performed: this was disposable local Chrome validation.
Initial test timing and a TypeScript-only test option error were fixed before the
passing runs. No runtime dependency or deployment configuration changed.

## Example and release verdict

With four rounds locked and the final round merely completed, the panel links to
that final round and disables tournament completion. Locking it enables the
confirmation. Completing the tournament leaves the final nine hidden until the
administrator independently releases them.

**READY WITH KNOWN LIMITATIONS:** the completion UI and affected validation pass.
Archive backend/UI and general tournament editing remain separate queued work;
the existing bundle-size warning and production deployment checks are unchanged.
