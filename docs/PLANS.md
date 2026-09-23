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

### Next candidate: disposable deployment and recovery rehearsal

**Proposed validation-only step; awaiting approval.** Continue the outstanding
recovery gate from the [friends deployment assessment](validation/friends-2026-09-23/README.md).

Goal: establish that the current production images, migrations, restricted runtime
role, proxy and backup/restore procedures work together on disposable data.

Scope: an isolated production Compose project with unused ports/volumes and
synthetic accounts. Build the current images, initialize with owner migrations,
refresh runtime grants, start the restricted API and proxy, write representative
scores and permissions, back up and restore into a second fresh isolated target.
Verify authenticated reads, score preservation, migration state and private/public
visibility after restore. Keep operational findings separate from repairs.

Invariants: do not deploy to production, use real credentials/data, overwrite
existing volumes, or alter server scoring/authorization behavior. Browser drafts
are outside server backups and must not be presented as restored server data.
Preserve the separate security assessment and its findings/plan state.

Validation: exercise the documented operator commands, health/readiness, exact
restored values and API authorization. Record image/source revisions, commands,
failures and untested external DNS/TLS/browser/device prerequisites. Produce a
reviewed readiness report and one bounded next candidate if a defect is found.

Stop: publish the validation report; do not silently implement operational fixes
or start a production deployment. Native 200% browser zoom and physical Android
remain separate outstanding acceptance checks when suitable tools/devices exist.

No automatic opponents/byes/brackets, team match play, extra holes, new scoring
rules, public match sharing, cold offline launch or background sync is included.
