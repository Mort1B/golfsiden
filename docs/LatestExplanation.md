# Profile presentation and self-service handicap audit

The profile now starts with “Mine turneringer”, including historical membership
links and tournament creation. Name and handicap remain visible below the list.
“Endre brukernavn” and “Endre passord” start collapsed and use native disclosure
controls with keyboard operation and visible focus. Mutation status and errors
remain visible when a section closes.

Self-service handicap editing no longer asks for an explanation. The details API
accepts version, player timestamp, name, and handicap; the repository writes
“Egen profilendring” into the existing history in the same transaction as the
change, retaining actor and timestamps. No migration was needed. Administrator
corrections to tournament handicap still require an explicit reason. Existing
entries, snapshots, score data, and historical ownership remain untouched.

For example, changing profile handicap from 8,2 to 14,4 needs no reason field and
creates an audited current-handicap change. A tournament already entered at 8,2
keeps 8,2, including its preserved results.

Password advice now suggests a long password or phrase. Profile, creator
onboarding, and invitation registration share a validator that preserves the
12–128 UTF-8-byte contract and whitespace. Invalid values get distinct short/long
messages with the exact applicable limit; normal guidance contains no encoding
explanation. Character-based HTML length restrictions no longer reject valid
multibyte passwords or truncate overflow before it can be explained. Current
password verification, duplicate username errors, session invalidation, stale
versions, and private-cache handling retain their existing behavior.

## Validation and review

- Frontend: 368 tests passed; typecheck, lint, production build, and separate
  browser-test TypeScript compilation passed.
- Rust: 114 tests passed with `cargo test --workspace --all-targets`; formatting
  and strict all-target/all-feature Clippy passed.
- PostgreSQL: the full database-enabled ladder passed 355 tests (including the
  114 unit tests). A fresh disposable PostgreSQL 17 container on port 55432 was
  migrated and seeded successfully. Tests cover self-only/CSRF checks, rejected
  caller-provided reasons, audit actor/timestamps, initial player history,
  stale/concurrent edits, immutable tournament results, and required reasons for
  administrator tournament corrections.
- Chrome: profile editing, collapsed sections and keyboard use, password overflow
  while collapsed, historical handicap preservation, duplicate/wrong-password/
  stale errors, logout on all devices, loading/retry/empty/inactive states, and
  long content passed at 320px, 390px, and 1280px. Creator and invitation flows
  exercise overflow rejection and real registration with a valid multibyte
  minimum password. Screenshots are under `/tmp/golf-profile-*.png`.
- Read-only specialist review found no correctness, security, or invariant
  findings. Its requested multibyte registration coverage was added.

Initial socket-dependent Rust checks were blocked by the sandbox and passed
with local socket access enabled. Docker access was unavailable; validation used
an isolated rootless Podman container. Initial test failures exposed a remaining
Rust test caller, native-disclosure limitations in jsdom, strict test typing,
and browser assertions that needed to distinguish simultaneous alerts and
expected signed-out 401 responses; these were corrected without weakening the
application contract. No required validation gate was skipped.

## Delivery limits

Verdict: **READY** for this bounded step. The frontend build retains its existing
large-chunk advisory. Deploy the frontend and backend together: older open
clients sending the removed `reason` property receive a validation error and
must reload. This iteration does not deploy production or change administration,
scoring navigation, or tournament leaderboard behavior; those candidates remain
queued in `PLANS.md`.
