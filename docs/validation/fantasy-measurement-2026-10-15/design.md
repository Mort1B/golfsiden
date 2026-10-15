# A1t predeclared local measurement design

Declared before fixture setup and measurement, against `b6a81a3`.

- Use an exclusively owned disposable PostgreSQL 17 Alpine container and the
  unchanged debug API in that container's owned network namespace with only
  loopback-published ports, with the current Vite frontend in real headless Chrome.
  Loopback traffic only; no hosted environment, application instrumentation,
  runtime optimization, schema/index or dependency changes.
- Create two synthetic tournaments through existing APIs: one and five rounds,
  each with eight zero-handicap golfers, four two-player scramble teams per
  round, 18 par-four holes and two enrolled managers. No real user data.
  The measured browser is the administrator, default Min firer section,
  first round selected, overall board selection, no expanded details, full
  authorized visibility, 1280x900 viewport and unchanged active-query inventory.
  Other-manager, nonmember and anonymous controls use separate contexts.
- First measure saves with all rounds configured and draft: one warmup save,
  then five fully observed and five minimally observed individual saves, each a
  deliberate captain edit with a new request and expected current revision.
  Alternate observer modes. No batch actions or automatic write retries.
- Fully observed saves capture method/normalized route, response status,
  completion/cancellation, elapsed time and decoded error classifications;
  minimally observed saves retain receipt latency and database receipt counts.
  Both await the receipt and a fixed one-second reconciliation tail. Report
  that tail separately from receipt latency. Compare the two observer modes;
  small samples cannot establish negligible overhead.
  Record the one-second cutoff separately, then require no finite request in
  flight and 500ms quiet (10s bound) before the next sample. This barrier is
  present in both observer modes; full mode additionally retains request records.
  A quiet window cannot prove absence of indefinitely delayed SSE delivery.
- With the settled page and SSE connected, observe one three-second idle window
  per fixture. Count completed API reads, cancellations and open SSE separately.
  Initial navigation and warmup are outside the measured window.
- Then use ordinary sporting APIs to open, score all 18 holes for each team,
  confirm and complete each round. Both managers have valid first-round picks;
  later rounds exercise normal carry-forward. Read the first round's results.
  Expect eight settled golfers, two settled managers and 18 holes per golfer.
  Retain exact response equality assertions, but publish no bodies or identities.
- Record the first observed settled read separately. It is application-first,
  not cold disk/cache: fixture setup already touches PostgreSQL pages. Warm
  five reads before timing. Compare pg_stat_statements tracking `none`/`top` in
  ABBA blocks of ten sequential requests (20 per mode per fixture), resetting
  only this owned database's statistics between blocks. Include normalized SQL
  fingerprints, counts, rows, execution time and buffer counters for tracked
  blocks. No SQL parameters, sessions, payloads or credentials in evidence.
  Set tracking through ALTER SYSTEM plus reload, not only the probe session;
  use an unmeasured API read after reload. Require no tracked application
  statements in the none blocks and expected positive counts in top blocks.
  Exclude collector/reset/activity statements and reset after all setup and
  warmup queries. Only isolated blocks support per-successful-read SQL counts.
- HTTP timings cover the loopback API request and full body consumption in the
  Playwright API client. SQL execution aggregates are server statements only:
  they exclude client round trips, projection, JSON decoding and some commit
  cost, and are not added to HTTP time. Bulk-facts SQL is separately identified;
  it does not measure Rust hashing/deserialization independently.
- Perform three controlled lock-wait reads per fixture by holding the target
  round row for 200ms in a separate transaction. Require direct PostgreSQL
  observation of a blocked API lock before releasing it. Report these apart
  from uncontended samples; absent observed waiting is a failed probe, not zero
  lock cost. Do not infer production contention or remove lifecycle locks.
- Check anonymous and nonmember denial, private/no-store headers, other-manager
  pre-lock pick privacy, exactly one accepted write/receipt per save and exact
  settled-result parity across all reads. Capture browser errors and classify
  cancellations independently of completed responses.
  These checks do not replace existing revocation/final-expiry or hidden-final
  noninterference regressions. Record whether the first result read finalizes
  anything and require stabilized reads to retain the same response.
- Retain machine-readable sanitized evidence, executable diagnostic support,
  commands, limitations and an independent read-only review. No full production
  regression ladder is implied by documentation/probe-only changes; build the
  unchanged API and run diagnostic syntax/assertion/browser/PostgreSQL checks.
- Stop at a local reproducible report. At most one later bounded optimization
  candidate may be proposed if observed cost supports it; no runtime change or
  production capacity/security certification follows from this experiment.

## Implementation boundary clarification after the run

The saved `receipt_ms` endpoint is the rendered current-lineup receipt obtained
through the authoritative round GET, rather than immediate provider acknowledgment.
The probe then waits for the edit control to become enabled before starting its
one-second tail. This adds an independently untimed variable reconciliation wait.
Request counts cover the whole click-through-quiet-barrier window; the pending
cutoff is UI-ready plus one second. This refinement of the original receipt/tail
wording is recorded as a design deviation, not retroactively predeclared timing.
