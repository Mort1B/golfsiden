# Application logic, performance and security review

The documentation-only review found eight actionable items: seven P2/medium and
one P3/low. No P0/critical or P1/high application finding was confirmed. Sporting
rules, preserved handicap ownership, score corrections and the recent Fantasy
and award logic were consistent in the paths reviewed. The application still
needs the identified session and security work; passing tests alone does not
establish complete safety or performance.

Three session defects were reproduced with synthetic data. A private leaderboard
request waiting on membership returned the same populated data after its session
was revoked. A round-opening request waiting on membership committed after its
session expired. In Chrome, a delayed anonymous creation response replaced a
newer login after the creation page had unmounted, including the browser cookie.
The browser scenario reproduced at mobile and desktop widths.

The other findings concern a reachable Rustls advisory, optional restore checksum
verification, password-work admission waiting, development-tool advisories and
team-list N+1 queries. Evidence strength and application exposure determine each
rating. Development-tool scanner critical labels do not establish a production
remote-code-execution path, and performance observations have no demonstrated
production latency-budget breach.

For example, revoking a session correctly makes the next leaderboard request
return 401, but a request that already passed the extractor can currently return
200 after a database wait. The selected next candidate binds leaderboard session
authority to the data transaction. It preserves membership and embargo behavior;
other endpoint families and lifecycle expiry remain separately bounded work.

Fresh validation passed: 1,113 frontend tests, 258 backend tests without the
database feature, and 736 tests in the PostgreSQL-feature ladder. The database
total includes the ordinary backend tests. Formatting, Clippy, typecheck, lint,
builds and schema-39 migration/seed passed. Four manual performance probes were
ignored in the database ladder. No full browser regression, production load test,
public deployment or physical-device acceptance was performed.

The [review report](validation/application-review-2026-10-14/README.md) contains
source locations, reproduced outcomes, current dependency sources, acceptance
criteria and limitations. [PLANS.md](PLANS.md) selects one next candidate and
orders the remaining work. No application code, dependency or migration changed,
and no remediation was started.
