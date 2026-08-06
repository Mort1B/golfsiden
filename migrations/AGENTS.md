# Migration Agent Rules

These rules apply to `migrations/` in addition to the repository contract.

- Published migrations are append-only. Never edit, reorder, or rename an applied
  migration; add a new forward migration instead.
- Migrations must work on a clean database and an existing database at the
  immediately previous schema version.
- Use PostgreSQL constraints for critical invariants: foreign keys, uniqueness,
  exclusive ownership, valid ranges, and lifecycle protections.
- Cross-table consistency must be enforced structurally when practical, using
  composite keys or triggers, not only handler checks.
- Every foreign key needs an intentional delete behavior. Defaulting silently to
  restrictive behavior is not a design decision.
- Add indexes for demonstrated lookup and ordering paths. Do not add speculative
  indexes without naming the query they support.
- Schema changes that transform data must be transactional, deterministic, and
  explicit about rollback or recovery risk.
- Triggers must remain narrow, documented, and covered by PostgreSQL integration
  tests. Avoid hidden business calculations in triggers.
- Seed data is development-only, deterministic, idempotent, and representative of
  round-specific team changes. It must never bypass production constraints.
- Validate migrations and seeds against the supported PostgreSQL version. SQL
  review alone is insufficient.
