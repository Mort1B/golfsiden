# DEPLOY-5 — current-application recovery rehearsal

Iteration 5 of six for Sunday 18 October 2026 exercises the actual production
backup/restore scripts with schema 39 and acknowledged sporting/Fantasy state.
The complete corrected sequence restores into an independent empty project,
compares every public table and sequence before login, then validates restored
accounts, results, exact replay and restart/reconnect. Final release/advisory
acceptance and operator-host acceptance remain open.

## Findings rated by severity

| Finding | Severity | Disposition |
| --- | --- | --- |
| No new application or production-script defect confirmed | No new P0–P2 | Actual current restore, parity, authority, login and reconnect checks pass. No production source/configuration, schema or dependency changed. |
| First browser control allowed only five seconds for a quiet native stream to open | P3 low, validation only | Login, restored facts and layouts passed; stream-open count was still zero, with no recorded network/console error. The corrected control allows 30 seconds initially and requires a later actual reopen plus fresh reads. Complete fresh sequence repeated. No application root cause or repair is claimed. |
| Initial recovery controls did not isolate byte corruption or witness late SQL work, and receipt labels could overstate mode/current-run coverage | P3 low, validation only; fixed | Review required the tampered file's own basename with the original digest, a readable truncated-archive TOC plus verbose late-work/rollback witness, mode-specific stream descriptions and current-run `complete:false` initialization. Corrected controls pass. |

The existing P3 Fantasy acknowledged-entry/read-retry limitation remains in
[DEPLOY-4](../critical-flows-2026-10-15/README.md). This step does not reopen its
unproven cause or add feature/performance work.

## Isolation and unchanged candidate

Docker Compose CLI 5.6.0 uses two private rootless Podman 5.8.7 APIs on amd64,
with independent owner/runtime/proxy secrets, project names, named volumes,
networks and loopback high ports. Only web ports are published. Native Docker
socket access remains denied; no host socket/group/daemon or deployed service
changed. Chrome 155.0.8059.79 accepts the disposable localhost certificate only
inside these contexts; no public trust or host trust installation is implied.

The exact [DEPLOY-3 artifacts](../production-rehearsal-2026-10-15/README.md#environment-and-immutable-artifacts)
remain unchanged:

| Service | Image ID |
| --- | --- |
| PostgreSQL | `10318b2908d877b5d80b7460f7fc984ce11d08e31dfc8a7cc391562631cf76c7` |
| API | `1a6524c2727580acae040f98e978216de8a0ef9a86a7190bd463b18533a23bda` |
| Web with private proxy logging | `badbe24d6c7b796cb174a733e9124a792fbb666319fcbc9ee046637a2d6741e6` |

Production scripts deliberately use only `compose.production.yml`, without the
fixture override or explicit `-p`. The runner therefore binds their exact
private `DOCKER_HOST`, `COMPOSE_PROJECT_NAME` and generated interpolation values,
clears inherited Compose/context/TLS settings, validates script-only rendered
destinations and checks actual owned PostgreSQL labels. Unique temporary local
image aliases resolve the script's normal image names to these exact IDs.
Aliases must be absent before creation and are removed after both projects;
the four retained baseline/repaired image IDs remain exact.

The source runs real owner migrations, grants and production API/web. The target
starts PostgreSQL/roles only, with **no migrations before restore**. Its public
relations, routines and standalone types total zero. Its database volume and
credentials are independent from the source. No development seed is used.

## Actual recovery evidence

[sequence.json](sequence.json) requires all source phases and successful owned
cleanup before `complete:true`. [source-flows.json](source-flows.json) repeats
the five critical-flow phases with 27 cumulative browser checks. Output and
screenshots go to a private source directory, preserving all historical DEPLOY-4
receipts and images. Synthetic data includes four golfers, two locked rounds,
144 scores, eight confirmations, score audit/replay records, historical handicap
snapshots, invitations/claims, users/sessions/memberships and submitted/carried
Fantasy receipts. Other public tables are compared even when empty.

1. Close source browsers, stop source API/web and fingerprint quiesced data.
   Execute the unchanged `scripts/backup-production.sh`; require a nonempty
   custom archive and exact canonical basename SHA-256 sidecar.
2. Move the pair into a private directory containing spaces on the independent
   target. Recompute its digest. Actual restore rejects missing confirmation and
   changed archive bytes with a correctly named original-digest sidecar; public
   objects remain zero.
3. Intentionally truncate the trusted synthetic archive and create a diagnostic
   checksum for those known bytes. Its full TOC remains readable. The actual
   restore script fails with zero public objects and no success/permissions
   output. A separate verbose control with the same archive/transaction flags
   creates tables and processes table data before failure, then leaves zero
   public objects in its owned probe database. That database is dropped. This
   demonstrates late-work rollback rather than only early archive rejection.
4. Restore the intact pair using the actual script and real permissions service.
   Before API startup/login, all **61 public tables** have identical row counts
   and SHA-256 of sorted canonical row JSON. Sequence metadata plus actual
   `last_value` and `is_called` match. [parity.json](parity.json) retains counts
   and archive integrity metadata, without rows, credentials or session values.
5. Repeat the actual restore against the populated target. It rejects at the
   pristine-schema guard and leaves every table/sequence fingerprint unchanged.
   Actual TCP runtime login has the expected identity, no excess cluster/schema/
   owner authority and read-only migration history. `CREATE TABLE` and migration
   `UPDATE` fail with SQLSTATE `42501`; all 39 migrations remain successful.
6. Start restored API/web and prove distinct organizer/member UI logins. Fresh
   authoritative reads preserve gross/net 73, locked-write denial, full released
   final visibility, Fantasy totals 88/94 and the member's carried-forward
   receipt. Replay the exact privately retained pre-backup Fantasy submission;
   the same original receipt returns after restoration and closure.
7. Keep the member's authenticated browser context and explicit same-origin
   native EventSource open while the runner actually restarts API/web. Require
   a new stream `open` event and fresh authoritative result/readiness requests.
   This is a native transport reconnect on the profile page, not a claim about
   application query refresh from unchanged cached DOM.
8. Stop restored PostgreSQL: health stays 200 and readiness becomes 503. Restart
   PostgreSQL and require readiness 200, then fresh organizer/member login and
   the same authoritative score/Fantasy/replay checks. Recheck actual runtime
   authority. Complete service logs omit generated database passwords and the
   shared proxy secret.

[browser.json](browser.json) has four checks, including the actual API/web stream
reopen. [browser-post-database.json](browser-post-database.json) has three checks
for fresh logins/reads after database reconnect; it makes no stream-reopen claim.
Screenshots show exact restored results with long synthetic names at 390 and
1280 widths, with horizontal-overflow checks:
[organizer mobile](restored-organizer-390.png),
[organizer desktop](restored-organizer-1280.png),
[player mobile](restored-player-390.png),
[player desktop](restored-player-1280.png).

## Harness, validation, review and cleanup

From the repository root, with retained images, Podman, Docker Compose and the
installed frontend Playwright/Chrome available:

```bash
python docs/validation/recovery-2026-10-15/recovery.py
python docs/validation/recovery-2026-10-15/postconditions.py
```

`recovery.py` owns both projects and private control pointer. Failure retains
private diagnostics and sets no successful current-run verdict. Use its
`cleanup` argument to remove only its recorded owned resources. The shared
rehearsal helper now supports PostgreSQL-only setup; default migrated setup is
unchanged. Critical-flow helpers accept a separate evidence directory and an
explicit private exact-replay capture flag; normal default behavior is retained.
These are validation helpers, not an alternative operator deployment tool.

Read-only review resolved the validation gaps above and checked script targeting,
empty-target order, quiescing, parity before read materialization/login, actual
sequence state, replay, browser restart proof and cleanup ownership. The first
attempt's partial checks and owned cleanup remain in the `triage-*.json` files;
they are not a complete passing sequence. The corrected run uses fresh source
and target projects.

For each attempt, exact owned cleanup removes three containers, three volumes
and two networks per project, verifies no owned resources remain, closes browser
contexts, stops each private API process and removes both private trees including
archives, hashes of raw rows, credentials, logs, CA and socket files. Three owned
temporary image aliases per attempt are removed; original four image IDs remain.
[Source cleanup](source-cleanup.json), [target cleanup](target-cleanup.json) and
[ownership](ownership.json) retain sanitized evidence. No prune or unrelated
resource action is used. [Postconditions](postconditions.json) independently
checks receipts, 61-table parity, images, deleted roots/aliases, source scope,
historical evidence preservation and harness/shell syntax.

No production source, script, schema or dependency changed. Unchanged full
Rust/frontend unit/type/lint/Clippy/database-domain ladders and the 41-case
[checksum contract suite](../restore-checksum-2026-10-15/README.md) retain their
specific prior evidence. The affected current ladder runs real production
schema/grants/backup/restore, corruption/nonempty/late-work controls, complete
data/sequence parity, actual restored browser/API behavior, restarts/reconnect,
log audit and exact cleanup. Current native Docker Engine, public host,
certificate trust/renewal, standard-port redirect, physical playing phones,
capacity and off-host transport/restore remain unverified. This local moved-pair
exercise is not an off-host disaster-recovery test. Unsent browser edits are
outside a server dump.

## Short operating handoff

Use the [deployment guide](../../deployment_guide.md#backup) for the supported
commands. Record the matching release images/schema, stop application writes
for a consistent handoff point, create and verify the archive/sidecar, and copy
both into protected off-host storage. Restore with explicit confirmation into a
fresh project/volume with independent private credentials; do not migrate before
restore. Start API/web only after restore/grants succeed, then check readiness,
organizer/member login, representative score/result/privacy/Fantasy facts and
restart/reconnect. Keep the source and trusted backup until that target is
accepted. Preserve unsent device edits separately. DEPLOY-6 owns the remaining
freeze, current advisory triage, release verdict and launch/rollback checklist;
none of those actions starts in this iteration.
