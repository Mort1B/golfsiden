# A1v — native-collation test portability — 2026-10-15

The retained P3 test portability issue affects one legacy individual-group
contract case, not application ordering or security. The original case assumes
uppercase-first native ordering for both team names and member display names.
This step changes only that test and evidence/documentation. Production SQL,
collation, API, UI, schema and dependencies are unchanged.

## Reproduction

The unmodified test from baseline `c428bb395c646eeffc8d759c0c0d69e4e7976161`
passes on Alpine and fails on Debian at the predicted first team-ID assertion:
actual `…0001` (`b-group`), expected `…0002` (`B-group`). The runner accepts that
baseline failure only when the exact test, exit 101 and both synthetic assertion
IDs match. It does not treat an arbitrary failure as reproduction.

| Fixture | Native group order | Native member order | Original test |
| --- | --- | --- | --- |
| Alpine/musl PostgreSQL 17.10 | `B-group`, `b-group` | `B-player`, `b-player` | Pass |
| Debian/glibc PostgreSQL 17.11 | `b-group`, `B-group` | `b-player`, `B-player` | Predicted assertion fails |

Both fresh databases use `en_US.utf8` with libc provider `c`. Direct synthetic
`VALUES` probes confirm both native orders; explicit `COLLATE "C"` returns
uppercase-first on both. No production query is changed to force that collation.
Exact versions and cached image IDs are in [Alpine baseline](baseline-alpine.json)
and [Debian baseline](baseline-debian.json). Images also differ in PostgreSQL
patch version, so this establishes the two environments' observable difference,
not a controlled claim isolating libc as the only causal variable.

## Repair and retained assertions

The target case retains its two case-variant groups and two case-variant members.
Two parameter-bound `unnest` queries sort only those known fixture values by
native name and UUID. Expected order is independent of repository-returned rows,
persisted-team queries and application helper functions. Exact API ID sequences
are compared with those fixture expectations, including collection cardinality.

Pairing's separate `lower(name),id` and `lower(display_name),player_id` contracts
retain explicit whole-sequence expectations: group IDs `…0001, …0002`, player IDs
`…0011, …0012`. Their folded names tie, retaining UUID tie-break coverage. On
Alpine these differ from native ordering; on Debian they happen to coincide.
The same-named team constraint prevents adding identical group names merely to
manufacture a native tie. No new fixture or constraint behavior is introduced.

Schedule, timestamp and withdrawn-entrant assertions remain. The source team is
located by UUID before timestamp comparisons, so native position no longer
determines identity. HTTP success, both full member sequences, both full group
sequences and the empty group's membership are asserted.

## Validation

| Check | Result |
| --- | --- |
| Corrected exact case, Alpine / Debian | One passed on each |
| Complete pairing-session suite, Alpine / Debian | 14 passed on each |
| Formatting | Passed |
| Workspace/all-target Rust tests | 264 passed, one existing ignored projection benchmark |
| Strict all-target/all-feature Clippy | Passed |
| Workspace/all-target PostgreSQL-feature tests, Alpine | 953 passed, five existing manual probes ignored |
| Forward migrations / seed, Alpine | 39 successful migrations; seed passed |
| Owned container and temporary credential cleanup | Passed on both baseline and candidate fixtures |

The PostgreSQL total includes ordinary backend tests and is not additive with
264. The five ignored cases are the projection benchmark and four existing
PostgreSQL profiling probes; their exact names appear in the
[final postconditions](final-postconditions.json). They are unrelated to this
test-only repair and were not invoked. Exact exit codes, counts and durations
are retained in [Alpine acceptance](candidate-alpine.json) and
[Debian acceptance](candidate-debian.json). The full PostgreSQL gate took 868s;
that duration is validation bookkeeping, not an application benchmark.

Independent [read-only review](review.md) found no test/source defects. Its one
P3 evidence-runner gap (exit-zero accepting zero filtered tests) was repaired,
negative/positive controls passed, and follow-up review closed the finding.
Because the application ladder had already loaded the earlier runner, the
primary separately verified exact focused/suite counts, all candidate exits
and all fixture cleanup against every retained JSON artifact. The linked final
postconditions confirm those checks. No review finding remains unresolved.

## Reproduction procedure and limits

Run `python3 docs/validation/pairing-collation-2026-10-15/validate.py candidate`
from the repository. It uses cached PostgreSQL images, private random credentials,
loopback-only random ports, tmpfs data and exclusively owned disposable containers.
The runner removes its containers and temporary credentials. Raw local diagnostic
logs remain mode 0600 in `/tmp`; tracked JSON contains sanitized counts and
synthetic ordering metadata only. It never targets a retained or hosted database.
The `baseline` mode is intended for the original test at the pinned revision,
with this runner copied into that checkout before making the repair.

The complete PostgreSQL ladder runs on the repository's Alpine image; focused
and complete pairing-session suites run on both images. Other database locales,
ICU, custom column collations, non-ASCII names and hosted versions are not covered.
There is no runtime, frontend or user-facing change, so browser/frontend checks
are outside the affected ladder. This is test acceptance, not a performance or
deployment-readiness claim. Other queued findings remain separate.
