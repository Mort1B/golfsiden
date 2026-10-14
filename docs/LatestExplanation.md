# Versioned Fantasy scoring

New Fantasy games now award **20 points for an actual hole-in-one** and **5 for a
non-ace net albatross or better**. One category applies per hole: a physical ace
that also qualifies as net albatross earns 20, not 25. A net score of one alone
is not an ace. Other hole categories, placement, match awards and captain rules
are unchanged. This is a deliberate game-balance scale, not a claim of calibrated
rarity or equal expected points across handicaps.

A captained ace contributes 40 hole points; a captained non-ace net albatross
contributes 10. Selecting both partners of a shared team ace gives 40, or 60 with
one as captain. The existing shared-team attribution and actual-shot precedence
remain intact. Match play still awards only its final outcome.

Schema 39 upgrades an existing game only if no selection window has effectively
closed. An expired deadline counts even before lazy locking, as does an opened
round or materialized lock. The migration locks relevant tables and establishes
one cutoff after waits. Already-frozen games keep version 1, with +10 for both
ace and net albatross, throughout their tournament. Eligible disabled and empty
games upgrade too. Runtime changes to versions and new version-1 games are
rejected. Scores, lineups, receipts and non-finish source tokens are preserved.

Every result envelope now identifies its game's rules version and includes it
in its revision fingerprint. The UI displays version-specific rules and rejects
missing/unknown metadata or category awards inconsistent with that version.
Independently loaded boards and breakdowns must agree with the game's version;
a mismatch hides incompatible points and offers read-only refresh. Selection
recovery and privacy boundaries are unchanged.

Validation passed: 256 backend tests, 732 PostgreSQL-enabled tests, formatting
and strict Rust lint; clean schema-39 installation, populated schema-38 upgrade,
repeated seeds and restricted-runtime API checks; 1,100 frontend tests across
141 files, typecheck, lint, build and strict browser TypeScript. Three new browser
scenarios and 13 retained scenarios passed at mobile/desktop widths. A real
pre-upgrade legacy game returned exactly the same overall JSON after upgrade.
Three existing exclusive database performance probes remain ignored. Read-only
backend/frontend review found no remaining issues. See the
[acceptance report](validation/fantasy-scoring-2026-10-14/README.md).

Deploy matching API/frontend builds with schema 39 and refreshed runtime grants;
see the [deployment guide](deployment_guide.md#schema-39-versioned-fantasy-scoring).
No hosted infrastructure was changed. This completes the requested presets,
optional net awards and Fantasy scoring sequence; deferred work remains separate.
