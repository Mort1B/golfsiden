# Route-level JavaScript splitting — 2026-09-17

**READY WITH KNOWN LIMITATIONS.** Deferring route-only modules reduces measured
initial gzip JavaScript by 37.6% for login and 35.4% for all measured match-result
workloads. Cold readiness improves in every workload. Warm median increases are
at most 32 ms (3.2%); the largest individual long-workload warm sample is 1,103 ms
versus 1,038 ms before. These small warm costs do not materially offset the
296–495 ms cold median improvements in this run. This is a bounded synthetic
comparison, not a claim about production latency or all routes.

## Provenance and reproduction

The unchanged [baseline](../README.md) used application source
`7fe6bcb1ffd6751d6621aba19293a801a885e81f`. Its documentation-only successor
`ac5f9d7110bf7f4a3ca273961e22ea8196179436` is the parent of this implementation.
The candidate was measured before commit: `sourceCommit` in the retained summary
is that parent, and `sourceStatus` explicitly records the modified frontend.
Asset SHA-256 hashes identify the actual candidate, not the unmodified parent.
No production source changed between the validated build, attribution and timing.
A fresh attribution build matched `dist` byte-for-byte, and all browser asset
hashes match [bundle.json](bundle.json).

From the repository root:

```bash
npm --prefix frontend run build
node docs/performance/bundle.mjs /tmp/route-split-bundle.json
node docs/performance/browser.mjs /tmp/route-split-performance
node docs/performance/summarize.mjs /tmp/route-split-performance/browser.json /tmp/route-split-performance
npm --prefix frontend run test:browser:routes
```

Use new output directories when preserving another run. The committed
[samples.csv](samples.csv) contains all 62 navigations; [summary.json](summary.json)
contains conditions, asset hashes, errors and distributions. Full per-request
artifacts/screenshots for this run are in `/tmp/route-split-performance/`.
The measurement scripts add source-status provenance and JavaScript body/request
summaries only; fixtures, throttling, readiness and observation protocol match
the baseline. No heavy validation jobs ran alongside measurements.

## Initial JavaScript across every loaded chunk

| Workload | Before gzip body | After gzip body | Reduction | JS resources before → after | Cold transfer before → after |
| --- | ---: | ---: | ---: | ---: | ---: |
| Login | 222,606 B | 138,922 B | 37.6% | 1 → 1 | 222,906 → 139,222 B |
| All measured match-result workloads | 222,606 B | 143,807 B | 35.4% | 1 → 5 | 222,906 → 145,307 B |

The match-result total includes the entry (138,922 B), `MatchResultsPage`
(1,167 B), `usePrivateResultQuery` (981 B), `useTournamentLive` (1,372 B), and
`MatchRound` (1,365 B). It is not just the smaller entry bundle. Resource timing
adds a 300-byte header estimate per cold resource. Warm transfers are zero under
the fixture's immutable static caching; warm `jsEncodedBytes` still describes the
cached body size and is not network traffic. JS request counts include cache hits.

The entry is now 452,821 uncompressed bytes, below Vite's 500 kB warning threshold.
Home and sign-in are eager; other pages are deferred. Vite also separates existing
route styles without CSS edits. This is a startup reduction, not removal of all
application code: visiting further routes downloads their additional modules.

## Ready time

Milliseconds, **median (min–max)**; five cold/warm pairs per 390px workload and
three each at 320px and 1280px. All widths have the same 4× CPU, 100 ms network
latency and 200,000 B/s download profile on the same laptop/Chrome version.

| Workload | Width | Before cold | After cold | Before warm | After warm |
| --- | ---: | ---: | ---: | ---: | ---: |
| login | 390 | 1,731 (1,716–1,738) | 1,236 (1,221–1,239) | 255 (238–256) | 239 (236–256) |
| populated | 390 | 2,370 (2,281–2,413) | 2,015 (1,957–2,016) | 855 (837–871) | 870 (868–870) |
| long | 390 | 2,500 (2,419–2,524) | 2,160 (2,043–2,185) | 1,021 (1,004–1,038) | 1,037 (1,003–1,103) |
| history | 390 | 2,412 (2,314–2,472) | 2,116 (2,012–2,122) | 968 (954–972) | 970 (969–984) |
| stress | 390 | 2,896 (2,866–3,298) | 2,559 (2,533–2,568) | 1,820 (1,339–1,869) | 1,411 (1,386–1,455) |
| long | 320 | 2,505 (2,450–2,527) | 2,148 (2,112–2,180) | 1,020 (1,005–1,037) | 1,035 (1,002–1,054) |
| long | 1280 | 2,426 (2,402–2,461) | 2,125 (2,103–2,196) | 1,005 (984–1,006) | 1,037 (1,004–1,055) |

All samples finish with the expected table/card counts, zero outstanding non-SSE
requests and no console/page/HTTP/fixture/overflow errors. Match-list reads remain
three per round in many cases (up to nine per navigation). Some cold and stress
samples have fewer requests because changed startup timing shifts the existing
stream-opening refresh relative to initial reads. SSE policy, authorization,
query staleness and API contracts are unchanged. In particular, the stress warm
improvement is not evidence that list amplification or database work was repaired.

The baseline's limitations still apply: gzip synthetic payloads compress unusually
well; immutable static caching is optimistic compared with the current Caddy
configuration; no real API/PostgreSQL time, production delivery, physical phones
or sustained live-event load is measured. Warm means a full navigation with fresh
JS/query state and cached static assets. Ready time is expected content plus two
animation frames, with a further 700 ms observation; it is not a field Web Vital.
The database measurement blocker remains as recorded in the baseline, and no
backend or database validation was needed for this frontend-only change.

## Behavior acceptance and review

- Full frontend ladder passed: 626 tests across 109 files, strict typecheck, lint
  and production build. Browser-specific TypeScript and ESLint checks passed.
- All 27 production Chrome tests passed: 16 new route checks plus 11 existing
  browser-return loading/ordering checks. New flows cover direct/in-app account,
  management, score, match and public-result routes; delayed loading; empty,
  populated, long and API-error states; browser history; JS/CSS failure recovery;
  account replacement; durable drafts; nondurable stroke edits and dirty notes.
- Repeated flow checks use 320, 390 and 1280px widths, including a 320×600 loading
  and navigation viewport. Screenshots were inspected for loading, chunk errors,
  management empty state, match scoring and shared results. The reload action
  accepts keyboard activation and has a measured minimum 44px height.
- The shared providers and shell remain mounted through child-module loading.
  Private import starts behind session verification. Only code is cached; late
  resolution cannot restore the old account. Import errors offer explicit,
  guard-aware reload. Rendering failures do not expose that reload action.
- Independent read-only source review found no blocking findings or additional
  blocking test gaps. Browser APIs use synthetic fixtures; these checks do not
  establish backend authorization or production deployment behavior.

No backend, schema, scoring, authorization, dependency, global-style or live-policy
changes are included. Further performance repairs and the wider security review
remain separate steps.
