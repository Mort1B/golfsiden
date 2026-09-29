# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. Continue hands-on testing using the [checklist](testing_checklist.md).

## Next candidate

A concrete bug from testing: define one bounded repair from its reproduction,
prioritizing blocked workflows, missing saved data or incorrect access. No further
broad assessment or speculative callback repair is a prerequisite for testing.
Hosting and deployment at gg26.no remain user-owned.

## Later queue

- Deferred: final-round visibility, pairing editor and tournament-start callback
  lifetime concerns. These are unverified follow-ups, not automatic prerequisites
  for testing; retain the [existing evidence](validation/stableford-settings-lifetime-2026-09-23/README.md).
- Deferred broader security assessment: further production proxy/database privilege,
  recovery and dependency-advisory investigation. Revisit for a specific finding
  or a separately requested assessment, rather than making another broad review
  a prerequisite for continued functional testing.

### Remaining deployment acceptance

The remaining PERSIST-1 same-account/actual-login scenarios may be revisited if
testing feedback warrants them; see the original
[validation limits](validation/match-note-retention-2026-09-23/README.md). Local
PostgreSQL validation is now available through rootless Podman. The functional
checks cover real database-backed match recovery after locking, but do not claim
every separate session-replacement scenario.

Public-host acceptance still needs a Docker Engine environment and public DNS/TLS
validation. Native 200% browser zoom and physical Android Chrome remain unverified.
Hosting and deployment checks belong to the user. These historical acceptance
limits do not expand the application-readiness step into server administration
or another assessment. Broader device coverage and polish can follow feedback.

No automatic opponents/byes/brackets, team match play, extra holes, new scoring
rules, public match sharing, cold offline launch or background sync is included.
