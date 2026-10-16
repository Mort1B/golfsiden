# RELEASE-REFREEZE — replacement frozen images

Verdict: **READY WITH KNOWN LIMITATIONS for this exact locally accepted bundle**.
This replaces the current handoff from DEPLOY-6. The historical bundle and all
of its identity/acceptance receipts remain intact. No hosted deployment or
registry publication occurred.

The new API/web contain committed RELEASE-RECOVERY source
`baeeda9d51fd730bffb32892bbcbc25a79a24079`: deliberate award-recovery abandonment
after denial and the canonical Fantasy four-ball lock guard. Schema remains39.
No application, Dockerfile, dependency, migration, production setting or backup
script changed in this step. The prior [source validation](../release-recovery-2026-10-16/README.md)
remains separate evidence, including its failed full PostgreSQL attempt.

## Frozen delivery

- Common tag: `release-20261016-35a7165802fc`, Linux/amd64.
- Tracked context SHA-256: `35a7165802fc87164b4653cecc7fdfee0cca254930a5171d72d446db6db0124c`.
- Bundle: `/tmp/golfsiden-release-20261016-35a7165802fc-juwwdsq1.tar.gz`.
- Bundle SHA-256: `ad571b549ee5f27d52a17adcdeb08cb12aa30f24652449dbca17a5593bcb4422`.
- Bundle bytes: 218921131. The protected extracted directory is recorded in
  [bundle.json](bundle.json). Copy the package to durable operator-controlled
  storage; `/tmp` is staging, not an off-host backup.
- [manifest.json](manifest.json) binds all three archive checksums/config IDs,
  runtime-file hashes, source/context identity, acceptance and remaining gates.
- Use the [operator handoff](OPERATOR.md) and unchanged protected
  [loader](verify-release.py). Load archives, keep the common tag, verify the
  private environment/project binding, and use both supplied Compose files with
  `--no-build --pull never`. All five service aliases are bound, including the
  migration and permissions tools used without the override by backup scripts.

| Image | Exact config ID |
| --- | --- |
| PostgreSQL | `ed0f7dce2da1088d611609dfc5686efc64d1a7b249a9f04466c95b215c0ee8bd` |
| API | `4049bdb0bb0d9435c764c5f4ae71c28a22f41a67ba3a21242f362e77459c7b40` |
| Web | `6a8fdffd694d4b927b15f8607e3abaa2dfba53aae2b108f4445fc8330e5cb20d` |

## Exact acceptance

[Build receipts](candidate-builds.json) come from `git archive` of the complete
source commit, with no worktree overlay. Dockerfile hashes equal that commit.
All three images have verified source/context labels. Builds use locally
available inputs with `--pull=never`; they are not a promise that a future build
will reproduce these bytes. [Content inspection](image-content.json) verifies
that the API executable and web assets changed, there are 66 built web files,
the migration executable is unchanged, and all three runtime package sets and
the Caddy binary are identical to DEPLOY-6. The rebuilt password-recovery
executable also differs; its existing execution/validation preflight passed.
The [actual module metadata](candidate-web-build-info.txt) still identifies
Caddy2.11.7, Go1.27.2, x/net0.60.0 and quic-go0.63.0 with standard modules.
The custom CLI prints `unknown`. PostgreSQL reports 17.11. No new advisory scan
or runtime security update was selected; the earlier conditional advisory
triage remains historical evidence, not a current scanner-clean claim.

The unchanged production/recovery harness ran through real HTTPS/Caddy and the
restricted runtime database role, using rootless Podman 5.8.7 and Compose 5.6.0:

| Evidence | Observed result |
| --- | --- |
| [Packaged preflights](preflight.json) |23/23; no database initialized or proxy listener started by probes |
| [Source acceptance/cleanup](source-cleanup.json) |56/56; exact candidate snapshot and restricted role/schema/startup guards |
| [Boundary browser](production-browser.json) |7/7; distinct accounts, cookies, CSRF, live updates, quota, security headers, long content |
| [Critical flows](source-flows.json), [sequence](sequence.json) |25/25, five ordered phases: setup, Fantasy, entry, numeric, final |
| [New release regressions](release-regressions.json) |3/3, Chrome 320/390/1280; no skipped cases |
| [Fresh restore acceptance](target-cleanup.json) |16/16; actual current backup/restore scripts, confirmation/corruption/nonempty controls and late transactional rollback |
| [Restored parity](parity.json) |All 61 public tables, 1122 synthetic rows and sequence metadata/state exact before target login |
| [Restored browser](browser.json), [after DB reconnect](browser-post-database.json) |4/4 and3/3; gross/net 73, Fantasy 88/94, exact receipt replay, API/web stream restart, outage/reconnect |
| [Protected transfer](transfer.json) |10/10; real archive load, exact aliases, collision/tamper/optimized-Python/config/password/project controls |
| [Release entry-point guards](guard-controls.json) |7/7 reject optimized Python before mutation |
| [Ownership](ownership.json) |Owned source/target containers, volumes, networks, temporary aliases, private env/CA/browser profiles removed |
| [Build cleanup](cleanup.json) |Owned build logs/staging removed; both frozen bundles retained |
| [Final postconditions](postconditions.json), [read-only review](review.json) |Identity, scope, retained bundle, packaging, syntax, links and cleanup checked |

The seven boundary checks are a separate receipt; they are not included in the
source receipt's56 checks. Critical and new regression checks overlap the source
receipt and must not be added together as unique tests. Runtime outage controls
preserve health200/readiness503 until reconnect restores readiness200. Actual
proxy502 logs and complete container logs omit secret/header canaries.

The [regression adapter](regressions.py) preserves every committed assertion in
`frontend/e2e/releaseRecovery.browser.ts`; only imports, both HTTPS browser
contexts, the owned SQL identity and private output paths are changed. The tests
retain the signed-in account, remove tournament-admin access in real SQL during
the award request, return403 and deliberately leave without a second write.
A separate case commits the original write, loses its response, denies the
exact replay and proves abandonment preserves award history. Four-ball accepts
sporting confirmation with one blank partner, shows authoritative missing holes
before confirmation and in lock review, returns409 on direct lock, then accepts
explicit per-hole pickups, renewed confirmation and locking. Locked writes still
deny. The ordinary final-round scorer sees the readiness warning while global
completion/results remain concealed; authorized own-side scoring data stays
available. Screenshots below were inspected for wrapping and usable controls:

- [Denied initial request,320](regression-denied-initial-320.png),
  [denied replay,1280](regression-denied-replay-1280.png).
- [Guarded locking,320](regression-guarded-lock-320.png),
  [1280](regression-guarded-lock-1280.png),
  [hidden final scorer,320](regression-hidden-final-scorer-320.png),
  [resolved and locked,390](regression-resolved-locked-390.png).
- [Restored organizer,390](restored-organizer-390.png),
  [1280](restored-organizer-1280.png), [restored player,390](restored-player-390.png),
  [1280](restored-player-1280.png).

## Limits retained

Native Docker Engine, chosen host architecture, public DNS/ACME/ports 80/443,
actual playing phones and a protected off-host dump/restore remain operator
acceptance gates. Local certificate exceptions do not establish public trust.
No physical device, hosted deployment, fresh vulnerability scan or production
capacity check ran here. The previous P3 Fantasy entry/read-retry limitation and
unclassified lifecycle-test failure remain in the dated reports.

The prior source full PostgreSQL attempt is **not green**: 934 passed, 1 failed,
5 ignored; separately completed remaining targets passed 19. The unchanged
`tournament_read_sessions` test assumes uppercase-first ordering at
`coherence.rs:225`; native Debian PostgreSQL17.11 `en_US.utf8` returned lowercase
first. Later assertions in that case were not reached. No collation, ordering,
test or application repair was selected here, and the source ladder was not
rerun for this delivery-only change. Runtime package equality and these focused
image regressions do not erase that limitation. Already locked unresolved rounds
are not reopened or repaired by the preventive lock guard.
