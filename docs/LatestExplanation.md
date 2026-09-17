# Keep tournament switching available for match-only results

L3 restores the **Turnering** selector on global **Resultater** when the selected
trip contains only match play. Previously that branch returned the match-results
page before rendering the shared controls, forcing a user with several tournaments
to leave results to switch trips.

The existing native tournament select is now a shared component. The global
match-only branch supplies it to `MatchResults` through an optional composition
slot, visible above populated, loading, error and empty match results. Dedicated
tournament match/history routes retain their existing navigation and mixed trips
retain the link back to gross/net overall results.

Switching uses the existing leaderboard URL builder, clearing the old round and
player selection. Gross/net routes continue to canonicalize with replacement,
so Back restores the previous match URL, including a private player filter.
The match-only branch remains before that canonicalization and still avoids
inapplicable gross/net queries. No query keys, authorization, scoring rules,
backend contracts or database schema changed.

For example, an account viewing one player's match history through `/leaderboard`
can choose a mixed trip, inspect its round or overall results, and use Back to
return to the same player's match results. Selecting the match-only trip anew
shows its full permitted table without carrying over another trip's player ID.

## Validation

- Regression proof: four new tests failed against the original selector-free
  branch; the dedicated-route control passed. All five now pass, along with all
  nine existing private-result regression tests. The tests cover switching,
  canonicalization/history, player filtering, loading/error/empty availability,
  match-only query suppression and the unchanged dedicated mixed-results link.
- Full frontend suite: **613 tests across 107 files passed**. Strict type checking,
  ESLint and the production build passed. The existing 500 kB bundle-size warning
  remains in the performance queue.
- Real Chrome: **two scenarios passed**, each across 320×600, 390×844 and
  1280×900. A real account belongs to match-only and mixed tournaments. Checks
  cover direct entry, switching both ways, Back/Forward, previous round/player
  removal, private-player restoration, both result scopes, dedicated mixed links,
  long names, no horizontal overflow, and reachable 44-pixel tournament controls.
  The second scenario verifies switching during loading, error and empty states.
  Screenshots were inspected. No unexpected console/network failures or
  match-only gross/net requests were observed.
- The browser loading/error/empty states use intercepted match-table responses;
  normal navigation, membership, rounds and results use the disposable real API.
  Initial test-only failures were corrected: Testing Library role-option types,
  exact existing overall heading names and awaited delayed-route cleanup.
- Read-only strategy and final source review found no issues. `git diff --check`
  passed; all affected production files remain below the 400-line limit.
- Backend/PostgreSQL ladders were not rerun: this step changes frontend composition
  only, with no backend or migration edits. Real browser flows used the existing
  disposable local API/database. No production environment was modified.

Evidence logs: `/tmp/l3-red.log`, `/tmp/l3-green.log`,
`/tmp/l3-frontend-full.log`, `/tmp/l3-navigation-final.log`,
`/tmp/l3-typecheck.log`, `/tmp/l3-lint.log`, `/tmp/l3-build.log`,
`/tmp/l3-browser.log` (initial test failures) and `/tmp/l3-browser-final.log`.
Screenshots use `/tmp/l3-<state>-<width>.png`.

**READY.** L3 is complete. The next candidate is the existing intermittent
frozen-page/offline-return validation investigation. Performance measurements
and the wider security review remain later work.
