# Local authentication assessment found two boundary defects

The authorized local review of authentication, sessions, CSRF and tournament
authorization is complete. The [report](validation/authentication-2026-09-23/README.md)
records source references, reproducible diagnostics, impact and recommended fixes.
Application source, migrations and dependencies are unchanged.

AUTH-1: requests rejected by one route can still allocate limiter buckets and
evict another route's active counters. The actual production limiter accepted a
previously blocked login after 615 ms instead of preserving its 60-second window.

AUTH-2: a handicap correction that was initially authorized can wait on a database
lock and commit after its session expires. The real router and disposable
PostgreSQL reproduced the update, audit entry and invalidation; the next session
read returned 401. A nonexpiring control passed. This does not demonstrate bypass
of logout, CSRF or exact tournament membership.

Validation passed 206 Rust library tests, 35 PostgreSQL integration tests and 18
frontend auth/cache tests. Independent read-only reviewers checked the boundaries
and evidence. Initial local socket restrictions were resolved; no platform
cybersecurity safeguard blocked this run. The report distinguishes the findings
from unverified read/logout semantics and remaining review areas.

Deployment remains **NOT READY** pending these repairs and the existing wider
security, public-host and device/browser gates. AUTH-1 is the proposed next bounded
step; remediation requires separate approval. This assessment stays local under
the user's scope, without external publication.
