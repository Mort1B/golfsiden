# Latest iteration — scoped Fantasy sporting fact loading

A1u narrows canonical sporting-fact loading to the requested round for a Fantasy
round result. It filters the existing ordered tournament owner identifiers before
the unchanged canonical bulk query. Overall and golfer/manager breakdowns still
load every round. Full inventories, preauthorization, repeatable read, all-round
locks, game locking, every due-window finalization/carry-forward, target validation,
visibility, final session recheck, commit and notification behavior are preserved.
Only two production files change; API/JSON, schema, dependencies and frontend
refresh/write behavior remain unchanged.

Pinned baseline and candidate debug builds run on identical restored one/five-round
fixtures: eight golfers, four scramble teams per round, 18 holes and two managers.
The five-round scoped HTTP median with SQL tracking off falls from 45.03 to 22.45ms;
bulk SQL averages 10.59 to 2.72ms and canonical owner rows fall 20 to 4. Both builds
retain 25 statements and five round locks per read; full result/breakdown controls
retain 20 owners. The one-round control has no consistent timing benefit. These
are local debug/container measurements, not a production speedup/capacity claim.

All scoped, overall and golfer/manager breakdown JSON remains equal across builds.
First reads still materialize all one/five windows, two/ten selections and zero/eight
carries from the same restored initial state, independently of steady timing.
Existing hidden-result, canonical-fingerprint, invalid-target, sibling-lock and
carry-forward cases remain. New assertions compare selected rows against full-load
paths in all six formats, and a new API expiry case waits on canonical facts before
requiring 401 and complete lazy-finalization rollback. Foreign-round 404s also
preserve rollback/no-notification behavior.

Validation passes: 22 focused PostgreSQL tests, 264 Rust tests, 953 PostgreSQL-feature
workspace/all-target tests, formatting, strict Clippy, 39 migrations, seed and builds.
Five PostgreSQL-feature manual probes remain explicitly ignored; the standalone
before/after probe runs separately. Real Chrome at 320/1280 shows two manager/eight
golfer rows, no overflow or unexpected errors, equal results and anonymous denial.
Independent review and exact evidence/limits are in the
[comparison report](validation/fantasy-fact-scope-2026-10-15/README.md). Source
regression checks precede measurement; frontend unit/lint checks are not repeated
because frontend source/contracts are unchanged. Owned fixtures and credentials
are removed.

The next candidate is a separate test-only repair for the retained native-collation
ordering assumption. No adjacent runtime optimization or hosting work begins here.
