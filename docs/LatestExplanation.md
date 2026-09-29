# Draft tournament detail editing

Exact tournament administrators can now correct a draft's name, description and
start/end dates under **Turneringsstyring → Innstillinger → Navn og datoer**.
The date range must contain every configured round; no round is moved. Active,
completed and archived tournaments show a read-only explanation. Wider schedule
changes, formats, teams, score data and public sharing are outside this iteration.

A strict versioned PATCH endpoint normalizes the four editable fields. Its
transaction locks rounds, authority and tournament in the established order,
revalidates the exact membership/session and checks expiry after possible waits.
An unchanged request preserves its timestamp; a changed commit emits one live
invalidation. Migration 33 adds independent context, status and date guards,
preserving historical data. Review exposed a snapshot-isolation containment
race; changed details now require READ COMMITTED, with a deterministic regression.

The form retains text after failed saves. Competing edits require explicit discard
and reload; it never silently merges or overwrites them. A save receipt appears
only after authoritative refresh. Accepted writes whose refresh fails require a
reload before another write. Tournament/user/session ownership prevents departed
callbacks from repopulating a different editor or private cache.

Example: an organizer can rename a draft golf trip or extend its end date. Moving
its start past the first round is rejected. If another administrator saves first,
the local text stays visible until the organizer chooses **Forkast utkast og hent
siste**. Starting the tournament closes this basic editing path.

Validation: 215 backend tests; all 629 PostgreSQL-enabled checks pass across the
full run and the corrected fixture target's rerun; 897 frontend tests; formatter,
Clippy, typecheck, lint and build passed. Clean migration, schema-32 preservation
and unchanged seed rerun passed. Real Chrome/real API checks cover 320/390/1280px,
async/error/conflict states, persistence and start. Read-only review found no
remaining actionable issue. See the [validation evidence](validation/tournament-details-2026-09-29/README.md).

**READY WITH KNOWN LIMITATIONS:** deploy migration 33 and matching API/web builds.
Hosted deployment remains user-owned. The next planned step is return to an
already prepared scorecard during a coverage gap while the app remains open;
that offline implementation has not started.
