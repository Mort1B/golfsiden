# Production images recover the preserved tournament state

The current production images were built and exercised in two isolated Compose
projects with synthetic data. Owner migrations ran twice, the restricted runtime
role started successfully, and Caddy served the app over local HTTPS. The normal
backup and restore scripts recovered the source into a fresh target.

Before any restored login or write, all 277 rows across 47 public tables matched
exactly, including 32 migration records, scores, handicap snapshots and audit
history. Administrator and member logins then returned the same private results;
anonymous and nonmember access stayed denied. The public result capability kept
the final back nine hidden, including when requested with administrator cookies.
For example, the administrator's preserved scorecard was 74 gross / 62 net over
18 holes, while the public projection exposed only its front-nine total of 36.

Chrome passed secure-session, score-write, live-event and reload checks on both
the source and restored stacks. Restored administrator/member results were also
checked at phone and desktop widths. Runtime schema/migration-history writes were
rejected. Confirmation, checksum and nonempty-target guards worked; a late restore
failure rolled back table creation and data loading completely.

One P2 documentation defect remains: the guide's standalone checksum example runs
from the wrong directory for a basename-relative sidecar. Verification from the
dump directory passed, and the restore script already uses that directory.
Correcting that example is the next bounded candidate. This validation step did
not change application code, migrations, deployment configuration or scripts.

The [recovery report](validation/recovery-2026-09-23/README.md) contains commands,
image IDs, diagnostic artifacts, exact results and inspected screenshots.
Recovery is **READY WITH KNOWN LIMITATIONS**; overall friends deployment remains
**NOT READY**. This was Compose on rootless Podman with local TLS, not a public
Docker Engine deployment. The separate security review, public-host acceptance,
native 200% browser zoom and physical Android Chrome checks remain outstanding.
Full unaffected implementation test ladders were not repeated. No production
deployment or real-data recovery occurred.
