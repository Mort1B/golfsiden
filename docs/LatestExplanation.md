# Latest iteration — legacy write authority assessment

A1o inventories retained team/round write authority at source revision
`5b79760247b221232fc64653a061be363de17866`. This step changes documentation only.
The [report and caller manifest](validation/legacy-write-inventory-2026-10-15/README.md)
cover six reachable round mutations, three retired team paths, ten retained
non-HTTP helpers and three provisional preflights.

Three source-supported P2 findings remain: direct round creation, aggregate
pairing replacement and course configuration check the actual session and exact
admin membership at transaction entry, but lack a final wall-clock expiry check
after dependent work. For example, a session can be valid at its locked query,
expire while the membership query waits, then proceed to the INSERT without
another expiry check. No new runtime race was reproduced in this assessment.

Session/user/membership locks serialize later revocation, credential and role
changes; natural expiry needs no row update. Database guards continue to enforce
draft state, roster scope, parent configuration and immutable course revisions.
Open, complete and lock
already recheck the session after all inner work through their A2 wrappers. Even
those final checks do not freeze time during COMMIT or network delivery.

The granular team-write routes are retired; their six retained helpers have no
backend source or test callers. They receive no HTTP vulnerability rating.
Pairing/configuration preflights use only the extracted user ID, but their final
write transactions independently require the actual session. Conditional
provider work before that final check is recorded as a limitation, without
claiming a successful provider request from the currently unusable catalog.

Source/caller reconciliation, manifest counts, local links, source anchors and
diff checks are the applicable validation. Existing tests were inspected, not
rerun. No build, database, migration, provider, browser or hosted action was
needed; prior acceptance runs remain historical evidence. Independent read-only
review confirmed the classifications, severity and bounded next candidate with
no actionable findings. Static route/manifest, source-anchor, link, scope and
diff checks pass.

The [plan](PLANS.md) proposes W1 only as the next bounded repair: reproduce late
creation expiry, then add a final check with complete rollback and no event.
Pairing/course repairs, helper cleanup, provider changes and performance work
remain separate. No implementation began in this assessment.
