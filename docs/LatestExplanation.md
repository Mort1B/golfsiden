# Latest iteration: Member round details and navigation

Round details now show the member-readable flight aggregate instead of legacy
team schedules. Flights show stored members, start times and starting holes,
with explicit missing-schedule states. Scramble and foursomes retain separate
score-owning teams. Individual rounds no longer show a misleading empty team
section; legacy individual groups remain explicitly separate.

Result links preserve the exact tournament and round. A neutral scorecard-summary
link appears after opening; the destination resolves owners and permissions.
Drafts omit it because the scoring page excludes drafts and would otherwise
select another round. Exact tournament admins retain their round-specific
management link. Missing course/tee setup is explained.

## Boundaries and review

The page reuses the decoded member pairings API, canonical user-rooted cache key
and existing SSE invalidation. It is keyed by account and round. Failed reads
hide retained private data and offer retry; mismatched status/format between
round and pairings reads requires refresh. Flight schedules never grant score
authority. No score/progress reads, mutations, backend contracts or database
schema were added. Final visibility and historical ownership are unchanged.

Final read-only review found no concrete defects. Suggested exact-admin link and
cross-account cache tests were added. Flight progress and tournament closure
remain outside this iteration.

## Validation

- Frontend: all 270 tests in 45 files, strict typecheck, lint and production build
  passed. Fourteen new page tests cover three formats, lifecycle navigation,
  missing setup/groups, legacy groups, pending reads, failed refresh/retry, mixed
  lifecycle snapshots, invalid routes, exact admin links and cache isolation.
- PostgreSQL 17: migrations and seed passed in disposable databases. The real API
  backed Chrome checks for all three formats, member presentation, tournament
  start, round opening and live member-page refresh.
- Chrome: the opt-in round-details suite passed at 320px, 390px and 1280px,
  including navigation touch targets and no horizontal overflow. Mobile and
  desktop screenshots were inspected. Scoring and results navigation retained
  the exact round. Detail requests did not fetch legacy teams, score access,
  completion, scoring cards or score mutations.
- Scoped browser injection covered pending, empty, long unbroken flight names,
  missing schedules, denied reads and retry. Happy-path diagnostics had no
  uncaught page/console errors, unexpected error responses or failed requests.
  The intentional denied response was checked separately. Initial harness
  errors (missing tournament-start version and a loading interception race) were
  fixed before the successful complete rerun on a fresh seed.

The existing Vite warning remains: 570.31 kB minified, 165.34 kB gzip. Backend
unit/Clippy and full PostgreSQL suites, the separate lifecycle browser suite and
deployment/recovery drills were not repeated: only frontend presentation changed.
Browser evidence uses the local development proxy, not production.

## Example and verdict

A foursomes member sees their four-player flight's start hole and separate
two-player score-owning teams. Opening makes the exact-round scorecard link
appear live; schedule facts do not determine edit access.

**READY WITH KNOWN LIMITATIONS:** affected checks pass; the existing bundle
warning and skipped broader checks above remain explicit. Browser run instructions
are in Documentation.md. Stop before the next queued step.
