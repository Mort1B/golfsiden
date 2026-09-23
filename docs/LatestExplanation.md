# Backup verification works outside the checkout

The deployment guide now verifies a backup checksum from the directory holding
the dump and its sidecar. The backup script records only the dump basename in the
sidecar, so the previous command failed when run from the repository checkout.
A subshell changes directory for verification and preserves the caller's location.

A temporary non-secret fixture outside the checkout reproduced the old command's
failure. The corrected guide command passed, then correctly failed after the
fixture contents changed. Documentation links and the scoped diff were reviewed;
whitespace checks passed. OPS-1 is resolved in the
[recovery report](validation/recovery-2026-09-23/README.md#confirmed-finding-ops-1-p2-resolved).

This documentation repair is **READY**. Application code, backup/restore scripts
and deployment configuration are unchanged, so their unit, database and browser
ladders were not rerun. The earlier recovery evidence remains in the report.
Overall deployment remains **NOT READY** pending the separate security assessment,
public-host acceptance, native 200% browser zoom and physical Android Chrome checks.
