# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. Continue testing using the [checklist](testing_checklist.md). The following
steps are planned; neither is implemented or started automatically.

## Next candidate

**TOURNAMENT-EDIT-1: edit basic tournament details (planned, not implemented).**

Goal: let an exact tournament administrator correct the name, description and
start/end dates of a draft tournament without recreating it.
Scope/behavior: version-checked atomic update and mobile management form with
validation, save receipt, conflict refresh and explicit retry. Require nonempty
name and ordered valid dates that contain every configured round date; reject
a narrower range rather than silently moving any round. Active/completed/archived
tournaments remain read-only for this first step. Do not alter round count, format, teams, scoring settings,
handicap snapshots, saved results or public-sharing behavior.
Validation: API/database tests for exact-admin access, status/version races and
invalid dates; frontend failure/conflict tests; mobile/desktop browser checks;
complete affected ladders and documentation. Stop after this bounded edit flow
is published; wider schedule/round restructuring needs its own step.

## Later queue

**OFFLINE-RETURN-1: return to a prepared card during a coverage gap (planned).**
Goal: allow explicit return to the same account's exact previously loaded card
and hole while the app remains running. Scope: existing stroke/team, four-ball
and Stableford scoring-route selection/cache coordination only; formats and
public sharing stay unchanged. Retain local queued edits and original conditional
revisions; label entry as pending server verification. Do not infer fresh access,
restore hidden results or permit offline confirmation. Fresh denial/lock evidence
and account changes invalidate eligibility; absent cached cards require connection.
Keep ordinary online resume's fresh-read behavior.
Validation: reproduce current navigation failure, then browser loss of connection,
several queued holes, route departure/return, more entries and reconnect with exact
server values; test denied/locked/expired/changed accounts and no cached card at
phone/desktop widths plus the affected full ladder. Stop after this return path
is verified and published. It does not provide offline reload or closed-app launch.
See the [course-connectivity assessment](validation/offline-course-assessment-2026-09-29/README.md).
Reliable reopening after browser closure/eviction needs a separately scoped
prepared app shell and private scorecard-retention/reauthorization design.

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
