# Latest iteration — required restore checksum binding

A5 closes the P2 restore-integrity gap. The original script reached simulated
restore without a sidecar and when the sidecar validated another file. Three
additional reproductions used a trailing-newline alias, exported `CDPATH` or a
symlink/`..` directory decoy. All five now fail before any Docker command.

The script requires readable, regular, nonempty archive/sidecar files. It hashes
the selected archive, reconstructs its exact canonical GNU SHA-256 basename
record and compares the complete sidecar bytes. The sidecar never chooses a
filename for verification. Extra records, changed data and malformed/noncanonical
formats are rejected. Exact shell path splitting, cleared `CDPATH` and physical
directory traversal ensure the hash names the selected file, including GNU-
escaped filenames. Literal basename `-` is rejected as the hash tool's stdin
sentinel. The host requires `sha256sum` and `cmp`.

The backup script and its normal relocatable one-record format remain unchanged.
The restore's subsequent pristine-target query, existing stop policy,
transactional restore flags and runtime-permissions recovery retain their original
behavior. There is no application, schema, dependency or frontend change.

All 41 focused script cases passed with a nondelegating Docker shim. They cover
the five bypasses, unusual quoted paths, canonical-format failures, tool failures,
confirmation, input/target guards and failure/action ordering. Host sh/Bash POSIX
and independent Alpine ash syntax checks passed. Read-only review found no
source/harness issue.

A disposable PostgreSQL 17.10 fixture also passed the unchanged backup script,
moved-pair restore with spaces, 3,001-row/digest parity and populated-target
rejection. A late-truncated archive with valid checksum and readable TOC failed
during the actual restore wrapper and rolled back to zero public objects. No
permissions action followed failure; a separate verbose control confirmed table
and data work before rollback. Stop and permission actions are shimmed and
checked for ordering, not actual application grant recovery. Owned containers,
temporary archives and comparison copies were removed.

A checksum is transfer-integrity evidence rather than authenticity. The staging
directory must remain operator-controlled and unchanged during verification and
restore; concurrent file replacement is outside this repair. Full application,
frontend/browser and hosted recovery acceptance were not rerun or claimed.

Exact baseline, commands, results, format compatibility and limits are in the
[restore report](validation/restore-checksum-2026-10-15/README.md). The next
candidate is bounded password-work admission in [the plan](PLANS.md).
