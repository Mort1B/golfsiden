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

### Next candidate: preserve tournament context

**Proposed; planning only, awaiting implementation approval.** Preserve the
separately planned security assessment. This is the next product repair, followed by
the validation-only candidate below. Execute one bounded step at a time.

Goal: a golfer or organizer working in tournament B can move between its rounds,
Score, Resultater, and administration without unexpectedly landing in tournament A.

Source evidence to reproduce: `frontend/src/ui/AppShell.tsx` currently links to
bare `/score` and `/leaderboard`. Score has separate session-local resume IDs in
`useScoreWorkspaceData.ts`; results selects from URL parameters in
`LeaderboardPage.tsx`. Round links already use `scoringSearch` and
`leaderboardSearch`. These are source observations, not a verified browser failure.

Scope: first reproduce with one account belonging to two tournaments, selecting
the non-default tournament. Trace main navigation, tournament/round pages,
management, private history/read cards, and singles-match routes. Repair only the
selection and link boundaries necessary to retain validated context. Use existing
typed URL builders, router ownership, identity-scoped queries, and score-resume
semantics. No backend contract, migration, scoring rule, or visual redesign is
planned; a need for one becomes a separate bounded candidate.

Required behavior:

- An explicit valid route/URL selection wins over remembered selection. Navigation
  within a tournament carries its validated ID and a compatible round ID; a round
  from another tournament must never travel with it. Resolve round-only routes
  through authorized loaded data before remembering their tournament.
- Main Score and Resultater actions retain the current tournament. From a neutral
  page such as Profil, retain only the last validated context in the current
  account session. Keep the tournament list and an explicit switch accessible;
  entering the list alone must not silently select another tournament.
- Switching tournaments clears incompatible round, owner, match, hole and
  destination-specific state. Keep result scope/metric where valid. Never copy
  score-owner or match identity into a destination where it has no meaning.
- Main Score navigation keeps fresh-authority resume behavior and selects the
  first missing persisted hole or complete-card summary within the selected
  tournament/round. Explicit score URLs retain their requested hole/view. Do not
  turn contextual navigation into an explicit URL that accidentally skips resume
  freshness checks. Singles match play keeps its existing dedicated entry flow.
- Refresh, copied canonical URLs, login return, and browser Back/Forward preserve
  explicit selections. A bare URL in a new application session uses existing
  authorized defaults; no new persistent storage is required. Sign-out/account
  change clears remembered context. Context is never proof of permission.
- Malformed, stale, inaccessible, or cross-tournament identifiers cannot enable a
  mismatched query or display stale private content. Revalidate authority, clear
  invalid context, and show an appropriate denied/empty/selection state or the
  existing authorized fallback. Loading and failed reads cannot establish new
  remembered context; delayed responses from A cannot replace B.
- Pending score edits, offline queues and conflicts retain their exact targets
  and existing navigation guards. Never redirect or replay a queued mutation into
  another tournament while preserving navigation context.

Invariants: preserve every root product invariant, administrator-managed teams,
session/private-cache isolation, server-owned handicaps and gross/net/Stableford
results, hidden-result projections, round locks and audited corrections. Add no
global copy of server state or new authorization authority.

Validation: create a failing two-tournament reproduction before repair, then
exercise both directions between Score and Resultater, management/round entry,
Profil return, switching, deep links, reload, login return and Back/Forward. Cover
admin, scorer and player access; a nonmember; removed membership; locked/no eligible
rounds; empty/loading/error states; held late responses; pending/offline edits;
and existing stroke-play, Stableford, four-ball and singles-match destinations.
Assert the visible tournament/round, canonical URL, request targets and mutation
targets, not just screenshots. Add focused behavior regressions, run the full
frontend ladder from [the workflow](AGENT_WORKFLOW.md#validation-ladders), and
repeat relevant Chrome route/resume/offline/identity browser regressions against
the real local API with disposable fixtures. Require read-only review of routing,
cache isolation and resume/queue interactions. Run backend/PostgreSQL ladders if
their implementation boundaries change; stop and re-bound that scope first.

Stop: after the reproduced context issue is fixed, reviewed, validated and
documented in `Documentation.md`, `ARCHITECTURE.md` if ownership changes, and
`LatestExplanation.md`, publish the bounded repair. Report unrelated findings as
queued candidates; do not begin the broader audit automatically.

### Following candidate: functionality, design and Chrome deployment validation

**Proposed validation-only step; awaiting approval after the context repair.**

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
