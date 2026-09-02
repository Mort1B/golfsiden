# Latest explanation

## Release readiness became an implementation boundary

This iteration did not add a golf feature. It converted the implemented private
tournament application into a reproducible, recoverable production system and
tested the existing product through that system. Findings were triaged as P0–P3;
all release-blocking P0/P1 findings were fixed, while low-risk release hardening
was preferred over refactoring or speculative optimization.

The initial baseline already had strong tournament authorization, CSRF, session,
score-authority, SQLx transaction, SSE, and final-round visibility coverage. Its
release blockers were operational: no production topology, no request throttle
around credential work, no production schema/role gate, and no exercised
backup/restore procedure.

No P0 finding was identified. Resolved P1 findings were the absent deployable
production/recovery baseline, an initial container toolchain that could not build
the locked Rust graph, non-portable PostgreSQL initialization wiring, a limiter
design that let one client exhaust a route-wide ceiling, and configuration that
could collapse the owner and runtime database roles. Resolved P2 findings
included restore atomicity/pristine-target validation, production environment
ignore and hostname validation, trusted-proxy identity, runtime authority and
migration-history protection, portable dump checksums, bounded request and
Argon2 work, and missing HTTP-level throttle coverage. P3 refactoring and
speculative performance work remained outside the release window.

## The production path is deliberately small

`compose.production.yml` now builds three boring images. Caddy serves the real
Vite output and proxies same-origin `/api` and SSE over HTTPS. The Rust image
contains only the release API and migration binary. PostgreSQL 17 uses a durable
volume and an internal-only network; its image contains reviewed role/grant
scripts rather than relying on host bind-mount execution.

Production configuration fails closed. Secure cookies, explicit external
migrations, a bounded connection pool, a valid HTTPS CORS origin when present,
and a 256-bit proxy secret are enforced. Caddy overwrites the internal client-IP
header and supplies that secret; untrusted forwarding headers cannot select a
rate-limit identity. Liveness is database-independent, while readiness proves
database reachability and exact embedded SQLx migration history. The normal API
role can use application data but cannot create or alter schema or write SQLx
migration history. Startup verifies the connected identity equals the configured
runtime role and rejects owner or privileged credentials.

The migration action is explicit and idempotent. The API refuses missing,
pending, unknown, dirty, or checksum-mismatched migration history before it
binds. Production images omit the seed binary, and the runbook explicitly
forbids development seed data.

## Abuse controls bound the expensive paths

Login, creator onboarding, invitation preview, invitation registration, and
invitation acceptance now have stable two-level rate limits: a narrow
client/resource bucket plus a broader client ceiling. Storage is capped and
stale buckets are evicted. A rejection returns `429`, `Retry-After`, the existing
JSON error envelope, and `no-store` without charging a broader bucket merely
because one narrow key is exhausted.

Login also rejects unreasonable syntax and bodies before database or password
work. Argon2 verification is cancellation-safe and limited by one shared
four-task semaphore, preventing a credential burst from creating unbounded
blocking work. Development remains unthrottled unless tests select the limiter
explicitly.

## Recovery was proved, not described hypothetically

The backup script writes a custom-format, no-owner/no-privilege PostgreSQL dump
atomically and creates a basename-relative SHA-256 sidecar that remains valid
when the pair moves to off-host storage. Restore requires a deliberate
confirmation phrase, checks the digest, refuses any nonempty public schema,
restores with `--single-transaction --exit-on-error`, and reapplies runtime
grants.

The production-like recovery drill backed up a representative database with 11
users, 2 isolated tournaments, 10 players, 6 rounds, 41 scores, active sessions,
an active main tournament, and a hidden final back nine. All containers and
named volumes were removed, PostgreSQL was initialized on fresh storage, and the
dump was restored. The release API then became ready, the preserved administrator
session still authenticated, every count and lifecycle/visibility fact matched,
and the order-stable score digest remained
`355f01af2d4ef812b9f33c3130e23987`. A second restore correctly refused the 191
objects already present.

The final privilege drill proved the runtime role could read all 18 migration
rows while both migration-history writes and public-schema creation failed with
PostgreSQL permission errors. A production API started with the owner URL failed
closed as `UnexpectedRole`; the normal runtime deployment became ready. The
PostgreSQL image also rejected example placeholder credentials before startup,
and a moved dump/sidecar pair verified successfully before the nonempty-target
restore guard refused it.

## Production-like product acceptance

The built frontend and release backend were exercised through Caddy HTTPS with
independent Chrome profiles at phone and desktop widths. The flow covered
tournament start, round opening, invitation issue/preview/registration, creator
onboarding into a second tournament, scramble, individual, and foursomes score
ownership, gross/net round and tournament standings, mobile score entry, logout
cache clearing, and SSE-authoritative refresh.

Phase 7C was rerun with separate exact-admin and ordinary-player sessions. Hidden
reads exposed 9 holes and 4/9 progress; release exposed all 18 holes, the scored
back-nine fact, and 5/18 progress; re-hide removed that fact before the blocked
authoritative refetch completed, then returned the player to hole 9 and 4/9.
The same script checked loading/saving states, a 44 px control, keyboard focus,
mobile/desktop overflow, network failures, API statuses, console errors, and
uncaught exceptions.

Bidirectional identifier substitution between the two tournaments covered
tournament, round, roster, score-access, scorecard, leaderboard, result-history,
invitation, score-mutation, and visibility paths. Every protected request failed
with `403` or the endpoint's non-disclosing `404`; missing CSRF also failed.

A realistic local burst connected 40 SSE clients, submitted 32 scores and 12
same-card conflict writes concurrently, and made 160 gross/net leaderboard
reads. Every request succeeded and all streams received an event. The repeat run
measured about 228 ms p50, 264 ms p95, and 267 ms maximum read latency. A coarse
sample showed roughly 60 MB API, 87 MB PostgreSQL, and 63 MB Caddy memory with 11
database connections. A live browser reconnected its EventSource after an API
restart and retained the scorecard; proxy and database restart tests also
recovered without losing state. With PostgreSQL stopped, liveness stayed `200`
and readiness became the intended non-secret `503`.

The final post-remediation ladder passed 114 backend tests and all 303
PostgreSQL-enabled tests, strict all-target/all-feature Clippy, the complete Rust
release build, 226 frontend tests across 40 files, typecheck, lint, and the Vite
production build. Clean and current migrations plus the idempotent development
seed passed on disposable PostgreSQL. Compose configuration and builds passed for
the PostgreSQL, release API, and Vite/Caddy images, and every deployment shell
script passed syntax validation. Both npm audits reported zero vulnerabilities.
The final read-only review found no remaining P0, P1, or P2 issue.

## Known limitations

The Vite bundle remains about 555 kB minified (161 kB gzip) and produces its
existing chunk-size warning; measured tournament latency did not justify release
window code splitting. `cargo audit` still reports RSA `RUSTSEC-2023-0071`
through SQLx's unused MySQL macro dependency and a yanked `chacha20` version
through an inactive dependency graph. The application builds PostgreSQL-only
SQLx and the affected RSA decryption path is not used. npm's reachable `nanoid`
advisory was fixed by updating the lockfile to 3.3.18, and npm audit is clean.

The baseline is intentionally one API instance on one host. Horizontal scaling
would require shared rate-limit and course-provider quota state. Availability
across total host loss depends on operators scheduling, monitoring, and copying
verified backups off-host as documented.
