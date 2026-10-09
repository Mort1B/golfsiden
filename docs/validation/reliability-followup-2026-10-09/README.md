# Five reliability/usability follow-ups — 2026-10-09

Bounded follow-up to `6a520dd`. Local working-tree acceptance against a fresh
rootless Podman PostgreSQL database, `reliability_followup_20261009`, the updated
API and Vite, with installed Google Chrome. No production data or hosted service
was used. No scoring, handicap, authorization, privacy-clearing, single-use-claim
or deduplication rule changed. No migration or runtime-grant change is needed.

## Verified before production edits

The first actual-parent regression run had **7 failures / 30 passes**, across
`FantasyPage.reliability.test.tsx`, `FantasyViewing.test.tsx` and
`ClaimExperience.test.tsx` (the latter mounts `PlayerClaimPage`). Failures were:

- Acknowledged refresh failure blocked departure.
- Successful toolbar refresh left stale failure feedback.
- Superseded uncertain request offered no explicit rebase.
- Pre-commit registration 503 offered no claim revalidation.
- Detail rendered outside its row and lacked the tested close/focus interaction.
- A round selection still expanded another round.
- Privacy clearing/reconnect lost the viewing preferences.

Initial PostgreSQL regression: **1 failed / 1 passed**. Two sessions of the same
manager demonstrated that an unaccepted older request returned generic
`fantasy_conflict` rather than the definitive supersession classification. The
serialization-conflict control remained generic as expected. Local initial logs:
`/tmp/golf-followup-before.log`, `/tmp/golf-followup-db-before.log`.

## Final changes and evidence

| Finding | Fix and regression evidence |
| --- | --- |
| Acknowledged writes trap navigation | Guards protect writing, unsaved input and uncertain requests. A read failure alone allows Score/logout. Receipt/read-only retry remain; toolbar and inline detail/source retries use the shared refresh outcome. Actual-page tests cover toolbar/detail success; Chrome exercises both exits with exactly one write. |
| Superseded requests remain uncertain | Under existing round locks, immutable request lookup precedes definitive old-revision rejection. New `409 fantasy_revision_conflict` means no receipt exists and the unchanged request cannot subsequently succeed. Generic conflicts remain uncertain. Explicit current-lineup/rebase choices issue no write; a later deliberate save gets a new ID and fresh revision. Real two-session Chrome and PostgreSQL tests assert exact original request identity and the subsequent new request. |
| Pre-commit registration failure has no recovery | Ambiguous registration reads the session, retains the original token only in mounted memory and offers deliberate revalidation. Available preview restores username/empty-password registration; consumed claim retains login guidance. Parent tests cover pre/post-commit outcomes, transient preview failure, concurrent login and unmount. Chrome tests actual committed registration with/without cookie, consumed preview, ordinary login, and injected pre-commit 503 followed by explicit fresh registration. |
| Details hard to discover | Selected detail is inside its leaderboard row with `aria-expanded`/`aria-controls`, a named region, close/Escape and focus restoration. A selected round filters both manager and golfer detail. Actual-page tests assert row containment and filtering; Chrome covers both boards and keyboard behavior. |
| Viewing context lost on query unmount | Provider owns only board kind, selected row/round IDs and lineup-round ID. Fresh inventories validate/prune IDs; missing/error projections do not erase preferences; user/CSRF changes reset them. Parent tests cover reconnect, removed inventories and account/session replacement. Chrome checks both board kinds/round selection through explicit private clearing. |

The four new PostgreSQL tests additionally cover an original pick withdrawn
before a competing save, historical accepted replay after a newer save,
changed-body request collision, a future expected revision that can later succeed,
and SQLSTATE 40001 remaining generic. A held manual-refresh regression through
FantasyPage proves obsolete refresh feedback cannot overwrite a later uncertain
write. No acknowledged refresh failure invokes another write.

The only API classification change is the additive error code above: status 409,
error envelope, paths, write bodies and successful receipt DTOs remain unchanged.
Old clients stay conservative for this new code. Deploy matching builds to expose
the explicit recovery choices.

## Validation commands and results

| Check | Exact result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `cargo test --workspace --all-targets` | 249 passed, 0 failed |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Passed; existing three vendored SQLx warnings remain |
| `DATABASE_URL=<disposable-local-db> cargo test --workspace --all-targets --features database-tests -- --test-threads=4` | 716 passed, 0 failed, 3 ignored exclusive pg_stat_statements probes |
| `DATABASE_URL=<disposable-local-db> cargo run -p golf-api --bin migrate` | Passed; migrations current |
| `DATABASE_URL=<disposable-local-db> cargo run -p golf-api --bin seed` | Passed; eight players and five rounds seeded |
| `npm run test` | 135 files, 1,014 passed, 0 failed (55.95s) |
| `npm run typecheck` / `npm run lint` / `npm run build` | All passed; build 3.83s |
| Full affected Chrome suite below | 17 passed, 0 failed (1.4m) |

Local logs: `/tmp/golf-followup-{backend,clippy,database,migrate,seed,frontend,typecheck,lint,build,browser}.log`.
The database count includes the backend tests; these counts are not additive.

Chrome command (from `frontend`):

```sh
GOLF_RELIABILITY_BROWSER=1 GOLF_FANTASY_BROWSER=1 GOLF_PLAYER_CLAIMS_BROWSER=1 \
  npm run test:browser:lifecycle -- reliability.browser.ts \
  reliabilityFollowup.browser.ts fantasy.browser.ts fantasyPrivacy.browser.ts \
  fantasyRelease.browser.ts fantasyReleaseFormats.browser.ts playerClaims.browser.ts
```

The initial focused 8-scenario Chrome set passed in 30.1s. The full affected set
passed **17/17 in 1.4m**, including previous scoring-format, nine-team changing
partners/carry-forward, privacy and single-use-claim acceptance. Console and
network assertions passed. The base Fantasy scenario decoded three retryable
`409 fantasy_conflict` result reads and recovered; zero canceled 409 bodies were
unverified there. The added recovery tests explicitly inject 503 reads, an aborted
lineup response, and malformed registration responses; expected 401/410 and
transient result conflicts are handled separately from unexpected failures.

Widths were 320, 390 and 1280 CSS pixels. Layout checks assert no horizontal
page overflow and existing Fantasy controls at least 44px tall. Screenshots were
visually inspected for long names, wrapping and inline detail layout:

- [Inline golfer, 320px](inline-golfer-320.png)
- [Inline manager, 1280px](inline-manager-1280.png)
- [Available claim after revalidation, 390px](claim-available-390.png)
- [Superseded original request, 320px](superseded-320.png)

Read-only review found three bounded issues during implementation: future revision
classification, stale revision masked by golfer eligibility, and local detail
retry bypassing action feedback. All were repaired and regressed. Final review
reported no remaining findings. Its held-refresh coverage suggestion was then
covered by the passing parent regression. Changed production files remain below
400 nonblank/noncomment lines; `git diff --check` passed.

## Limits

Live loss/reconnect is exercised by dispatching controlled error/open events on
actual EventSource instances; this proves parent clearing/restoration, not a
physical packet-loss experiment. Registration response loss is induced after a
real API commit; the no-cookie scenario uses a separate request context so the
browser truly receives no cookie. Pre-commit 503 is injected before the request
reaches the registration handler. These are controlled boundary failures.

Concurrent-login protection and out-of-order refresh completion are covered by
parent tests, not separately orchestrated across physical browsers. No physical
Android/iOS device, native 200% zoom, hosted DNS/TLS, production load test or new
backup/restore rehearsal was performed. Existing exclusive pg_stat_statements
performance probes remain ignored; they are outside this bounded reliability step.
In-memory recovery/preferences still do not survive a deliberate reload, page
closure or session replacement; no offline Fantasy write or durable secret store
was added.
