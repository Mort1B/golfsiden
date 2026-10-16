# RELEASE-RECOVERY — award exit and four-ball/Fantasy locking

One bounded source hardening step, 16 October 2026. Both findings were verified
against the clean starting `main` checkout before implementation. No unrelated
worktree changes existed. Schema stays 39; there is no migration, dependency,
scoring-formula, production configuration or hosted change.

Both requested fixes pass their targeted tests and real-browser acceptance.
**The full PostgreSQL command is not green:** one unchanged native-collation
assertion fails on this disposable Debian database. Details and the separately
completed remaining targets are below. No unrelated repair is included.

## Verified failures and repairs

**Award denial trap (P2).** `useAwardActions` retained uncertain commands after
403/404, correctly preserving uncertainty, but `AwardsProvider` only discarded
unsent drafts. Its route, before-unload and scoring/logout guards therefore had
no deliberate exit when authority was gone. The provider is already above the
actual management parent's query/access branches; that ownership remains.

**Oppgi lokal gjenoppretting og forlat siden** explicitly explains that local
abandonment does not undo accepted server effects and denied replay does not
prove original noncommit. Outside in-flight dispatch it clears local drafts and
uncertain requests, preserves own acknowledged receipt handling and leaves for
the tournament list. It releases its own guards and sends no request. Exact
replay remains a separate deliberate action; receipt validation, request identity,
session fencing, read-only acknowledged reconciliation and authorization stay.

**Four-ball/Fantasy completion dead end (P2).** Sporting confirmation correctly
accepts one numeric partner per hole plus acknowledged blanks, while canonical
Fantasy hole evaluation keeps a side pending if either input is unentered.
Previously normal locking checked sporting completion/confirmation alone and
could make ordinary repair unavailable permanently.

`fantasy::completion` now reads canonical source facts and calls the existing
Fantasy hole evaluator, batching partner names. It emits owner-scoped partner /
hole messages. Global completion readiness stays visibility-projected; authorized
scoring GET/confirmation responses carry only their side's issues, including for
non-admin scorers on hidden final rounds. Member/history cards erase the field.
The organizer receives correction/reconfirmation links and `ready_to_lock=false`.
The lock endpoint recomputes under the existing round mutation lock and returns
`409 round_fantasy_inputs_unresolved`. Sporting confirmation and completion still
succeed. Deliberate actual-score/pickup entry invalidates confirmation; resolving
blanks alone is insufficient until the scorecard is confirmed again. No scores
are invented or blanks converted automatically. Locked protection stays intact.

## Regression evidence

- Actual `TournamentManagementPage` parent: 403 and 404 tests retain the exact
  signed-in account while admin membership disappears during dispatch. The editor
  unmounts into **Ingen tilgang**, uncertainty stays guarded, abandonment exits,
  guards clear and only one award write occurred.
- Real PostgreSQL + Chrome: exact membership is changed to viewer through local
  test SQL during a held award request (`UPDATE 1` asserted). The actual API then
  returns 403. Safe exit and logout succeed; no extra award mutation occurs.
- Committed-original control: the original API write commits but its receipt is
  replaced with a synthetic 503. Manual exact replay after real admin revocation
  receives 403; both request bodies are identical. Abandonment leaves exactly
  one immutable server history row. The initial-denial control leaves zero rows.
- Integrated Fantasy: one partner scores all 18 holes, the other stays blank.
  Sporting confirmation and completion succeed; Fantasy remains pending. Both
  disabled UI locking and direct lock 409 are asserted. Partner fields are then
  deliberately resolved, locking still fails until reconfirmation, Fantasy settles
  and normal locking succeeds. PostgreSQL additionally rejects a current-version
  write specifically with `RoundNotEditable` after lock.
- Hidden-final scorer: global completion issues remain erased while the authorized
  own-side scoring warning identifies all unresolved holes. Read projections
  erase it. Decoders reject foreign-team, duplicate, wrong-kind and concealed
  issues and inconsistent lock readiness.

## Validation

Final counts and command results are retained in [checks.json](checks.json).
The complete affected workflow ladders ran: Rust formatting, all-target workspace
unit tests, all-feature strict Clippy, all-target database-feature tests against
disposable PostgreSQL 17.11, migrate, seed, and frontend tests/types/lint/build.
The new browser file also receives a separate strict TypeScript check because
normal frontend typecheck covers `src`, not `e2e`.

Chrome checks at 320, 390 and 1280px cover empty awards, held/pending dispatch,
access-denied recovery, long partner names, sporting confirmation, guarded lock,
hidden-final scoring warning and resolved/locked state. Checks assert no horizontal
overflow, visible region buttons at least 44px high, expected request counts,
server effects and logout. Screenshots were visually inspected. Existing award
browser controls cover exact replay and acknowledged read-only retry, independent
awards, hidden-member privacy and native Stableford scores. Existing four-ball
controls cover populated/offline/conflict/storage recovery, loading and failed
reads. Expected injected failures/authorization statuses are distinguished from
unexpected page/console/network failures.

Selected inspected evidence: [denied award at 320px](denied-initial-320.png),
[denied exact replay at desktop](denied-replay-1280.png),
[pending dispatch](pending-initial-390.png),
[hidden-final scorer](hidden-final-scorer-320.png),
[Fantasy partner warning](fantasy-warning-390.png),
[guarded mobile lock](guarded-lock-320.png),
[guarded desktop lock](guarded-lock-1280.png), and
[resolved locked round](resolved-locked-390.png).

The full ladders intentionally leave the existing manual projection benchmark
ignored (requires explicit release-mode timing). Three match-list measurements and one team-loading measurement
remain ignored because they require an exclusive `pg_stat_statements` database.
These performance measurements are outside this correctness step; no performance
claim is made.

## Review and limitations

Read-only specialist review identified and closed the hidden-final scorer warning
gap and avoidable per-team name queries. It also strengthened the regression to
match the precise locked-round error with a valid expected score revision. Final
review has no remaining implementation findings. Evidence/document review is
recorded with the final checks.

Initial attempts exposed local test setup issues: blocked sandbox sockets,
production-image credential requirements, an API still waiting for the build,
missing interactive stdin for test SQL, and a logout locator expecting a link
instead of the actual heading. These attempts are not passing evidence. The
corrected acceptance asserts real SQL demotion and exact server history counts.
The first full database run stopped at the schema-28 Stableford upgrade fixture:
current confirmation-response construction now reads Fantasy tables that do not
exist in that historical schema. The fixture now inserts its historical
confirmation through the original bound round/session/actor transaction context
and existing triggers. Snapshot preservation and exact receipt replay assertions
remain. Both legacy-upgrade tests pass; the full ladder is rerun after the repair.
No production missing-table fallback or schema check was weakened. The stricter
browser response observer also records exact post-lock `round_not_editable`
scoring GET409 as expected, rather than broadly exempting all conflicts.

One combined browser invocation initially skipped four-ball because only the
award opt-in was enabled; all four four-ball cases were run separately with their
own opt-in before completion.

The final full database command exits **101**: **934 passed, 1 failed, 5 ignored**
across 87 harness results. It stops in unchanged
`tournament_read_sessions::coherence::response_order_roles_player_links_empty_and_error_contracts_are_preserved`
at `backend/tests/tournament_read_sessions/coherence.rs:225`. The assertion expects
`B-trip` before `b-trip`; Debian PostgreSQL17.11 `en_US.utf8` returns `b-trip` first.
A read-only `VALUES ... ORDER BY name` probe confirms that order. Both the test
and `tournaments/reads.rs` native ordering query have empty diffs. This case does
not call changed award/Fantasy readiness paths. Its later role/player-link/empty/
error assertions were not reached. See [exact failure](database-collation.txt).
All five subsequent test targets plus the remaining example were run separately:
**19 passed, 0 failed**. Total coverage across those commands is **953 passed,
1 failed, 5 ignored**; this is not a passing full ladder. The analogous older
[pairing collation report](../pairing-collation-2026-10-15/README.md) is context,
not evidence that this exact test passes on Alpine. Native database collation,
production sorting and the unrelated test remain unchanged and unselected.

No physical phone, WebKit, public host, native Docker deployment, image rebuild,
production-role runtime rehearsal or replacement frozen bundle was exercised.
Local Chrome uses the dev frontend and current Rust API with a disposable database
owner role. Existing authorization/role/concurrency database tests supplement
that acceptance. Already locked rounds are not reopened or repaired by this
preventive guard. The earlier DEPLOY-6 bundle predates these fixes and requires
matching replacement API/web image acceptance and refreezing before launch.
