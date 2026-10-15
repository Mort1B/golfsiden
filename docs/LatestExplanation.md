# Latest iteration — session-bound course reads

A1n repairs the final three P3 routes in the private-read inventory: course
catalog, registered presets and provider-course detail GETs. They retain the
actual session and exact tournament-admin membership, with final database
wall-clock expiry checks. Every selected-route status is private/no-store.
The bundled catalog, presets, provider normalization and response contracts are
unchanged. No schema, frontend, course-configuration write or provider settings
changed.

Catalog query validation and local search run inside the shared course-read
authority transaction. Presets use the same owner around their existing ordered
SQL/assembly. Empty results still require authority. Direct/nested serialization
conflicts discard the local attempt and retry once, then return generic `503`.
The historical internal user-only preset helper retains its contract and shares
the same projection, including for schema-20 upgrade fixtures.

Provider detail commits session/admin preflight before readiness/cache/network
work, then separately reauthorizes the same session/tournament before exposing
success or error. No database lock spans external I/O. The one-shot fetch future
sits outside both local retry loops, so a conflict never repeats fetching, quota
or cache work. A legitimately admitted fetch may warm the bounded global cache
or consume quota even if postflight denies its response.

Before repair, controlled late revocation, credential loss and expiry still
returned catalog/presets `200` or provider `409` incomplete. All now return `401`.
Preset-loading expiry also returned `200`; populated and empty cases now return `401`.
Existing preset membership conflicts returned `500`; the new full local retry
reauthorizes and returns `403` after demotion, or `503` after repeated conflicts.
Seven original regression tests failed as expected. The final focused run passes
eight real-GET cases with expanded snapshot/no-write assertions, five conditional-
provider cases and six existing course contract tests. Backend formatting, 264
unit tests and strict Clippy pass.

The catalog still has no usable provider entry. Remote/cache success is covered
only through the same API orchestration helper with local doubles, not a live
successful provider GET. The tests hold real loopback HTTP while authority writers
finish, verify both success/error suppression after loss, and prove two local
postflight attempts execute fetching only once. No external provider was called.

The full PostgreSQL ladder passes 946 tests, zero failures, with five existing
manual/exclusive probes ignored. Migrate and seed pass with all 39 migrations.
The final focused run separately validates the expanded assertions added during
that full run. All owned PostgreSQL fixtures were removed.
The [A1n report](validation/course-read-session-2026-10-15/README.md)
records the boundaries, baseline evidence, commands and limits. This closes only
the inventoried GET family, not unassessed legacy writes or hosted security
acceptance. Final independent evidence/plan review found no issues. The [plan](PLANS.md)
closes A1n and proposes a documentation-only legacy-write authority inventory;
no adjacent assessment or implementation began.
