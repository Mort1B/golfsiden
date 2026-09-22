# Match lists stay subscribed while their table loads

**Completed — READY WITH KNOWN LIMITATIONS.**

The shared match-results view now keeps current-round list observers mounted
during table-only loading. Readiness disables new list fetching and removes all
private list content, links and round headings from the DOM. An already-started
refresh may finish while hidden. When the table returns while that list is still
fresh, the same observer can display it without cancelling and replacing a read.

Initial lists still wait for their table. Parent errors, missing rounds and
account/tournament/round removal still remove old owners. Other `MatchRound`
callers remain enabled by default. There is no copied server state, historical
admission flag, CSS-hidden private markup, changed freshness policy or altered
live subscription. Authority checks, synchronous projection erasure, stale-denial
guards, transport cancellation and shared management reads remain in place.

The [comparison report](performance/gated-observers/README.md) retains reproduction,
request/server/DOM evidence and build provenance. Savings are conditional: a list
that finishes while its table remains held for 21 seconds is stale when the gate
reopens, so it is fetched again under the unchanged 20-second freshness rule.
The real-browser deadline check preserves that behavior. Two reads can complete
in this case; no universal request reduction or production speedup is claimed.

A restricted-final browser transition uses nine halved holes followed by a full
match concession after hole nine. Its restricted response retains those nine hole
reports, hides the concession and completion metadata, removes scoring permission
and excludes match points. The decoder accepts the response while the table is
held, but private DOM appears only after the gate reopens. Phone and desktop
checks cover loading, populated, long names, restricted results and empty lists.

Full-card history analysis, disposable PostgreSQL authorization measurements and
the wider security review remain queued. Backend, database, API contracts,
scoring rules and writable intent are unchanged.

## Validation and measured result

- Full frontend ladder passed: **708 tests in 113 files**, TypeScript, ESLint and
  production build; browser-specific TypeScript passed too.
- **42 production-browser route tests passed**, plus the final nine gated-result
  cases rerun with the real deadline, ledger-consistent projection and explicit
  completed-request counts. Mobile bottom-navigation clearance and screenshots
  were checked at 320/390px and desktop rendering at 1280px.
- **18 native HTTP/SSE cases passed** using the unchanged investigation harness.
  Each visibility/table-release cycle now starts three lists, all completed,
  instead of six starts with three aborts. Stream open likewise drops from six
  native starts to three. Reconnect and same-account return retain three reads.
- All 225 obsolete held responses cancelled before release, 36 current held
  tables completed and 18 expected denials failed closed. No stale account repaint,
  unexpected errors, overflow or pending non-SSE requests remained. All 55 emitted
  asset hashes match the fresh build attribution.
- Independent read-only review found no production blocker. It strengthened cold
  disabled-observer assertions and final browser evidence. All nine ordering
  tests fail against the prior production implementation and pass with this fix.

Backend/PostgreSQL checks are outside this frontend-only step. Synthetic fixtures,
finite DOM observation and desktop viewport emulation do not establish production
speedup, database behavior, physical-phone or actual BFCache coverage. The measured
benefit applies while the returned list remains fresh; prolonged table holds can
still cause two completed reads. The next candidate is investigation only of
full-card history payloads; no payload/database/security work started here.
