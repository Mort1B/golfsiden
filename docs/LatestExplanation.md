# Scoring resume and simpler page composition

The main Score navigation now returns to the lowest-numbered hole without a
persisted score for the last selected tournament, round, and player/team. For
example, a card with scores at holes 1 and 3 resumes at hole 2. A fully registered
writable card opens its summary for confirmation. A fresh server read must finish
before this decision; failed reads offer retry without choosing from stale cache.
Explicit hole and summary links, Back/Forward, manual navigation, and quick card
switching preserve their intentional selection. Saving and background refresh do
not advance the hole. Locked/restricted reads retain visible-hole selection.

The active hole or summary appears immediately after owner/tournament/round
identity and essential notices. Existing labeled tournament, round, and owner
selects expand under “Bytt turnering, runde eller spiller/lag”. Hole/view controls,
quick card switching, and totals remain available below. Tournament creation has
more space above the list filters. Tournament standings no longer display the
introductory explanation paragraphs; qualification, provisional, and visibility
information remains.

A narrow session provider remembers only validated IDs. It clears them before a
new identity renders, retains them across same-user refresh, and leaves the router
mounted so account creation and invitation acceptance retain their success state.
The memory lasts only for the mounted application session; explicit URLs retain
selection across reloads. TanStack Query still owns all card and permission data,
using the existing canonical keys. Score writes, confirmations, authorization,
handicap calculations, backend contracts, and database schema are unchanged.

Validation:

- 388 frontend tests passed across 65 files, including eight new route/provider
  regressions for gaps, stale reads/retry, explicit history, team/round retention,
  restricted locked cards, complete cards, failed saves, and identity transitions.
- Type checking, ESLint, production build, browser TypeScript, and diff whitespace
  checks passed. The existing large-bundle warning remains (approximately 611 kB
  minified); bundle optimization is outside this step.
- Chrome scoring checks cover 320px, 390px, and 1280px: long populated and expanded
  states, keyboard disclosure, first-gap resume, retained team/round, quick
  switching, Back/Forward, loading, retry, failed-save guards/discard, all-scored
  summary, empty owners, list spacing, and removed leaderboard copy. Controlled
  endpoint fixtures exercise these states; console and unexpected network
  failures are checked. Screenshots are under `/tmp/golf-scoring-*.png`.
- The two existing real-API lifecycle browser regressions passed against migrated
  and seeded disposable PostgreSQL, including scoring corrections and final-nine
  hide/release/re-hide. Creator and invitation browser completion passed, with
  explicit assertions that the invitation receipt and joined success remain.
- Signed-in tournament creation and tournament-results wording browser regressions
  passed. The results case required a fresh seed after lifecycle validation had
  locked the original rounds; it passed after resetting the disposable database.
- Read-only scoring review found an initial router-remount defect. It was fixed,
  regression-tested, and re-reviewed with no remaining findings.
- Backend Rust/database test ladders were not run: no backend, migration, or API
  contract changed. PostgreSQL migration and seed commands ran for browser setup.

Release verdict: **READY WITH KNOWN LIMITATIONS** (existing bundle warning and
in-memory-only resume context). Tournament leaderboard live-update diagnosis is
still the separate next candidate; this iteration does not claim to fix it.
