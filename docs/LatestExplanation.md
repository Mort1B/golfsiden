# Configurable overall tournament ties

Tournament admins can now choose how equal overall totals are ranked under
**Turneringsstyring → Innstillinger → Ved lik totalscore sammenlagt**. “Delt
plass” remains the default for existing and new tournaments. The optional
“Siste runde, deretter delt plass” compares the final scheduled round using the
selected gross/net view, including when that round is excluded from best-N.
The choice is permanently frozen by tournament start or prior round opening.

For example, two qualified players share a best-N total of +6. Their final-round
scores are +2 and +4: the player with +2 ranks first within that group. Equal
final scores retain shared competition places. Every member must have a complete,
visible, non-provisional final contribution and no selected provisional result;
otherwise the entire group stays tied. The final is the configured scheduled
round, never the latest completed round. Hidden final scores cannot influence
member-visible positions or explanations.

The standings show the chosen rule and a same-metric “Siste runde” explanation
only for groups actually compared, including residual shared places. Round
standings, completed-only qualification, mandatory-round selection, preserved
handicaps and administrator-managed team attribution keep their existing rules.

## Boundaries and compatibility

Migration 0025 adds a closed policy enum/default and extends the existing
pre-start database guard. The admin settings PATCH accepts an optional non-null
policy; omission preserves the existing setting. Tournament reads expose policy;
standings additionally expose the scheduled final number and nullable per-entry
comparison score. Frontend decoders validate these against visible contributions
before accepting server ranks into the private cache.

Creation/onboarding inputs and serialized retry fingerprints are unchanged.
Schema-upgrade tests preserve nonempty score, snapshot and receipt history.
Configuration/start race tests force both contention orders. Review also found
and resolved a missing creation response projection and a settings form that
could remain busy after same-user session replacement. A keyed editor and deferred
response regression now protect that replacement session. Settings reconcile
scoped authoritative queries after writes and expose retry after refresh failure.

Deploy migration 0025 with matching API/frontend builds and reload existing
clients before enabling the optional policy. Older clients can reject newly split
places. The deployment guide covers upgrade and backup-based rollback. No
production database or account was changed by this iteration.

## Validation

- `cargo test --workspace --all-targets`: **130 passed**.
- Full workspace/all-target PostgreSQL suite with `database-tests` and
  `--no-fail-fast`: **399 passed**, including those domain tests. Disposable
  PostgreSQL 17 migration and development seed commands also passed.
- `cargo fmt --all -- --check` and all-target/all-feature Clippy with
  `-D warnings`: passed.
- Frontend: **445 tests passed in 75 files**; typecheck (including browser
  TypeScript), lint and production build passed.
- Chrome: **2 new scenarios passed** at 320×600, 390×844 and 1280×900. Actual API
  settings exercised default, delayed save, 503/retry, successful persistence,
  concurrent stale update and permanent start freeze. Controlled standings
  exercised metric-specific winners, residual ties, excluded finals, a missing
  group member, hidden final, loading, error/retry, empty and long-name states.
  Touch targets, trial-click reachability and horizontal overflow were checked;
  mobile/desktop screenshots were inspected. No unexpected console/page errors
  or HTTP failures occurred; expected 409/503 and navigation aborts were asserted.
- Existing Chrome return-to-page suite: **8 passed**, including visibility/tab
  return, stream reconnection, session expiry, offline resume and locked-round
  refresh.
- Read-only backend/frontend and durable-document review has no open findings.
  Diff whitespace and changed production-source size checks passed; the largest
  changed production file has 329 substantive lines.

The first full database passes identified two legacy exact-data fixtures that
needed the new default/metadata; both were updated while retaining their history
assertions. The large leaderboard JSON fixture was split without increasing
compiler limits. The final full suite passed. Initial browser failures were test
harness issues (tie notation, source-module interception and a no-op stale-write
setup); the corrected scenarios passed against the implemented behavior.

**READY WITH KNOWN LIMITATIONS:** the existing build advisory remains for the
629.96 kB minified JavaScript bundle (182.10 kB gzip). Physical iOS/Safari and a
production deployment were not exercised; available browser validation used
local Chrome and a disposable API/database. No queued feature was started.
