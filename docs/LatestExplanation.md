# Navigation keeps the selected tournament and round

The bounded context repair replaces bare main-menu destinations with validated
session-local tournament and round selections. The controlled Chrome reproduction
used an account with two active tournaments: opening the non-default tournament's
round and choosing **Resultater** previously selected the other tournament.
The same case now keeps the tournament and round.

Route workspaces publish IDs after their existing typed queries succeed. The
private shell retains those hints through **Profil** and the tournament list.
Tournament-only pages keep an earlier selected round within the same tournament,
checking the loaded rounds when available. Switching tournament, an explicit
invalid round, a removed round, or authorization denial cannot carry that old
selection into a replacement workspace. Old route/account callbacks are ignored.
Malformed routes and module-loading failures restore ordinary navigation;
contextual links wait while a new target is still unresolved.

Main **Score** navigation uses `tournament`, optional `round`, and `resume=1`.
It keeps the existing fresh-authority checks before choosing the first missing
persisted hole or complete-card summary. Explicit owner/hole/view URLs, login
return and Back/Forward retain their exact selection. The score-owner resume hint
is usable only in its original tournament and round. Explicit results-control
changes commit synchronously so a rapid scope change followed by a tournament
selection cannot reuse the previous scope. The new memory contains no
scores or permissions and does not change score/match queues, mutation targets,
API contracts, backend rules or database schemas.

For example, after scoring hole 1 in tournament B's earlier round, a golfer can
visit results, switch to Brutto, visit Profil and return to Score. The application
keeps B and that round, refreshes authority and opens hole 2. Tournament A receives
no score mutation. Returning through B's tournament overview also preserves the
earlier round, even when a later round is open.

Read-only review identified and resolved invalid-route navigation lockout,
retained context after delayed stroke/match authorization denial, and loss of an
earlier round through tournament-only pages. The final review found no remaining
concrete blockers in this scope. The broader functionality/design/deployment
assessment is separate and has not been performed.

## Validation

- `npm --prefix frontend run test`: 751 tests across 116 files passed.
- `npm --prefix frontend run typecheck`, `npm --prefix frontend run lint`,
  `npm --prefix frontend run build`, and
  `frontend/node_modules/.bin/tsc -p frontend/tsconfig.browser.json --noEmit`: passed.
- `npx playwright test --config playwright.routes.config.ts` from `frontend/`:
  all 42 production-build Chrome cases passed, covering route chunks, recovery,
  pending-write guards, identity transitions, result privacy, lost/frozen/visible
  returns and overlapping authority refresh. These cases use controlled API
  fixtures, including held responses and simulated failures.
- Real-API production-preview matrix: all 32 cases passed, covering context,
  stroke play, Stableford, four-ball, singles match play, private results,
  confirmation, locks and offline delivery. Subsequent final changes were limited
  to the rapid results-control race and more precise test readiness checks.
- The original rapid results-switch regression passed three consecutive runs
  after the control fix; no extra wait was added to that sequence.
- Final context/results/private-read suite: all 15 cases passed on the final
  production build, including a real separate-account nonmember denial and
  the strengthened post-SSE private-result baseline checks.
- Documentation links, `git diff --check`, and changed production-file line limits:
  passed. Independent read-only review has no remaining concrete blockers.

Chrome was Google Chrome 153.0.8010.36 on Linux. The new context journeys use
320x600, 390x900 and 1280x900 viewports and assert URLs, visible selection, exact
mutation destinations, persisted scores, console/network health, navigation
hit targets and overflow. Phone and desktop screenshots were inspected. Test
accounts and scores belong to the disposable `golf-context-pg` database; existing
security-review services/data were not used.

The first development-server two-tab lost-response test timed out; its isolated
production-preview rerun passed without a test or queue-code change. One final
matrix attempt could not connect because the preview process had terminated;
that infrastructure failure is recorded separately from the restarted run.
An earlier real-API run passed 28/32 cases. Investigation distinguished the actual
rapid results-control race from test setup prerequisites: confirmation must be
ready before holding its refetch, sign-out must finish before navigating to login,
and the private projection must be restored after SSE opens before injecting a
later transient failure. The unchanged baseline reproduced the confirmation
setup failure; the rapid-switch and private-history failures reproduced against
the changed build and were investigated separately. The original privacy,
denial, retention and queue assertions remain intact.

No backend/migration code changed, so the Rust unit/Clippy and complete PostgreSQL
ladders were not rerun. The real API used a fresh PostgreSQL database initialized
by the existing migration runner; browser fixtures created their own tournaments.
Physical Android Chrome, other browser engines, Caddy/TLS deployment, and the
broader functionality/design assessment remain untested in this bounded repair.
This iteration makes no deployment-readiness claim.

The bounded context repair is complete and reviewed. The next product step is the
separate functionality, design and Chrome deployment assessment in `PLANS.md`.
