# Required restore checksum binding — 2026-10-15

A5 closes the P2 [restore-checksum finding](../application-review-2026-10-14/README.md#a5--p2-checksum-requirement-is-optional-in-the-restore-script).
Production baseline: `694f7360f51b41cbbeed3d07fd9ee841bdfa5cb7`.

## Failure and repair

The [original-source probe](baseline.json) applies five synthetic cases to an
isolated copy of the original script. Every case exits 0 and reaches simulated
`pg_restore`, despite failing the required selected-archive integrity contract:

- Missing sidecar.
- A valid digest naming a different intact file.
- A trailing-newline archive name whose stripped alias has a valid pair.
- Exported `CDPATH` redirects relative directory selection to a valid decoy.
- A symlink/`..` directory path selects a valid logical-directory decoy while the
  original archive path resolves through the physical parent.

The baseline is an expected test failure, not a real restore: the Docker shim
records arguments, ordering and input digest and never delegates to Docker or a
database. The baseline was rechecked after correcting the harness's selected-byte
assertion and adding an explicit script-under-test option; all five bypasses
still reproduced. No production source rollback was used.

The production repair changes only `scripts/restore-production.sh` before its
existing database guard. After explicit confirmation it requires both selected
archive and `.sha256` sidecar to be readable, regular, nonempty files. The host
must provide GNU `sha256sum` and `cmp`. It preserves path bytes with shell
parameter expansion, rejects the literal basename `-`, clears `CDPATH` and uses
physical directory traversal. A failed `cd` or hash command explicitly aborts.

The script computes one canonical record for the selected basename and compares
it byte for byte against the entire sidecar, restoring the one final newline
removed by command substitution. GNU escaping keeps embedded filename newlines
and backslashes within that record. The sidecar is never interpreted as a list
of files to read. Foreign-file digests, duplicate/appended records or bytes,
changed archive bytes and malformed formats all fail before any Docker command.
Only the one checksum record is held in memory; the archive is hashed as a
stream. No speedup or large-archive benchmark is claimed.

The supported manifest is exactly the default text-mode GNU output emitted by
the existing backup script: lowercase SHA-256, its basename and one final newline,
including GNU filename escaping when needed. Binary-marker, uppercase-digest,
missing-terminator and extra-line alternatives are deliberately rejected.
The backup script is unchanged. Restore's unusual-name tests use correctly
constructed GNU records; they do not claim the old backup wrapper supports
leading-dash/trailing-newline output basenames. Normal timestamped backup names
and moved dump/sidecar pairs remain compatible.

The existing sequence after checksum success is unchanged: pristine-public-schema
query, existing service-stop policy, `pg_restore --exit-on-error
--single-transaction --no-owner --no-privileges`, then runtime-permissions recovery.
No permissions command or success message follows a failed restore. No database
schema, application source, scoring rule, frontend, dependency, deployment setting
or hosted environment changed.

## Focused acceptance

The [retained probe](restore_probe.py) and [41 passing cases](focused-results.json)
exercise actual script subprocesses with a private fake Docker executable:

- All five baseline bypasses now fail with zero Docker calls.
- Valid canonical records, spaced directories/basenames, relative paths,
  leading-dash names, GNU-escaped backslashes and middle/trailing newlines,
  newline directories, Unicode and literal shell metacharacters are handled as
  data. Literal basename `-` fails before hashing standard input.
- Missing, empty, unreadable and nonregular inputs/sidecars; malformed/wrong,
  duplicate, blank-line, appended-byte/NUL, binary-marker, uppercase and
  unterminated manifests; failed hashing/comparison utilities.
- Confirmation, empty-input and nonempty-target guards, failed target query,
  existing nonfatal stop-failure policy, restore failure and permissions failure.
- Exact Compose/environment arguments, complete public relation/routine/type
  query, selected stream bytes, transaction/ownership flags and action order.

No real Docker/database is invoked by this probe. It records no environment
contents, credentials or archive payloads. Readability cases ran as UID 1000;
none was skipped.

## Actual disposable PostgreSQL acceptance

The separate [PostgreSQL probe](postgres_probe.py) and
[passing results](postgres-results.json) own one unique PostgreSQL 17.10 Alpine
container with no published host ports. Its narrow Docker shim delegates only
PostgreSQL commands to that recorded container ID:

- The unchanged production backup script creates a genuine custom-format archive
  and exact canonical sidecar for two synthetic tables and 3,001 rows.
- Moving the pair to a directory containing spaces still permits the repaired
  restore. Row counts and ordered-data digests match the source.
- Repeating against the populated target fails after the actual pristine-schema
  query, before the stop/restore/permissions actions.
- A late-truncated archive has a readable complete table of contents and a newly
  valid checksum. The actual wrapper reaches restore, fails late and rolls back
  to zero public relations/routines/types; no permissions action follows.
- A separate verbose control confirms table creation and table-data processing
  before the same transactional failure and zero-object rollback.

Service stop and the permissions tool are shimmed and checked only for invocation
and order. This small fixture does not prove application runtime grants,
schema-39 data recovery, current UI/logins or hosted disaster recovery. The
harness records ownership before startup and removes only its own container ID;
temporary archives and its isolated environment file are removed on completion.

## Validation

| Check | Result |
| --- | --- |
| Original script baseline | Five expected failures; all reached simulated restore |
| Focused script contract suite | 41 passed, no skipped cases |
| Actual PostgreSQL fixture | Backup, moved-pair restore, nonempty guard and late-failure rollback passed |
| Host `sh -n` / Bash POSIX syntax | Passed for restore and unchanged backup scripts |
| Independent Alpine BusyBox ash syntax | Passed for both scripts; no networking or database process |
| Independent read-only source/harness review | No actionable findings |

The [validation summary](validation-results.txt) records commands and limits.
ShellCheck and a standalone dash are not installed; no result is claimed for
them. Production shell syntax was additionally parsed by Alpine ash. Full
Rust/frontend/browser ladders were not rerun: those sources, contracts and
user-facing states are unchanged. The affected shell/recovery boundary received
focused functional checks and actual PostgreSQL transaction validation.

## Limits and remaining work

Checksum matching proves transfer integrity, not authenticity if both files can
be replaced. Staging must be operator-controlled and remain unchanged during
verification and restore. This step does not pin an open archive descriptor or
protect against concurrent file/symlink replacement between hashing and restore.
Recover the original trusted sidecar rather than generating one from unverified
bytes merely to satisfy the gate.

The existing script's stop-failure policy and runtime-permissions helper remain
unchanged. No hosted configuration, actual deployment or full application recovery
was exercised. Older [recovery evidence](../recovery-2026-09-23/README.md) retains
its own historical scope; it is not promoted to current-release acceptance.
The next candidate is bounded Argon2 admission in [the plan](../../PLANS.md).
