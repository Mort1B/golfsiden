# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. See the [latest explanation](LatestExplanation.md) for the completed iteration.

## Next candidate

**Wider application and operational security review (awaiting approval).** Define
and review a bounded assessment of authorization, session handling, private-data
boundaries and production operations before implementation. Keep findings separate
from repairs; prioritize concrete reproducible risks and validation evidence.

## Later queue

No additional implementation step is currently approved.

### Next candidate: functionality, design and Chrome deployment validation

**Proposed validation-only step; awaiting approval.**

Goal: establish whether the existing product is usable for the friends' deployment
and identify reproducible blockers without adding features or silently repairing
unrelated defects. Use disposable accounts/data and a production frontend build
served through the local production-like proxy/API topology. Do not deploy or
mutate real user data as part of this assessment.

Scope and acceptance matrix:

| Boundary | Required evidence |
| --- | --- |
| Account to tournament | Sign-in/out, invitation join, login return, account switching, expired session, authorized tournament selection; no stale private data or redirect loops |
| Organizer setup to playable round | Existing creation, roster, saved-course selection/configuration, administrator team assignment, pairing/access and round opening; controls discoverable on mobile and desktop |
| Score to results | Save/retry, confirmation, existing formats, gross/net and native Stableford presentation, private history/read cards, match results; API-confirmed persistence and a second session receiving live updates |
| Connectivity to authority | Pending/offline delivery, reconnect, conflicts, duplicate submission protection, tab return and overlapping refresh; no lost edits or stale permissions |
| Lifecycle to privacy | Complete/lock, rejected ordinary writes, authorized correction, tournament completion/archive and hidden-result visibility; existing public share remains appropriately restricted |
| Layout to interaction | Navigation and current tournament are clear; primary actions visible; loading, empty, populated, failure/retry, disabled/locked and long-content states remain usable |

Chrome/design acceptance: run installed Google Chrome with the existing Playwright
`channel: 'chrome'` configuration; record version, OS, viewport and tested commit.
Validate 320px and 390px phone widths, a short phone viewport, and 1280px desktop,
plus both sides of the actual navigation breakpoint. Check 200% zoom, keyboard
navigation/focus, semantic labels, error announcements, readable contrast,
44x44px primary touch targets, and no unintended page overflow, overlap or content
hidden beneath fixed navigation. Verify profile, tournament switching, saved
courses and administration are reachable at desktop widths. Preserve the existing
visual language; propose focused usability repairs only where evidence warrants.

Exercise native form validation (including username patterns), numeric inputs,
dialogs, history, reload, storage, SSE and retry behavior in Chrome. Capture
screenshots plus DOM/state assertions, console errors and failed requests, with
expected authorization failures distinguished from defects. Desktop Chrome mobile
emulation is layout evidence, not Android device evidence: smoke-test the primary
journey on Android Chrome if available and record the exact blocker if unavailable.

Validation/output: map existing `frontend/e2e/*.browser.ts` suites to the matrix,
run relevant suites with their required fixtures/environment flags, and add only
missing boundary regressions. Count skipped tests as untested. Run the full
frontend ladder and affected backend/PostgreSQL ladders per the workflow; collect
real-API evidence rather than relying solely on mocked browser tests. Produce a
reviewed report with scenario, role, viewport, expected/actual behavior, evidence,
severity and reproduction for each finding. Retain the active security review's
unresolved deployment blockers in the readiness decision.

Stop: document a `READY`, `READY WITH KNOWN LIMITATIONS`, or `NOT READY` verdict
for the tested deployment setup. Wrong tournament/owner writes, private-data
exposure, lost scores, broken critical journeys, or untested critical boundaries
prevent `READY`; cosmetic limitations must be explicit. Propose one bounded repair
for the highest-priority remaining defect and wait before implementing it. Update
current documentation only where verified behavior needs correction and record
the assessment in `LatestExplanation.md`; this verdict does not authorize deployment.

No automatic opponents/byes/brackets, team match play, extra holes, new scoring
rules, public match sharing, cold offline launch or background sync is included.
