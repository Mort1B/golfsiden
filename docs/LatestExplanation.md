# Latest iteration: Archive UI and tournament history filtering

Exact tournament administrators can now archive a completed tournament from
the management lifecycle section. Confirmation explains preserved member access
and results, unchanged final-nine visibility, and no reversal. Draft/active
tournaments cannot archive; the archived panel links directly to history.

Dine turneringer now defaults to Nåværende (draft, active and completed) and offers
Arkiv and Alle views with counts. Selection is URL-backed, including browser
back/forward and direct archive links. Filtering never replaces the shared
membership cache or changes other tournament selectors.

## Safety and interaction

The typed API validates returned tournament identity and archived status.
Confirmation is permanently discarded when authoritative reads refresh, including
unchanged responses. Escape cancels and restores focus; duplicate submissions are
blocked. Success, conflicts and uncertain responses reconcile server state before
another action. Failed reconciliation requires a successful explicit refresh.

Completion and archive share the extracted closure-query reconciliation function
without changing completion behavior. Late responses never insert private cache
data after unmount/account change. Existing exact-admin access remains authoritative.

List loading, refreshing, error/retry, no-membership and empty-view states are
explicit. Failed reads hide stale cards. The list has no global SSE subscription:
return/window-focus refresh stale data under the shared 20-second freshness window;
manual refresh always requests current data after another administrator's archive.
Target-scoped management still receives live archive updates.

No backend, schema, score, membership, snapshot, final visibility or deployment
configuration changes are included. Archived history remains private and accessible
to existing members. There are no deletion or reversal controls.

## Validation and review

- All 326 frontend tests passed across 54 files, including 22 new archive API,
  panel, list-state and list-page tests. The full completion regression suite also
  passed after the shared reconciliation extraction.
- Coverage includes exact API shape/result identity, completed-only eligibility,
  confirmation/focus/Escape, refresh expiry, conflicts, lost successful responses,
  failed reconciliation recovery, duplicates/unmount, both live-refetch race orders,
  archive links, current/archive/all partitioning, browser-style back navigation,
  unfiltered cache preservation, empty/error/retry and identity transitions.
  Management tests assert no archive controls for non-admin roles, revocation or
  account change.
- Type checking, lint and production build passed. The existing Vite warning for a
  minified JavaScript chunk over 500 kB remains; bundle splitting is outside scope.
- Fresh PostgreSQL 17 migration/seed succeeded. Real Chrome scored, confirmed,
  completed and locked all five seed rounds, explicitly completed the tournament
  and archived it through a second administrator's UI. The first administrator's
  pending confirmation disappeared through live refresh. Members retained archive
  list/detail and hidden final-nine access, with no management controls.
- Chrome passed at 320, 390 and 1280 pixels for archive draft/blocker, confirmation,
  terminal, long-name, pending mutation and injected conflict states; list archive,
  loading, empty, long-name and error/retry states also passed. The suite verifies
  overflow, minimum button/link heights and actionability. Mobile/desktop
  screenshots were visually inspected.
- The real authenticated workflow produced no collected page, console or
  failed-network diagnostics. Deliberate conflict/read errors were injected only
  afterwards. Browser back navigation and current/archive/all list behavior passed.
- Independent read-only plan/source/test review found no concrete issues. Its
  documentation review clarified the stale-dependent return/focus refresh timing.

Backend unit/integration/Clippy and production deployment/recovery drills were not
repeated: backend and migrations are unchanged, and browser validation used only a
disposable local database. No checks were substituted for browser validation.

## Example and release verdict

A completed tournament remains under Nåværende. After explicit archive confirmation,
it appears under Arkiv and Alle instead. Its member history links still work and
hidden final-nine results stay hidden until the independent visibility action.

**READY WITH KNOWN LIMITATIONS:** this archive UI/history step and affected checks
pass. List-only cross-session updates use stale-dependent focus/navigation or manual refresh;
the bundle-size warning and production deployment checks remain outside scope.
The lifecycle UI queue is closed; further roadmap work needs an explicit priority.
