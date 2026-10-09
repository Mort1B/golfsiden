# Fantasy and player-claim recovery

Three verified reliability defects are repaired without changing scoring rules,
authorization, privacy contracts, APIs or single-use claims.

Fantasy recovery now belongs to the tournament/account/session workspace above
query-driven branches. Local picks, captain, exact uncertain request and own
acknowledged receipts survive SSE clearing, failed queries and round switching.
Private server projections still clear immediately. Navigation/unload guards
protect unresolved input; the state remains in memory and does not create an
offline write queue or survive a confirmed reload/session replacement.

Write and refresh outcomes are separate. An acknowledged write followed by a
failed read keeps its receipt and exposes read-only refresh retry. Shared action
ownership covers selection and administrator controls. Reconciliation reconstructs
the original target from identifiers, checks query state, and explicitly reads
newly enabled activation dependencies. A failed read never invokes write-failure
handling or repeats an acknowledged write.

A lost player-registration response now triggers a session read. Only the
expected prepared player and chosen username can establish recovered success;
canonical-session and mounted-lifetime checks protect a concurrent login. Without
a matching session, the page explains that the account may already exist and
opens ordinary login with the chosen username. Password/claim secret are cleared,
and the UI no longer tells an already-linked player to obtain another claim.

For example, a lineup can commit revision 2 and then fail to refresh: its receipt
remains visible, and **Prøv oppdatering igjen** sends GETs only. If registration
commits without delivering its cookie, ordinary login reaches the original
prepared player; registration is not repeated.

The initial parent regressions failed before production edits. Final validation:
134 frontend files / 996 tests, typecheck, lint and production build passed;
12 Chrome scenarios passed against a fresh local PostgreSQL database, including
real commits with deliberately broken responses both with and without cookies.
Read-only review findings about parent action lifetime, switched targets,
definitive rejected replay and newly enabled reads were resolved. See the
[exact evidence, commands, screenshots and limits](validation/reliability-2026-10-09/README.md).
No hosted deployment or physical-device acceptance was performed.
