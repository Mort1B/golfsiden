# Fantasy refresh duplication removed

R2 now reuses a target response only when the current refresh observed a new
network fetch and successful completion, and the same active query still passes
session, state and freshness checks. Manual cache writes, older initial requests,
query erasure/replacement and failed reads cannot substitute for that evidence.
Otherwise the existing explicit target read runs. Per-write preflight, SSE and
full-family invalidation remain unchanged.

A three-round unit fixture now performs 12 reads instead of 15. The real-SSE
Chrome probes reduced round request starts from 24 to 21 for three saves and
from 60 to 55 for five saves, with the same three/five writes. Broader overview
refreshing remains intentionally unchanged; no latency speedup is claimed.

Validation passed: 1,113 frontend tests, typecheck, lint, strict browser TypeScript,
production build, four preset/recovery Chrome cases and two request-count probes.
Ten proof tests cover manual data, old requests, replacement, cancellation,
inactivity and overlapping refreshes. The [evidence report](validation/predeployment-fixes-2026-10-14/README.md)
also records R1's completed test repair and intermediate fixture corrections.

The remaining authorized step is R3: measure and specialize the backend's
round-only result projection while preserving its transaction and authority checks.
Physical iPhone and hosted acceptance remain separate.
