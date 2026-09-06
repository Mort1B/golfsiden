# Latest iteration: Visibility-safe flight progress

Opened round details now show **Fremdrift per flight** below the stored setup.
Each flight displays registered/required hole entries across its scorecards and
each card's progress. Shared team cards count once; full projections also display
completed and confirmed card counts. Drafts explain when progress becomes available.

## Boundaries

The frontend reuses member-readable pairings and completion-validation. Individual
owners come from preserved opening snapshots; team cards map through every stored
member of that exact round team. Current roster activity, current teams, start
times and starting holes never determine ownership or progress. Older rounds
without stored flight mappings show an explicit unavailable message, not an
inferred assignment. Backend contracts and database schema are unchanged.

Non-admin members viewing a hidden final see only holes 1–9. Aggregates sum only
the projected counts, with no completion/confirmation totals or percentages
suggesting full-round completion. Round completed/locked status does not override
redaction. Exact admins retain their authorized full projection.

The canonical account/round completion query participates in existing score/SSE
invalidation and synchronous cache clearing on visibility signals, stream opening,
reconnection and stream errors. No derived progress is retained in local state.
Failed reads hide retained progress; lifecycle/mapping mismatches offer a refresh
of both the round setup and progress. No score mutations, missing-score alerts,
automatic team setup or administrator readiness redesign were added.

## Validation and review

- All 285 frontend tests in 47 files passed, plus strict typecheck, lint and
  production build. Fifteen new mapping/component cases cover all three formats,
  shared cards, preserved-owner mapping independent of active rosters, hidden
  open/completed/locked rounds, missing/split historical mappings, draft/empty/error
  states and visibility/disconnect/reconnect cache clearing. Existing auth/cache
  and decoder tests also passed.
- PostgreSQL 17 migrations and development seed passed in an isolated disposable
  database. All 21 focused integration tests passed: final-round visibility (4),
  private workspace reads (4), and round pairings (13).
- The opt-in Chrome flight-progress suite passed against the real local API:
  all formats, shared-card counts, live score increments, final hidden/released/
  re-hidden projection, confirmation, correction and reconfirmation. No unexpected
  page/console errors, error responses or failed requests occurred in these flows.
- Browser loading, empty, long-name, denied-read and retry states used scoped
  response injection after the real API checks. All states were checked at
  320px, 390px and 1280px without horizontal overflow. Mobile hidden and desktop
  released screenshots were inspected. The intentional 403 was checked separately.
- Read-only review found no production defects. A P3 documentation ambiguity was
  corrected to distinguish non-admin front-nine views from full admin projections.

Initial test-harness syntax, notification-timing and repeated-text assertions were
corrected before the passing ladder. The existing Vite chunk warning remains
(573.88 kB minified, 166.27 kB gzip). Backend unit/Clippy, the full PostgreSQL suite,
separate lifecycle/round-details browser suites and deployment/recovery drills
were not repeated: production backend/schema/deployment code did not change.
Browser checks use the development proxy, not production.

## Example and verdict

A four-player scramble flight has two shared cards. Its denominator is 36 hole
entries for 18-hole cards, not 72. A restricted final view with four individual
cards uses 36 visible entries; reaching 36/36 never claims full-round completion.

**READY WITH KNOWN LIMITATIONS:** affected validation passed; the existing bundle
warning and skipped broader checks remain documented. Repeatable browser commands
are in Documentation.md. Tournament closure requires a separate contract decision;
no closure action is introduced here.
