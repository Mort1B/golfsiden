# PERSIST-3: Stableford settings callback ownership

Date: 2026-09-23. Starting revision: `ca17461`.

**The confirmed Stableford-settings path is repaired.** A late mutation response
cannot recreate private round queries after its account/session departs or its
editor unmounts. The repair is ready within this bounded scope; deployment remains
**NOT READY** pending related unverified callbacks and operational/device gates.

## Implementation

[StablefordSettings](../../../frontend/src/features/tournaments/StablefordSettings.tsx)
keys its form by account, CSRF identity, tournament and round. It synchronously
ends mounted ownership on unmount and checks the canonical session query before
submission, each response-driven cache write, each invalidation, local completion
and the continuation after conflict refresh. The canonical check covers session
publication before React has remounted the form. The separate observer for a new
session prevents inherited mutation receipts/errors or old completions releasing
its busy state or clearing its draft.

Normal current-session success updates the same round/round-list queries. Stale
configuration refresh and explicit retry remain available. Server authorization,
expected-version checks and draft-only settings rules are unchanged. Ignoring a
late callback **does not undo a server write**: fresh authorized reads recover the
actual settings. No shared query boundary, API, migration, dependency, server or
production configuration changed.

## Reproduction and validation

- A failing-first suite used the actual settings component, QueryClient and session
  transition code with held synthetic API responses. **19 of 22 cases failed** on
  the old implementation ([baseline](baseline.log)): late writes/refetches and
  inherited mutation UI were observed; existing negative controls passed.
- The repaired focused suite passes **27 tests**, including 24 lifetime cases:
  success/stale/opened/generic failure after logout, another account, renewed CSRF,
  unmount and target change; canonical publication before React rerender; a new
  session's active draft while the old response arrives; conflict-refresh completion
  after replacement; normal failure/retry. StrictMode is enabled for these cases.
  Assertions inspect actual private query contents, writes, invalidation calls,
  current input and receipts. Existing value validation and locked-round UI pass.
- Full frontend: **806 tests across 120 files passed**, plus typecheck, lint and
  production build. Evidence: [focused](focused.log), [full tests](tests.log),
  [typecheck](typecheck.log), [lint](lint.log), [build](build.log).
- **Nine installed Chrome scenarios passed** on the production build with real
  routing/auth/session code, loopback SSE and synthetic typed API responses.
  Held success/conflict responses at 320/390/1280px preserve a new session's draft,
  produce no old receipt/error or extra round fetch, and allow its subsequent save.
  Logout, account switch and navigation departure also suppress the old receipt.
  The component tests establish cache erasure; the browser tests establish UI and
  network behavior. [Chrome output](chrome.log).
- Chrome uses long labels and an empty roster, checks pending/save states, no
  horizontal overflow and verifies the save control's 44px target and clickability.
  Console/page errors, failed requests and HTTP errors are checked; only the
  intentionally induced stale-settings 409 is allowed. Reviewed captures:
  [320px renewed input](renewed-320.png), [390px late conflict](conflict-390.png),
  [1280px renewed input](renewed-1280.png).
- Independent read-only review found no actionable defect in the final production
  change. The prior settings test fixture now seeds the canonical session query,
  matching real AuthProvider behavior; no assertion was removed.

## Related concerns, not repaired or newly confirmed

Read-only inspection found analogous unfenced continuations in:

- [usePairingEditor](../../../frontend/src/features/tournaments/pairings/usePairingEditor.ts),
  lines 109–118: response cache writes, draft adoption and error/success refreshes.
- [TournamentStartPanel](../../../frontend/src/features/tournaments/TournamentStartPanel.tsx),
  lines 89–95: tournament cache insertion, receipt and account-root invalidation.
- [FinalRoundVisibilityControl](../../../frontend/src/features/tournaments/FinalRoundVisibilityControl.tsx),
  lines 48–58: visibility cache insertion/receipt, projection invalidation and error refetch.

These remain **source-supported, unverified concerns**, not confirmed findings from
this step. Their runtime timing and impact need separate held-response reproduction.
No other-account UI disclosure or server authorization bypass is established.
Final-round visibility is the next proposed bounded follow-up; pairing and tournament
start remain queued separately. Course configuration's mounted check is also not
proof of complete session ownership; this repair does not claim application-wide
mutation coverage.

## Limits and closeout

Only synthetic local browser/API data was used. No production, external test
targets or real credentials were accessed. Browser responses are intercepted;
actual-server settings acceptance is not revalidated here. Backend/PostgreSQL
ladders were not rerun because this step changes only frontend callback ownership.
The prior PERSIST-1 Docker-backed repeat remains separately queued. No platform
safeguard rejection occurred during this repair.

Physical Android Chrome and native 200% zoom remain unverified. The task's preview
and SSE services are stopped at closeout. Work is committed to main and pushed to
origin/main under the owner's standing publication instruction.
