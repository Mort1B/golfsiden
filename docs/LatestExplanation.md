# Prioritize working functionality for hands-on testing

The next priority is working functionality so the user can keep testing from a
phone and with other testers. The user reports hosting at gg26.no and will handle
deployment themselves. `docs/PLANS.md` now defines one bounded application
test-readiness candidate instead of another callback investigation.

Practical acceptance covers organizer/player login, tournament setup and start,
round opening, score entry and editing, confirmation, and gross/net results. It
includes real database-backed local browser flows and persistence across service
restarts. The handover provides a concise manual checklist and known limitations
for the user's deployment. Local setup must preserve existing test progress;
hosted mutations and server administration are outside the agent's scope.

Further broad assessment, unverified callback concerns and polish are deferred.
Only reproduced blockers to the agreed flows, authorization or saved data belong
in this step. Existing product invariants and affected validation/review
requirements still apply to repairs. After handover, work follows concrete testing
feedback rather than another general investigation.

This iteration changes documentation only. No application behavior changed, no
services were started, and no new runtime, browser or deployment checks were run.
The prior [Stableford repair evidence](validation/stableford-settings-lifetime-2026-09-23/README.md)
remains available; neighboring callback concerns are not claimed repaired.
Application test readiness must be established by the practical checks, not
inferred from this plan update or equated with hosted deployment sign-off or an
exhaustive security assessment.
