# Organizer summary and clearer round navigation

Tournament administration now starts with “Dette trenger oppfølging”, a concise
summary ordered by round number. Organizers can see rounds ready to open,
complete, or lock; missing-flight counts; setup work; and scorecards needing
attention. Each entry leads to the existing round control or scorecard.
“Rundestyring” replaces the visible “Livsløp” label, while existing `#lifecycle`
links remain valid.

For example, an open round with one incomplete card and one complete,
unconfirmed card shows those as separate tasks. Only the second card gets a
confirmation shortcut. A team scorecard counts once, preserving its tagged team
owner rather than treating its members as separate cards.

The summary reuses existing private readiness endpoints and canonical query keys.
Opening requires the active tournament and the server's readiness result;
completion and locking use the corresponding server flags. Full projection,
matching identity/status, and settled authoritative reads are required before
suggestions appear. Loading, failed/paused reads, and authority refresh suppress
suggestions. Exact-admin membership remains the workspace boundary; losing that
membership removes the summary and controls. Summary links never mutate state.

There is one applicable readiness read per non-locked round, shared with mounted
lifecycle/scoring consumers. No backend, migration, new cache, or notification
service was introduced. “Oppdater oversikten” refreshes membership, tournament,
rounds and their readiness queries. Empty/all-locked states remain calm and link
to tournament status where appropriate.

Review identified that an already-selected setup editor could stay collapsed
when its summary link was followed again. Navigation activation now reopens that
exact course or pairing editor and focuses its section, without discarding drafts.

## Validation

- All 380 frontend tests passed. Unit/interaction coverage includes task ordering, authoritative readiness,
  missing flights versus teams, tagged confirmation owners, incomplete cards,
  empty/all-locked states, refresh/failure/retry, paused requests, restricted or
  mismatched projections, live disconnect/reconnect, and membership changes.
- Typecheck, lint, production build, and browser-test TypeScript compilation
  passed. Browser TypeScript now explicitly
  includes Vite's environment types alongside Node types; strictness is unchanged.
- The dedicated Chrome organizer scenario passed, exercising mixed round states, long
  content, keyboard navigation, exact destinations, repeat visits after collapsing
  both editors, refresh errors/retry, empty/all-locked views, and membership loss
  at 320px, 390px and 1280px. It asserts no API mutations from summary interaction.
  Screenshots are in `/tmp/golf-organizer-*.png`.
- Both existing lifecycle Chrome scenarios passed, covering real opening, score entry and SSE,
  confirmation, completion, cross-session corrections, locking, permissions and
  final-round visibility, plus loading/error/empty layouts. Their two confirmation
  locators distinguish the original control from the new summary shortcut.
- A disposable rootless PostgreSQL 17 container on port 55432 was migrated and
  seeded for real browser validation. Backend/database ladders are not applicable
  to this frontend-only change; no backend or migration source changed.

## Scope and delivery

Verdict: **READY** for this bounded step. Read-only review found one repeated-link
issue, which was fixed and browser-tested; final review has no remaining findings.
No applicable validation gate was skipped.

The existing frontend large-chunk build advisory remains. No administration
transition rule, automatic team setup, notification, scoring-page navigation,
or leaderboard aggregation changed. Candidate 3 (scoring flow and spacing) and
candidate 4 (leaderboard troubleshooting) remain queued; production deployment
is separate from Git publication.
