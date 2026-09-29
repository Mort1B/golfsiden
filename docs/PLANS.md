# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. No implementation step is currently approved.

## Next candidate

**TEST-READY-1: functional readiness for continued hands-on testing.**

Goal: verify that the existing application supports the main workflows so the user
can keep testing it with others at gg26.no. The user owns hosting and deployment.
Prioritize working functionality over further broad assessments, speculative
repairs or polish.

Environment: use dedicated local test services and synthetic accounts for agent
validation. Confirm available services before setup; preserve existing databases
and user test progress. The user reports an existing deployment at gg26.no and
will handle its setup. No remote access, hosted mutations, production migrations
or server administration are part of this step.

Scope and behavior: run the existing application with PostgreSQL and verify login
as organizer and player, tournament setup/start, round opening,
score entry/editing, confirmation, and gross/net results. Exercise existing team
and individual paths with administrator-managed teams. Check that saved scores
survive reload, sign-out/sign-in and a local service restart. Provide a short
manual checklist the user can repeat after deploying.

Validation: exercise real database-backed browser flows at mobile and desktop
widths, including relevant loading, empty, error and populated states; check
console/network failures. Fix only reproduced defects that block these flows or
compromise authorization or saved data; run the affected validation ladder and
review for any repair. Record exact blockers and untested paths without turning
unrelated findings into new prerequisites.

Invariants: preserve all scoring, handicap-snapshot, team ownership, round-lock and
authorization rules. Keep test data across ordinary restarts; resets must be
explicit. No new features, scoring rules or broad refactoring.

Stop condition: the core flows work in the validated environment, saved data
persists, and the user has a concise checklist and known limitations for continued
testing on their deployment. Stop and hand over; subsequent work follows concrete
testing feedback. If local validation is blocked, report the exact unverified
paths without substituting mocked results or claiming readiness. This is
application test readiness, not hosted deployment sign-off or an exhaustive
security assessment. Publish completed validated repository changes normally.

## Later queue

- Bugs reported during hands-on testing: prioritize broken core flows, lost data
  and confirmed authorization defects; address one bounded repair at a time.
- Deferred: final-round visibility, pairing editor and tournament-start callback
  lifetime concerns. These are unverified follow-ups, not automatic prerequisites
  for testing; retain the [existing evidence](validation/stableford-settings-lifetime-2026-09-23/README.md).
- Deferred broader security assessment: further production proxy/database privilege,
  recovery and dependency-advisory investigation. Revisit for a specific finding
  or a separately requested assessment, rather than making another broad review
  a prerequisite for continued functional testing.

### Remaining deployment acceptance

Repeat the PERSIST-1 actual-login and database-backed match browser controls once
local Docker permissions allow a disposable PostgreSQL service. The repair has
unit/component and real Chrome evidence with synthetic API responses; see its
[validation limits](validation/match-note-retention-2026-09-23/README.md).

Public-host acceptance still needs a Docker Engine environment and public DNS/TLS
validation. Native 200% browser zoom and physical Android Chrome remain unverified.
Hosting and deployment checks belong to the user. These historical acceptance
limits do not expand the application-readiness step into server administration
or another assessment. Broader device coverage and polish can follow feedback.

No automatic opponents/byes/brackets, team match play, extra holes, new scoring
rules, public match sharing, cold offline launch or background sync is included.
