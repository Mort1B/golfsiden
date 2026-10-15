# DEPLOY-4 — critical organizer/player acceptance

Iteration 4 of six for Sunday 18 October 2026 passes on the unchanged DEPLOY-3
production candidate. One fresh, complete five-phase sequence passes **32 named
checks**, including **27 browser checks**, then removes its owned resources.
No new P0–P2 application defect was confirmed and no production source, schema,
dependency or configuration changed. Current-schema recovery and final release
handoff remain open.

## Findings rated by severity

| Finding | Severity | Disposition |
| --- | --- | --- |
| Joining Fantasy can acknowledge the entry but leave the editor disabled after read reconciliation fails | P3 low, usability limitation | The existing acknowledged-action message and **Oppdater Fantasy** read-only retry restore editing. A fresh claimed player submits exactly one entry write, recovers through this state, then saves revision 1. No uncertain second entry write or lost selection was observed. Retained limitation; no speculative repair selected. |
| Local Chrome reports `ERR_ABORTED` for acknowledged no-content POST responses | Observation; no application severity established | Actual responses are `204`. Passive native entry-fetch controls without a tournament stream reproduce three of three events both with the original Caddy flush setting and with the default setting. Root cause remains unproven. The monitor accepts only explicitly declared POST paths with actual response `204`, records separate entry/logout counters and fails missing/non-204 responses. |
| Acceptance harness assumptions and an omitted organizer-entry monitor declaration | P3 low, validation only; fixed | Corrected actual selectors, async save/visibility waits, wire-field expectations and expected denied/cancelled requests. The final sequence starts from empty state and passes every phase and monitor. Earlier partial attempts are not counted as passing acceptance. |

The first finding is observable recovery behavior. The no-content observation
does not establish its cause: passive controls also reproduce without the
Fantasy action or live stream. No Caddy or application runtime change follows
from that correlation. The exploratory Caddy setting was restored exactly before
cleanup; the final fresh sequence uses the original candidate.

## Environment and artifacts

Docker Compose CLI 5.6.0 uses a private temporary rootless Podman 5.8.7 API on
amd64. Native local Docker socket access is denied and noninteractive sudo is
unavailable. No host socket, group, daemon or deployed service changed.
The real production Compose file runs PostgreSQL, API and Caddy/web; only web
ports are published, on loopback at random high ports. PostgreSQL remains on its
internal network. Fresh owner/runtime secrets, accounts and localhost CA files
are private and disposable; no production seed is used. Real Chrome
155.0.8059.79 uses certificate exceptions in disposable local contexts only.

The exact images are the [DEPLOY-3 candidate](../production-rehearsal-2026-10-15/README.md#environment-and-immutable-artifacts):

| Service | Image ID |
| --- | --- |
| PostgreSQL | `10318b2908d877b5d80b7460f7fc984ce11d08e31dfc8a7cc391562631cf76c7` |
| API | `1a6524c2727580acae040f98e978216de8a0ef9a86a7190bd463b18533a23bda` |
| Web with private proxy logging | `badbe24d6c7b796cb174a733e9124a792fbb666319fcbc9ee046637a2d6741e6` |

API/PostgreSQL retain source `b73d13e7f3aa8ee3508e306fdbb0b20584595937`
and context `a32af6b17ec12f63abd5570d582c1ca49711a14a601de640aff0e1a8363ecea3`.
The web retains source `22e90af40a9ba2e04d1a0ff2c02fbe26aaf634d1` plus
only the reviewed Caddy overlay, context
`0e116c0531485bf26b3f1e4a3b8a20da0863fdca401746e26b7c4d87cf245634`.
All runtime inputs remain unchanged from DEPLOY-3, including its logging repair
and 65 byte-identical web assets. Actual IDs and current input equality are
checked again by [postconditions](postconditions.json).

## Executed flows

[sequence.json](sequence.json) records setup → Fantasy → fresh entry → numeric
scores → final round, all exit 0, followed by successful final audit and cleanup.
[integration.json](integration.json) has 32 checks: four production-fixture
checks, 27 cumulative browser checks and one final runtime/log audit. Phase JSON
files contain cumulative checks; their counts must not be added together.

- [Setup](setup.json): organizer creates a two-round individual tournament in
  the UI. A distinct player registers through the actual invitation link; a
  prepared golfer later registers through a separate personal claim link.
  Four long-name golfers, two manual 18-hole par-4 courses and four-player
  flights are saved through organizer controls. Teams remain unnecessary for
  individual cards. Anonymous/outsider reads and player management/lifecycle/
  Fantasy configuration writes are denied; the player sees **Ingen tilgang**
  without organizer controls.
- [Fantasy](fantasy.json): rules 2 activates, empty standings render, and the
  organizer prepares both rounds with four picks and a captain. Round 2 commits
  but loses its response; the batch stops after one confirmed save. Exact replay
  uses the same path/body and returns the same revision-1 receipt. The other
  manager's draft remains absent even from organizer reads; results conceal
  other selections and contributions. Private reads require `no-store`.
  Loading, controlled read `503`, explicit read retry and populated boards pass.
- [Fresh entry](entry.json): a third manager claims the prepared golfer and
  joins through the UI. The test counts exactly one entry POST, requires the
  acknowledged/read-failure message before using its read retry, then saves
  four picks and a captain with an authoritative revision-1 receipt.
- [Numeric scores](numeric.json): organizer starts the tournament and opens the
  prepared round. A player score commits, loses its response and retries the
  identical request/acknowledgement. An offline local edit 5 conflicts with
  server 7; the browser retains the choice until **Behold serverscoren** removes
  the local queue and shows 7. Durable rows contain neither password nor CSRF.
  Authorized API writes prepare the remaining deterministic cards; organizer
  UI confirms/completes the round. Player gross/net views both show 72 with
  preserved handicap 0. An organizer UI correction while Completed changes to 73,
  clears confirmation and prevents locking until reconfirmed. After locking,
  ordinary writes to both organizer and member cards reject `409`; player lock
  rejects403. Gross/net 73 remains authoritative.
- [Final round](final.json): opening materializes the member's unsaved round-2
  selection from the previous locked round; further selection writes reject `409`.
  The member's view of another golfer has only nine visible numeric holes and
  no hidden completion/confirmation facts. Fantasy withholds final points,
  ranks and all nine back-nine hole facts. A hidden score change does not reveal
  its value. Organizer release and re-hide update the actual browser through
  native live refresh, including a page-return control. Player visibility
  administration and outsider Fantasy reads reject `403`; logout removes private
  boards and subsequent reads reject `401`.

The four handicap 0 golfers score par 72 for settled golfer 10 / manager 50. Correcting
the organizer to 73 produces golfer 4 and manager38 for captain-organizer, versus
44 for captain-member. Final-round par cards settle 50 each, yielding exact
two-round manager totals **88 and 94**. The member's board shows eight contribution
rows and the carried-forward receipt. Fantasy uses score/confirmation facts;
there is no separate settle endpoint or effect on sporting completion.

Correction acceptance is the existing authorized Open/Completed score path
followed by reconfirmation. Locked ordinary score writes are denied. This report
does not invent or claim a numeric correction operation while already locked.
Historical nonzero handicaps and other sporting formats retain existing focused
coverage; this is a bounded integrated journey, not another format matrix.

## Browser evidence and monitoring

Real Chrome runs at 320/390/1280 CSS-pixel widths with horizontal-overflow
assertions and synthetic long names. Relevant error, loading, empty, populated,
uncertain-save, conflict, denied-management and hidden/released states are
retained. Fresh mobile/desktop screenshots were inspected.

Screenshots: [invitation](invitation-joined-320.png),
[roster desktop](roster-1280.png), [round setup](round-setup-390.png),
[denied management](denied-management-320.png),
[Fantasy empty](fantasy-empty-320.png),
[uncertain batch](fantasy-uncertain-390.png),
[private draft](fantasy-private-draft-320.png),
[loading](fantasy-loading-390.png), [read failure](fantasy-read-error-390.png),
[recovered desktop](fantasy-recovered-1280.png),
[claimed player](claimed-player-fantasy-390.png),
[score conflict](score-conflict-320.png), [score recovered](score-recovered-390.png),
[net results](net-results-1280.png), [locked round](locked-round-390.png),
[hidden final](final-hidden-320.png), [released final](final-released-390.png),
[settled managers](fantasy-settled-managers-1280.png),
[settled golfers](fantasy-settled-golfers-390.png).

Every phase checks console/page errors, request failure classes and HTTP error
bodies. Expected error responses require the exact method/path/status/code;
cancelled Fantasy `409` projections are explicitly bounded. GET cancellation during
navigation/refetch is accepted. Lost-response and offline exceptions end after
their recovery window. Entry/logout `ERR_ABORTED` requires an actual `204` response
and an explicitly declared POST path; three entry events and one logout event
are recorded separately. These are acknowledged responses, not unacknowledged
write failures silently ignored by the monitor.

## Reproduction, review and cleanup

With the retained images, installed frontend Playwright/Chrome, Podman and Docker
Compose available, from the repository root run:

```bash
python docs/validation/critical-flows-2026-10-15/run.py
python docs/validation/critical-flows-2026-10-15/postconditions.py
```

`fixture.py` reuses the DEPLOY-3 fixture's private generated configuration,
destination validation, image/source guards, real owner migrations/grants and
owned cleanup. Its sole shared-helper change allows a separate receipt path;
historical DEPLOY-3 evidence is preserved. The runner refuses to replace an
existing private pointer. On failure it leaves only its private fixture for
diagnosis; `fixture.py finish <private-control-path>` audits and cleans that
project. Full acceptance requires all phases and successful cleanup before
`complete:true`. `postconditions.py` audits this recorded run, including its
exact observed counts, rather than requiring identical cancellation timing in
every future browser version.

Early exploratory/resumed attempts exposed selector, hidden-tab, stale saved-
label, server-controlled switch, wire-field and monitoring assumptions. They
also supplied the separate [passive no-content diagnostic](no-content-diagnostic.json).
The monitor correctly rejected a later otherwise-positive run because the
organizer entry declaration was omitted; adding that exact declaration fixed
the harness. [Exploratory cleanup](exploratory-cleanup.json) and
[monitor-triage cleanup](monitor-triage-cleanup.json) record partial checks and
cleanup only. Neither is a passing complete sequence. All final phase JSON and
screenshots were replaced by the one complete fresh run.

Read-only specialist review checked scoring arithmetic, correction/lock
semantics, privacy, exact request replay, fault-window boundaries, assertions,
fixture ownership and monitor exceptions. Findings in the harness were resolved;
the primary agent owns changes, evidence, integration and publication.

For each of the three owned projects, cleanup removed exactly three containers,
three volumes and two networks, verified no owned resources remained, stopped
the exact private API process and removed credentials, logs, diagnostics, CA and
socket directories. Browsers/contexts close. Four prior image IDs remain exact.
Final actual readiness/schema39/runtime-role reads, no service restarts and
absence of generated database passwords/proxy secret in complete container logs
pass. Runtime-role reads use the restricted connection; the full negative grant
and startup guards retain unchanged DEPLOY-3 evidence.

All new harness syntax and unchanged runtime inputs are audited. No Rust,
frontend application, schema or dependency changed, so unchanged full unit/
type/lint/Clippy/database-domain ladders were not repeated. The affected ladder
is fresh production Compose/schema/grants, actual browser/API failure/recovery/
privacy/scoring checks, final log/runtime audit and owned cleanup.

## Remaining release gates

DEPLOY-5 must prove backup/checksum/restore of this current application into a
fresh isolated project, state parity, real grants, login and restart/reconnect.
DEPLOY-6 freezes the tested candidate and triages current dependency/image
advisories before an explicit release verdict and operator handoff. Neither step
starts here. Native Docker Engine, actual host architecture, public DNS/ACME/
certificate trust, standard-port redirect following, physical playing phones,
production capacity and off-host recovery remain unverified. The prior
full-default lifecycle-test failure retains its specific recorded limitation.
No new feature, broad polish or speculative performance work is selected.
