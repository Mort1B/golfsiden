# Per-hole handicap strokes in Oppsummering

Score → Oppsummering now shows a compact handicap badge beside each hole's
Par/Index. Positive allocations display `+1`, `+2`, or more; negative allocations
use `−1`, `−2`, and wording explaining strokes given back. Zero allocations have
no badge. The indicators appear before scoring and remain separate from the
recorded gross/net score. Accessible labels include the hole and allocation.
The shared summary covers player and team cards, including read-only history.

The backend exposes required signed `handicap_strokes` on every scoring/read
hole, including confirmation responses. Domain assembly calls the existing
stroke-index allocator with the same preserved owner playing handicap used for
net scoring and the full round length. Net is gross minus this exact allocation.
Read projection copies the field only for visible holes; it never redistributes
an 18-hole handicap across a nine-hole visible prefix. Individual and foursomes
snapshots and scramble's preserved member-handicap calculation remain unchanged.

For example, playing handicap 20 over 18 holes displays `+2` at stroke indexes
1–2 and `+1` elsewhere. A gross 5 on index 1 remains net 3. With playing handicap
−2, the two strokes given back appear at indexes 17–18. Changing today's profile
handicap does not change historical allocations.

No migration or scoring-policy change is required. Deploy this backend before
or alongside the frontend: the new runtime decoder rejects older responses that
omit the required field. Existing frontend versions ignore the additional field.

## Validation

- Backend: formatting passed; the ordinary workspace/all-targets test run passed
  117 tests; Clippy with all targets/features and warnings denied passed.
- PostgreSQL 17: the workspace/all-targets database-feature run passed 359 tests
  (including the 117 unit tests). Forward migrations and seed both succeeded on
  a disposable local Podman database. API regressions cover individual,
  scramble, foursomes, profile-handicap changes, confirmation responses, and
  restricted final-nine projections. Domain cases cover 9/18 holes, reversed
  indexes, zero/negative/multiple allocations, and scored/unscored owners.
- Frontend: all 400 tests in 67 files passed. Strict application and browser-suite
  TypeScript compilation, ESLint, and production build passed. The existing
  greater-than-500-kB bundle advisory remains (614.41 kB main chunk).
- All 10 affected Chrome browser scenarios passed: the 2 new summary scenarios
  and all 8 existing return-to-app scenarios. Run them from `frontend/` with
  `npx playwright test --config playwright.lifecycle.config.ts
  handicapSummary.browser.ts returnLoading.browser.ts`. Checks cover 320, 390,
  and 1280px widths, overflow, accessible labels, 44px hole controls, reachability,
  long names, scored/unscored and zero/negative allocations, team switching, and
  restricted views. Screenshots were inspected on mobile and desktop. Existing
  recovery scenarios cover delayed/error/empty/offline and locked states.
  Browser console/network assertions passed. Initial new-fixture failures were
  resolved by supplying valid completion/round contracts and an exact heading
  locator; no production-code repair was needed.
- Read-only scoring/contract review found no production-code issues and noted
  the backend-first deployment requirement recorded above.

Browser scenarios use runtime-decoded API fixtures and a real local event stream;
PostgreSQL tests separately exercise real API handlers and persisted snapshots.
Production Caddy and physical iPhone/Safari testing are outside this local harness.
Administrator-assisted password recovery remains queued as a separate step.

**Verdict: READY WITH KNOWN LIMITATIONS.** Local validation and review are complete;
production/browser environment limits and the existing bundle advisory are recorded above.
