# Friends deployment validation

The functionality, design and Chrome assessment is complete for application
commit `38e3eef`. The full [report and evidence](validation/friends-2026-09-23/README.md)
record commands, topology, roles, viewports, confirmed findings and skipped gates.
No production application code, scoring rules or schemas changed.

The deployment sign-off verdict is **NOT READY**. The separate security assessment
and a current backup/restore exercise remain unresolved; physical Android and
native 200% Chrome zoom were not verified. The tested desktop Chrome journeys
provide broad functional evidence but cannot establish those deployment gates.

Three UI defects were reproduced:

- Keyboard focus on a profile tournament card can be partly covered by fixed
  navigation at 320×600. Manual scrolling reveals it.
- Management back navigation is 40×40px, below the repository's 44×44px requirement.
- Small tournament section counts measure 4.4705:1 contrast, below 4.5:1.

The next product candidate is one bounded shared navigation/focus accessibility
repair. It remains proposed; this assessment does not implement those fixes.
The pre-existing separate security-review plan edits are preserved.

## Functional and browser evidence

The assessment exercised account/profile/recovery, tournament creation and joining,
manual organizer team/flight setup, saved courses, score entry and persistence,
existing formats, offline/retry/conflicts, return freshness, private reads,
completion/archive and public result-sharing boundaries. The detailed report
separates real API assertions from mocked layout/error tests.

Chrome 153.0.8010.36 ran on Linux. Layout checks covered 320×600, 390×844, 699×900,
700×900 and 1280×900; screenshots and DOM geometry were inspected. Main navigation,
profile, saved courses and administration were reachable on mobile and desktop.
The actual UI journey issued an invitation, joined an existing account, saved a
supplied course, assigned two players to a team and flight, and opened the round.

Production assets were served through Caddy against disposable PostgreSQL with
separate owner/runtime roles. A separate HTTPS production-mode API smoke verified
secure session attributes, CSP/security headers, a native score event through the
same-origin proxy, persisted scores and an exact second-session gross total from
5 to 9. Its local internal certificate does not prove public TLS/DNS readiness.

All 751 frontend tests, typecheck, lint and production build passed. The browser
matrix includes 42 route/return cases, 26 resumed core cases and independent fresh
seed suites; final per-suite results and overlaps are recorded in the report.

Only browser-test maintenance was needed: lifecycle assertions now use the current
server-saved and flight-completion wording, with hidden-state checks strengthened.
A new opt-in HTTPS regression covers the previously missing proxy event boundary.
The initial local API outage, production onboarding rate limit during repeated
fixture development, and mock-stream/CSP incompatibilities are documented rather
than counted as passing checks. No production checks or policies were weakened.

No backend or migration code changed, so the full Rust/PostgreSQL ladders were not
repeated. Fresh migration/seed and real API mutation behavior were tested. This
work does not deploy the application or modify real tournament data.
