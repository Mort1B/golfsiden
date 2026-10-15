# Latest iteration — bounded diagnosis and Sunday deployment path

A1z investigated the retained default-concurrency failure in the unchanged
match-lifecycle late-expiry case. Two default lifecycle-module runs pass all six
cases, the exact match case passes alone, and the two-thread module control
passes all six. No panic, early-response error, gate timeout or rollback failure
was observed. Source fingerprints and exact counts/target execution match; the
owned PostgreSQL and private temporary files are removed.

Independent source review found no actionable match-specific defect. The test's
late trigger follows real snapshot/status work, rejected expiry must roll back
all preserved state without SSE, and valid controls traverse the same gate and
commit exactly one event. The original full-run receipt retained no panic/action
or timing details. Its cause remains unknown, and the unchanged helper supplies
no new gate/deadline timestamps. These successes bound non-reproduction; they do
not turn the earlier failed full default ladder into a pass. No speculative fix,
artificial delay, timeout increase, retry or ignored test was introduced.

The user confirmed one host with the supplied Docker Compose setup and asked for
production-ish use by Sunday 18 October within 5–8 iterations. The queue now has
a six-iteration target including this one, with two contingency slots for
confirmed launch blockers. Next are current production image/configuration
acceptance, integrated Caddy/production-mode/runtime-role rehearsal, critical
two-account user flows, actual current-schema recovery, then freeze and a clear
readiness verdict. New features, speculative optimization and broad polish are
deferred. Test-maintenance work will not automatically consume further iterations
without actionable evidence or a required validation blocker.

The deployment review established evidence gaps, not new P0–P2 product defects:
current Docker image builds (including the image's Rust 1.88 versus recent host
Rust 1.99 acceptance), production proxy/runtime-grant integration and current
application restore still need direct acceptance. Actual host/DNS/TLS, secrets,
storage, off-host backups and playing-phone checks remain operator-owned.

No existing application/test, schema, dependency or deployment source changed,
so unaffected full Rust/frontend/browser ladders were not repeated. Fresh
focused PostgreSQL checks, receipt postconditions, Python syntax and docs/diff
checks pass. Earlier full two-thread 953-test database acceptance remains
separately linked evidence. The
[report](validation/lifecycle-expiry-investigation-2026-10-15/README.md) records
commands, reviews, limits and launch priorities; the
[plan](PLANS.md) defines the bounded next production-image step.
