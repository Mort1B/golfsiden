# Latest iteration — portable native-order contract test

A1v closes the retained P3 test portability issue in the legacy individual-group
contract case. The original test passed on Alpine/musl PostgreSQL 17.10 but failed
at the predicted team-ID assertion on Debian/glibc PostgreSQL 17.11. Both use
`en_US.utf8`; their native case ordering differs. This changes one test plus
evidence/docs. Production SQL, collation, API, UI, schema and dependencies are
unchanged.

Native team/member expectations now sort parameter-bound fixture values under
the database's own collation, independently of returned application rows. The
test retains both case variants, exact sequences and explicit case-folded UUID
ties for pairing groups/members. It finds the source team by UUID for timestamps
and preserves schedules, withdrawn entrants and empty-group membership.

For example, Alpine returns `B-group,b-group` while Debian returns
`b-group,B-group`; both preserve the separate pairing contract's low/high UUID
sequence. Broader locales, ICU, custom column collations and non-ASCII behavior
are not newly established.

Validation passes: the exact case and all 14 pairing-session tests on each image,
264 ordinary Rust tests, 953 PostgreSQL-feature tests, formatting, strict Clippy,
39 migrations and seed. Five existing manual probes remain ignored in the
PostgreSQL-feature ladder. Independent review found no test/source defects; its
evidence-runner count gap was repaired, exercised with negative controls and
closed. Final exact-count/cleanup checks confirm all four owned fixtures are removed.
The [acceptance report](validation/pairing-collation-2026-10-15/README.md) retains
exact environments, evidence and limits. Frontend/browser checks are outside
the affected ladder because no runtime or user-facing behavior changes.

The next candidate is the separately bounded P3 SQLx `verify-ca` contextual-name
compatibility repair. It has not started. Hosting and broader optimization remain
outside this step.
