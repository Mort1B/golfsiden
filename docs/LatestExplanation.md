# Persist Fantasy games and round selections

FANTASY-3 adds private selection APIs and PostgreSQL persistence at schema 37.
Members can enter an enabled game, save four golfers and a captain, recover an
accepted save by retrying its request ID, and automatically carry eligible picks
forward. Administrators configure games/deadlines and record audited non-finish
dispositions. Fantasy result APIs and screens are still planned; this is not yet
a playable game in the UI.

The database enforces complete lineups, captain membership, tenant identity and
immutable receipts. Deadline closure uses the earlier published UTC deadline or
actual golf-round opening. Temporal membership and golfer eligibility make lazy
closure reflect the deadline itself. For example, a golfer withdrawn after a
09:00 deadline still belongs to a valid lineup at that deadline even if the first
read occurs at 09:15. A later round can carry that lineup only if all four are
eligible at its own deadline. Captains and source-round provenance are preserved.

Pre-lock reads expose only the caller's lineup, including for administrators.
After lock, current members can read participating selections. No entry can
backfill a closed round. Accepted retries return their original receipt; changed
bodies or revisions conflict. All routes use the existing authentication/CSRF
boundaries, private no-store responses and post-commit invalidation.

Non-finish records bind to a canonical full-source token and retained mutation
generation. Score changes, either four-ball partner, Stableford input, confirmation
and accepted match changes invalidate earlier tokens. Empty cards and deleted
inputs retain protection against stale re-attestation. Only exact administrators
can read these tokens or write dispositions; locked changes, replacements and
reversals require explicit corrections with reasons. This does not complete the
golf round or override confirmed cards/accepted match results. Future projection
work will apply these records to the tested domain scoring rules.

Validation:

- Backend default ladder: 249 tests passed (247 library, two CLI).
- Final database ladder: 695 tests passed, including the 20 new Fantasy tests.
- Focused Fantasy PostgreSQL coverage: 20 tests, including controlled lock-wait
  deadline/revocation/activity races, real opening, pre-lock privacy, exact replay,
  carry-forward, membership moves, all format source generations and direct guards.
- A populated schema-34-to-37 upgrade preserved exact player/round/tournament
  contents; migration replay and repeated seed passed. Fresh final migrations and
  CLI seed twice passed on disposable PostgreSQL 17.
- Restricted non-owner role exercised configure/entry/save/lock/carry/open,
  non-finish recording and later score invalidation. Runtime role initialization
  and grant refresh passed, with only the disposable hostname substituted for
  Compose's `postgres`; actual runtime login cannot write migration history.
- Formatting and strict all-target/all-feature Clippy passed. Read-only schema,
  API and durable-documentation reviews have no outstanding findings.

The first full database run caught a new identity guard blocking an existing
password-recovery membership move. The fix records old/new membership history
without changing recovery authority serialization; all 20 recovery tests pass.
Review also closed direct timestamp fabrication and concurrent player-activity/
roster history races. Two Clippy findings and one new test fixture's missing
required display name were repaired. Existing vendored SQLx warnings remain.

No frontend or browser checks ran because this step adds no screens or frontend
contracts. No production deployment or new restore rehearsal was performed.
Large-field closure latency is unmeasured: first materialization resolves/writes
per entry under tournament round locks, while subsequent reads reuse selections
and bulk-load receipts. The [next candidate](PLANS.md#next-candidate) is FANTASY-4:
authoritative round/overall result projections, including the golfer points board.
