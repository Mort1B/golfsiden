# Plans

`PLANS.md` contains only the active implementation step and a short work queue.
Completed behavior belongs in `Documentation.md`; durable technical decisions
belong in `ARCHITECTURE.md`.

## Active step

None. The UI gap audit below is planning-only; implementation has not started.

## Next candidate

### Round lifecycle controls and readiness

**Goal:** An exact tournament administrator can take an existing round through
`draft -> open -> completed -> locked` entirely in the application, understand
what blocks each transition, and reach the existing flow that resolves it.

**Evidence from the 2026-09-06 source audit:**

- `backend/src/api/rounds/lifecycle.rs` and `completion.rs` already expose
  readiness reads and authorized open, complete, and lock mutations.
- `frontend/src/api/roundLifecycle.ts` implements opening/readiness decoding and
  an open wrapper, but no UI invokes it. Complete and lock wrappers are absent.
- `TournamentManagementSections.tsx` exposes tournament start and final-round
  visibility, but lists round lifecycle states without transition controls.
- `RoundPage.tsx` contains course metadata and teams, without lifecycle actions
  or an administrator navigation link.
- Completion decoding and scorecard confirmation already exist. The missing
  surface is an administrator readiness overview, not a new confirmation model.

**Scope and ownership:** Frontend API, focused lifecycle components/hooks,
management integration, a round-page administration link, relevant tests and
documentation. Use the existing management Lifecycle section with one selected
round; preserve the selected round in the URL for direct links and refresh.
The primary agent owns the bounded step and integration. No backend, migration,
scoring-rule, or lifecycle-policy change is expected; record any discovered
contract defect separately before expanding this step.

**Required behavior:**

1. Resolve the exact tournament membership before exposing administrator
   controls. Link from the round page to that round's management controls for
   exact admins. Other members retain their existing read access. Global account
   roles must not confer tournament administration.
2. For a draft round, read `GET /api/rounds/{id}/pairing-validation` and show
   actionable readiness issues, including affected entrants, teams and flights.
   Link to tournament start, entrants/invitations, course configuration, or
   pairings as appropriate. Keep the round selection when navigating to setup.
   Allow `Åpne runden` only when authoritative readiness is true. Explain before
   confirmation that opening freezes pairings/course facts and captures handicap
   snapshots; call the existing POST only on explicit submission.
3. For open/completed rounds, reuse the existing completion-validation decoder
   and identity-scoped query key. Show required player/team cards, scored versus
   required holes, confirmation state, and server readiness. Link each blocker
   to its exact existing scoring/confirmation flow; keep read-only scorecard
   links separate and preserve tagged historical owner identity.
4. Offer `Fullfør runden` only for an open round with `ready_to_complete === true`.
   Explain that completion makes results count while corrections remain possible.
   Offer `Lås runden` only for a completed round with `ready_to_lock === true`,
   with explicit confirmation that ordinary corrections then become unavailable.
   A correction after completion removes confirmation and must block locking
   until that card is confirmed again. Locked rounds show a read-only outcome.
5. Add CSRF-protected complete/lock API methods with runtime response validation,
   expected round/tournament identity and expected resulting status. Preserve
   existing endpoint contracts; never synthesize successful lifecycle state.
6. Handle loading, empty rounds/owners, failed reads, background refetch failures,
   pending mutations, success, and retry. Disable actions when required authority
   or readiness is unavailable, stale after failure, or inconsistent with the
   current round. Prevent duplicate submissions. On a conflict, refetch round
   and readiness and explain the new blocker; opening currently returns a generic
   conflict, so recover detailed reasons through its readiness read. On uncertain
   network outcomes, reconcile server state before permitting another attempt.
7. Reconcile mutation results and invalidate affected round detail/list,
   tournament/roster lock state, pairings/readiness, score access/cards and both
   leaderboard views using established private query ownership. Handle another
   admin's transitions and score corrections through existing SSE/refetch flows;
   do not depend on SSE alone for local mutation success. Preserve dirty course
   and pairing drafts through the existing explicit conflict behavior.
8. Keep final-back-nine release/re-hide independent from completion and locking.
   Do not treat redacted/null member readiness as administrator readiness or
   leak hidden progress when membership changes.

**Invariants:** All root product invariants remain unchanged. The server remains
the authority for readiness, transaction locking, snapshots, score ownership and
authorization. No reopen/unlock, locked-score correction, automatic final release,
automatic tournament completion, or one-open-round restriction is introduced.

**Validation:**

- Cover API request/CSRF and malformed/wrong-identity/wrong-status responses;
  user-visible transition controls, confirmations/cancel, blockers and links;
  duplicate prevention, failed mutation reconciliation, unauthorized access,
  stale/refetch/SSE transitions, logout/account changes and tournament isolation.
- Exercise individual, scramble and foursomes owners; missing course/flight/team
  setup; incomplete and unconfirmed cards; empty required owners; completed-card
  correction/reconfirmation; locked state; and hidden final visibility.
- Run frontend test, typecheck, lint and build ladders. Run the existing backend
  and PostgreSQL lifecycle/authorization integration coverage to verify the
  reused contracts; if backend changes become necessary, first bound them and
  apply the full affected backend/database ladder.
- Use a real browser at 320-390px and desktop against a disposable local database:
  start a tournament, resolve readiness, open, score/confirm, complete, correct,
  reconfirm and lock. Include loading/error/empty/populated/long-name states,
  keyboard/focus handling, a second-admin race and non-admin access. Inspect
  console/network behavior and verify that locking does not release the final.
- Obtain read-only review of lifecycle, authorization and cache synchronization.
  Record exact skipped checks and blockers; never imply unrun coverage.

**Documentation and stop condition:** Update current operator flows in
`Documentation.md` (including its overview, which currently implies the missing
controls exist), relevant API inventory/boundary details in `ARCHITECTURE.md`,
and the completed iteration in `LatestExplanation.md`. Close and publish only
this step when implementation, review and applicable validation are resolved or
explicitly recorded. Do not begin the later queue without approval.

## Later

- **Member round details and navigation:** Replace `RoundPage.tsx`'s legacy
  team-schedule presentation with the member-readable pairings aggregate.
  Show flight names, tee times, starting holes and members, and separate
  score-owning teams for scramble/foursomes. Individual rounds should show their
  flights rather than a misleading empty team section. Add round-scoped links to
  scoring and results, unconfigured-course messaging, and deliberate retry/empty
  states. Validate all three formats and mobile/desktop layouts. No authority
  may be inferred from schedule facts.
- **Flight progress:** Build a member-visible progress overview using stored
  flight membership and visibility-projected owner progress. Count shared team
  cards once; do not infer hidden final completeness from flight totals. Reuse
  the first step's admin readiness where appropriate. Bound any missing read
  contract before implementation and validate redaction plus live updates.
- **Tournament closure contract decision:** Tournament start is implemented,
  but `backend/src/api/tournaments.rs` has no complete/archive action. Define
  whether and when a tournament closes, required round states, invitation and
  correction behavior, archive visibility, and any reversal policy before
  planning backend/database enforcement and then UI. This is not a missing
  button over an existing endpoint.
- Configurable tie-breaks, public share links, offline scoring, and account
  recovery.
- Decide whether PostgreSQL should enforce at most one open round per tournament;
  reads currently select the highest-numbered open round deterministically.
- After roadmap completion, performance work, and security review, decide the
  contracts for four-ball, Stableford, and match play separately.
