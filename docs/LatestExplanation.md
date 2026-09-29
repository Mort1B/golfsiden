# Scoring-card reads recover from an authority update

The scoring-card GET now retries its complete database read once when PostgreSQL
rejects its snapshot after a concurrent authorization-row update. For example,
an otherwise harmless membership update previously caused HTTP 500; the fresh
attempt now returns the saved card. Concurrent logout or removed permission
returns the usual 401/403 instead of a generic error.

Each attempt repeats all access checks and preserves the existing isolation and
share locks. A second serialization conflict returns a non-cacheable 503 without
card data. Other errors are not retried. Scores, audits, confirmations, score
writes, API fields and scoring rules are unchanged.

Six deterministic PostgreSQL/API regressions failed before the fix and passed
afterward. They cover session revocation, membership removal, individual/team
recovery, the two-attempt limit and unrelated errors, while asserting unchanged
stored score state and no emitted score events. Real Chrome checks passed at
320px, 390px and 1280px using actual database contention: valid scoring recovered,
logout removed the scoring UI, and a viewer downgrade removed editing controls
while preserving allowed read-only access.

**READY** for this bounded fix. Validation passed: 215 ordinary backend tests;
621 database-enabled tests (including those ordinary tests); 806 frontend tests;
formatting, strict Clippy, frontend typecheck/lint/build, fresh migration and seed,
and the three real-browser scenarios. Three existing performance measurements
remain explicitly ignored because their separate measurement setup was not
configured. Independent read-only review found no blocking issues.

The [validation report](validation/score-read-retry-2026-09-29/README.md) records
full validation results, independent review, screenshots and the failure-first
evidence. The earlier functional-readiness run's exact overlapping action was
not captured; this repair covers the reproduced serialization-conflict class
for this endpoint. Repeated contention can still return the deliberate 503.

Continue testing with the [short checklist](testing_checklist.md). Deploy the
updated API normally; no migration or configuration change is required. Hosting
at gg26.no remains user-managed and was not accessed. Physical-device and hosted
acceptance remain outside these local checks. Broader assessment and other
queued concerns remain deferred pending concrete testing feedback.
