# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. See the [latest explanation](LatestExplanation.md) for the completed iteration.

## Next candidate

**Wider application and operational security review (awaiting approval).** Define
and review a bounded assessment of authorization, session handling, private-data
boundaries and production operations before implementation. Keep findings separate
from repairs; prioritize concrete reproducible risks and validation evidence.

## Later queue

No additional implementation step is currently approved.

### Next candidate: correct the backup verification example

**Proposed documentation-only repair; awaiting approval.** Resolve OPS-1 from the
[recovery rehearsal](validation/recovery-2026-09-23/README.md).

Goal: make the documented standalone checksum command work when the dump and its
basename-relative sidecar live outside the checkout.

Scope/behavior: update the Backup example in `deployment_guide.md` to verify from
the dump directory; update the current explanation and finding status. Preserve
the existing backup/restore scripts, image configuration and application behavior.

Validation: reproduce the old failure and corrected success using a temporary
non-secret file/sidecar outside the checkout, run documentation checks and review
the scoped diff. Preserve the separate security assessment and its plan state.

Stop: publish only the reviewed documentation repair. Do not begin security
remediation, production deployment or additional feature work. Public-host
acceptance, native 200% zoom and physical Android Chrome remain outstanding.

No automatic opponents/byes/brackets, team match play, extra holes, new scoring
rules, public match sharing, cold offline launch or background sync is included.
