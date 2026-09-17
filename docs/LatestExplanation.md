# Route-level JavaScript splitting

**Completed — READY WITH KNOWN LIMITATIONS.** Route-only page modules now load
when opened. Home and sign-in stay eager; private pages retain their session gate.
Shared authentication, query, scoring-guard and offline providers keep their
existing lifetime. No backend, API, schema, dependency or scoring rules changed.

The [comparison report](performance/route-splitting/README.md) retains build
hashes, all 62 production-browser timing samples and measurement conditions.
Initial gzip JavaScript falls from 222,606 to 138,922 bytes for login (**37.6%**)
and to 143,807 bytes across all five chunks used by measured match-result pages
(**35.4%**). The entry is 452,821 uncompressed bytes; the former Vite size warning
is gone. Further routes download their own modules as needed.

Under the unchanged synthetic 100 ms/1.6 Mbps/4× CPU profile, login's median cold
ready time falls from 1,731 to 1,236 ms. The 72-card, three-round mobile workload
falls from 2,500 to 2,160 ms. Every measured cold median improves by 296–495 ms;
warm median increases are at most 32 ms (3.2%). Warm stress timing also improves,
but stream-open scheduling changes request counts, so it does not establish a
repair to API read amplification. API/query/SSE policies are unchanged.

## Loading and recovery

The small explicit module loader caches code only, forwards current props and
ignores resolution after unmount. This keeps account state out of the module
cache and avoids adding a Suspense fallback delay to warm full navigations.
The application shell and its navigation stay available during child loading.

Failed JavaScript or route stylesheet imports show an accessible error with
“Last siden på nytt”. Recovery requires a deliberate action, retains the URL's
query/fragment and honors the existing scoring guard. Rendering failures retain
existing route error handling; they are not classified as chunk download failures.

For example, a golfer can queue a durable score, navigate to Profile, encounter a
failed module download, reload deliberately and return to the same local score.
A score that could not be stored on the device, or an unsaved match note, continues
to block navigation/logout until resolved or discarded. Late module resolution
after logout cannot restore the former account's page data.

## Validation and limits

- Full frontend ladder passed: **626 tests / 109 files**, typecheck, lint and build.
  Browser-specific TypeScript and lint checks also passed.
- **27 production Chrome tests passed**, including existing return-loading and
  ordering regressions plus new route/direct-link/history, account replacement,
  durable/nondurable scoring, API error, and JavaScript/CSS recovery checks.
- Mobile 320/390px and desktop 1280px checks include delayed, error, empty,
  populated and long-content states. Screenshots were inspected; keyboard reload,
  44px reload height, overflow and unexpected console/network failures are checked.
- All **62 performance navigations passed** final content and zero-pending-request
  assertions. The fresh attribution build matched emitted assets byte-for-byte;
  browser hashes match the retained bundle. Source status records that measurement
  used the implementation worktree on parent `ac5f9d7`, not the unmodified parent.
- Independent read-only review found no blocking findings. Script syntax checks
  and `git diff --check` passed. No backend/database ladder ran because those
  layers and contracts did not change.

Timings use highly compressible synthetic API data and optimistic immutable
static caching, not production Caddy delivery or real API/PostgreSQL latency.
Physical phones, production deployment, sustained live load and backend privacy
are outside these browser fixtures. The earlier PostgreSQL measurement blocker
remains recorded in the baseline. These limits prevent a production performance
claim but do not block the measured frontend change.

The completed step is closed. The plan proposes one investigation of remaining
match-list startup amplification; no further repair or security review has begun.
