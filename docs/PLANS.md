# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. See the [latest explanation](LatestExplanation.md) for the completed step.

## Next candidate

**Local private/public result-projection assessment (awaiting approval).**

Goal: verify that private result reads require exact tournament access and public
capabilities expose only the intended projection. Scope source, existing tests
and controlled disposable local validation of grant scope, expiry/revocation,
rotation, response allowlists and concurrent membership changes.

Behavior/invariants: assessment only; no application, migration, dependency or
operator repairs. Preserve gross/net semantics, historical snapshots and private
account/scorecard boundaries. Use only synthetic accounts and disposable local
services; do not access production, external targets or real credentials.

Validation: inspect handlers, repositories, decoders and existing privacy/race
tests; reproduce concrete concerns where warranted. Report confirmed findings
with source/impact/recommendation, separate unverified concerns, record limits
and exact safeguard notices, and obtain independent read-only review.

Stop after the report and one bounded next candidate. Keep browser-persistence,
operational/dependency assessment and any remediation separate. Work and commits
remain local under the existing scope.

## Later queue

No queued work is currently approved.

- Remaining broader security assessment: browser/offline persistence,
  production proxy/database privileges, recovery operations and dependency advisories. Define a bounded next scope and
  authorized environment before proceeding.

### Remaining deployment acceptance

Public-host acceptance still needs a Docker Engine environment and public DNS/TLS
validation. Native 200% browser zoom and physical Android Chrome also remain
unverified. Confirm the available host/browser/device environment before defining
the next bounded acceptance step. Keep the separate security work above
distinct from these operational and device acceptance checks.

No automatic opponents/byes/brackets, team match play, extra holes, new scoring
rules, public match sharing, cold offline launch or background sync is included.
