# Latest explanation

## Curated local course catalog

Tournament administrators now search a bundled eight-course shortlist instead
of spending GolfCourseAPI requests on free-text search. The catalog contains
Hacienda del Álamo, Saurines de la Torre, Mar Menor, Oppegård, Drøbak,
Miklagard, Oslo, and Haga. Its optional query matches display names and internal
common or accentless aliases case-insensitively; an omitted or blank query lists
all eight in deterministic file order.

Every catalog result reports a nullable verified provider course ID and an
explicit readiness state. Live bounded verification found Oslo (`dcm3cn0g`) and
Haga (`kcmzs8qz`), but their holes omit stroke indexes. Miklagard (`0zm1pe1a`)
has no provider tees. The remaining five returned no verified provider match.
Those facts are recorded as `incomplete` or `missing`; no ID or scorecard fact is
guessed, and none of the eight is currently presented as import-ready.

The old runtime provider-search route and search cache are gone. Catalog reads
work without an API key and perform zero external I/O. Provider detail remains a
separate future-ready boundary: only an entry marked `usable` may reach it.
Known incomplete IDs return a stable conflict and unknown IDs return not found
before touching quota or network.

Live verification also exposed that provider detail uses a
`{ "course": {...} }` envelope. The client now decodes that real shape while
continuing to reject empty tees, incomplete holes, invalid or duplicate stroke
indexes, and other unusable scorecard facts.

## Compact example

Catalog readiness is checked before the provider client is called:

```rust
match provider_course_readiness(&provider_course_id)? {
    ProviderCourseReadiness::Usable => {}
    ProviderCourseReadiness::Incomplete => return Err(CatalogIncomplete),
    ProviderCourseReadiness::Unknown => return Err(CatalogUnknown),
}
```

## Invariants

- The real API key remains only in ignored backend environment state and is
  never serialized or logged.
- Local catalog search consumes no provider quota and aliases are not exposed.
- Bundled JSON rejects unknown fields, duplicate names/aliases/IDs, invalid IDs,
  and inconsistent readiness states.
- Incomplete provider data cannot cross the detail or later import boundary.
- Provider course IDs remain opaque and no upstream tee ID is invented.
- No migration, local course revision, round mutation, SSE event, frontend
  behavior, or tournament/scoring invariant changed in this step.

## Validation

- Six catalog unit tests cover all eight entries, stable order, aliases including
  the supplied `Hacienda del Alamos` spelling, diacritics/case, blank/list-all,
  query bounds, duplicate aliases, readiness consistency, and unknown JSON
  fields.
- Nine provider-detail tests cover the live envelope, sensitive Bearer header,
  caching, quota, timeout, saturation, response bounds, status mapping, opaque
  IDs, and rejection of missing stroke indexes or empty tees.
- The PostgreSQL API test proves signed-out and cross-tournament rejection,
  private/non-cacheable catalog reads, local filtering without a key, removal of
  provider search, and zero provider calls for incomplete or unknown details.
- The complete backend suite passes 56 tests without database features and 140
  with all features: 56 unit tests plus 84 PostgreSQL integration tests. Rust
  formatting, all-target/all-feature Clippy with warnings denied, and
  `git diff --check` pass.
- Independent review found and verified fixes for strict bundled-JSON fields,
  stale response documentation, exact all-eight metadata coverage, and the
  complete `1..=hole_count` stroke-index permutation; no findings remain.
- Frontend and browser checks are not applicable because this step adds no UI or
  frontend API consumer.
