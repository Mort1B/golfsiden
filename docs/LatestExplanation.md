# Playable four-ball stroke play

Four-ball is now available from tournament setup through scoring, confirmation,
round completion, overall standings and private history. It uses administrator-
assigned two-player teams, a shared 18-hole tee and a configurable allowance that
defaults to 85%. Each player keeps an individual handicap snapshot and input card;
the server independently selects the team's best gross and net score on each hole.
The team's round result contributes once to each frozen partner's overall total.

For example, on a par-four hole, A scores 4 with no received stroke and B scores 5
with two received strokes. The team counts gross 4 from A and net 3 from B. Equal
winning inputs retain both player identities. A pickup supplies no counting score;
one numeric partner result is enough for the hole. There is no scalar team handicap.

## Persistence, authority and results

Migration 0028 introduces dedicated player inputs, audits and immutable delivery
receipts. Numeric, pickup and restored numeric values retain the same input UUID
and advance a positive revision. Legacy numeric scores and immutable request bodies
remain unchanged. Ordinary member scorecards are actor-free; scoring reads and
team confirmation authorize both partners. Membership precedes private-read format
errors. Hidden-final filtering applies before totals, progress and winner details.
Public links retain their existing overall-summary scope.

Confirmation requires one numeric team result on all 18 holes and explicit
acknowledgement of remaining blank partner fields. Every actual partner input
change clears team confirmation, even when neither team total changes. No-ops and
receipt replays preserve it. A completed-round pickup can leave a side incomplete:
the card stays readable, its overall contribution is withheld until complete again,
and fresh confirmation is required before locking. Locked rounds reject ordinary
input changes and replay.

## Scoring and offline behavior

The mobile scorecard has two named partner controls, explicit pickup confirmation,
per-player stroke badges, and independent gross/net winners. Hole and summary
controls precede the card. Read-only history supports the same hole navigation.

Four-ball adds a tagged offline protocol while retaining existing untagged numeric
requests. Follow-up edits expect the acknowledged predecessor's revision; conflicts
show the local and current server states, including pickup and unentered values.
Pending edits or delivery verification on either partner block confirmation. Both
player-card leases are acquired atomically across tabs. Storage failures retain
usable retry/discard controls and guard navigation until the unsaved edit is handled.

Browser validation reproduced a live-refetch cancellation being treated as a
network error, leaving an already-updated card disabled for the retry delay. A
regression failed before the repair and passes afterward: cancellation retries on
the next queue wake while keeping verification pending. Only a successful fresh
read clears verification; actual network failures retain their existing backoff.

## Validation

- Backend: formatting, all-target tests (**192 passed**) and all-feature Clippy
  with warnings denied passed.
- PostgreSQL: the full database-feature ladder (**505 passed**) and **17** focused
  four-ball cases passed on disposable PostgreSQL 17.11. Fresh migration, a populated
  schema-27 upgrade and seeding twice passed. Coverage includes retained identity,
  audits, no-op/replay, direct SQL guards, current authority, deterministic receipt
  contention and late session expiry, rollback of confirmation changes, completed
  pickups, once-per-partner attribution and hidden private/public projections.
- Frontend: **517 tests across 89 files**, typecheck, lint and production build
  passed on the final source. Queue regressions preserve the actual legacy serialized
  head and verify atomic leases, acknowledged successors and cancellation recovery.
- Real Chrome: **13 cases passed**, comprising six four-ball cases and seven existing
  offline-scoring regressions. Layout checks used 320x600, 390x844 and 1280x900 with
  screenshots, overflow checks and reachable 44-pixel controls. Cases cover long
  names, empty/populated/loading/error states, manual setup, both partners, pickup
  identity, both conflict choices, lost acknowledgements across reload/two tabs,
  storage failure, confirmation, completed corrections, locked history, hidden
  results and tab return. The legacy cases retain account isolation and blocked
  corrections. Console and network assertions passed with deliberate injected
  failures accounted for.
- Read-only backend, frontend, concurrency and documentation reviews have no open
  material findings. Production source files remain below 400 substantive lines;
  diff and documentation-link checks passed.

Execution logs are under `/tmp/four-ball-*.log`; browser screenshots use
`/tmp/golf-offline-four-ball-*.png`. These temporary local artifacts are not published.

## Scope and readiness

**READY:** playable four-ball is implemented, reviewed and validated. The build
retains its existing chunk-size warning (main JavaScript 685.92 kB, 197.98 kB gzip);
performance work remains queued. Four-ball is 18-hole only. Cold offline launch,
background sync and queued confirmation are not provided. Stableford and match
play retain their tested calculation foundations but remain unavailable until
separate playable-integration steps are approved and completed.
