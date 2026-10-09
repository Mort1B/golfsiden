# Fantasy recovery exits, claim revalidation and viewing context

The five verified follow-up findings are fixed within the existing recovery and
private-query boundaries.

An acknowledged Fantasy write with failed refresh no longer blocks Score/logout.
Its receipt and read-only retry remain available; successful toolbar/detail retries
clear the failure state. Unsaved input and uncertain submissions remain guarded.
Obsolete refresh feedback cannot overwrite a later operation.

Exact reconciliation now distinguishes an accepted historical request, definitive
supersession and transient conflict. The additive `409 fantasy_revision_conflict`
is returned only when immutable request history has no receipt and the expected
revision is older than current, under the existing round locks. Future revisions
and generic conflicts remain unresolved. Users can explicitly adopt the current
lineup or rebase their original picks/captain; only a later deliberate save creates
a new request. Paths, bodies, receipt DTOs, auth and deduplication are unchanged.

A registration failure before commit can recover through **Kontroller kontolenken
på nytt**. This reads the session before revalidating the original claim. An
available claim restores deliberate registration with the username retained and
password empty; an unavailable claim offers normal login. A committed response
lost with its cookie recovers the expected player session. Without the cookie,
ordinary login reaches the same prepared player. Secrets remain only in mounted
memory, and concurrent login cannot be overwritten. Registration never retries
automatically and claims remain single-use.

Manager/golfer details now open beneath their selected row. Close/Escape restores
row focus, and a selected round limits the detail to that round. Board/row/round
preferences survive query-driven unmount/reconnect as identifiers only, validate
against fresh responses, and reset at account/session boundaries. Protected
projections still clear immediately. Scoring rules and offline-write policy did
not change.

For example, an unaccepted revision-0 request can lose its response while another
session saves revision 1. Reconciliation sends the exact original ID/body, then
reports definitive nonacceptance. **Behold valgene som nytt utkast** keeps those
choices for review at revision 1 without writing; **Lagre firer og kaptein** then
creates the new request. A generic conflict never permits that inference.

Before production edits, seven actual-parent frontend regressions and the new
PostgreSQL outcome regression failed. Final validation passed: 249 backend tests;
716 tests with the database feature (3 existing exclusive performance probes
ignored); 1,014 frontend tests across 135 files; formatting, strict Clippy,
typecheck, lint, build, migration and seed; and 17 Chrome scenarios at mobile and
desktop widths. Read-only review findings were repaired and regressed.

See the [exact commands, screenshots, evidence and limits](validation/reliability-followup-2026-10-09/README.md).
No hosted or physical-device acceptance was performed. Live and response loss
were controlled boundary injections against the local stack. No queued work was
started.
