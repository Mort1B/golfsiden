# Match-list startup investigation

**Completed — READY WITH KNOWN LIMITATIONS.** The investigation identifies one
bounded next repair: forward existing query cancellation into match-list/table
HTTP requests. No production source, dependency, runtime refresh policy, API,
schema, scoring or authorization behavior changed.

The [investigation report](performance/startup/README.md) retains the reproducible
HTTP/SSE harness, all 48 samples, browser/server request timelines, DOM epoch
observations, resource bytes and production asset hashes. The application remains
at `d047830` throughout measurement; browser hashes match the published build.

Protected result queries already reject superseded successes and suppress late
denials from cancelled generations. Their list/table adapters do not pass the
query signal to `fetch`, so HTTP reads continue even after their results lose the
right to publish. The observed startup sequence is initial read, stream-open
refresh, then another read when a cleared table remounts its list children.

Across 48 cold/warm navigations at 320, 390 and 1280px, all **468 list reads and
120 table reads completed**. None of those 588 fetch calls received a cancellation
signal. Controlled overlap/reconnect cases included **84 held old responses**
that completed after their query generations were superseded. Their old content
did not reappear. Natural startup produced one or three reads per round; delayed
opening and held initial reads consistently produced three.

For example, a three-round page had settled content before stream opening at
2,356 ms. Three replacement list fetches began at 2,363–2,367 ms, the cleared table
hid its children at 2,384 ms, and three more list fetches began at 2,487–2,494 ms as
the table returned. All nine lists completed. The trace records DOM/request order;
the source establishes the parent-unmount cancellation interpretation.

The next step can attach the existing signal to the protected list/table reads
without removing initial/reconnect authority verification. Its acceptance is
observable aborts of superseded delayed requests, preserved fresh content and
unchanged denial/account/return-order behavior. It need not reduce request starts.
Management's ordinary list consumer keeps its existing lifecycle, with shared-key
navigation included in validation. Redesigning refetch ownership, subscriber
fan-out, history payloads or backend authorization stays outside that step.

## Validation and limits

- Production build and fresh bundle attribution passed; emitted/browser hashes
  agree with the unchanged published application.
- **28 tests across five files passed** for private-result cancellation/denial,
  live invalidation, queued return refresh, subscriptions and match/scoring keys.
- **11 existing production browser return-loading/ordering tests passed**,
  including unfinished returns followed by changed authority at all three widths.
- **48 investigation navigations passed** correct fresh content, transient epoch,
  zero-pending-request, console/network error and overflow checks. Representative
  mobile/desktop screenshots were inspected.
- Read-only source/harness review added transient-content assertions and actual
  browser resource timing. Final independent review recomputed the retained
  counts, byte ranges, transient epoch assertions and timelines, checked all 55
  asset hashes and found no blocking findings. Script syntax and diff checks passed.
- Full frontend unit/lint ladders were not repeated because only documentation
  and observational scripts changed; the build includes TypeScript compilation.
  Backend/PostgreSQL checks were not applicable and were not run.

Synthetic epochs validate frontend ordering, not backend membership, hidden-final
or revocation enforcement. Instrumentation affects scheduling; these traces are
not field latency measurements. The fixture shortens native SSE retry to 200 ms,
uses optimistic immutable asset caching and compressible data, and observes a
finite settling window. Browser completion/body bytes are distinguished from
server socket handoff. No database speedup or cancellation of already-started SQL
is established. The earlier PostgreSQL blocker was not retried in this step.

The investigation is closed and the transport repair is planned but unstarted.
