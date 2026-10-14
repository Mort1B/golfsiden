# Stableford card reads recheck session expiry after loading

A1i closes the P2 late-expiry gap in the Stableford member/scoring card GETs. Both
original endpoints returned 200 after their session expired while waiting for
the final confirmation query. The same regression now returns private 401 from
both, with unchanged inputs, audits, receipts, confirmations and no read SSE.

A final central database wall-clock session check now follows complete card
assembly, including inputs, confirmation and domain point/visibility projection,
immediately before commit. Empty cards follow the same path. Retained locks still
serialize later authority writers; the check cannot freeze expiry through commit
or network delivery.

One fresh complete repeatable-read attempt is allowed after `40001`. Stored
context, principal, exact membership, player snapshot, scoring permissions and
facts are reloaded. A second conflict maps to generic private 503 through a GET-only
mapper; other mappings and existing all-status non-storeable headers remain.
Settings, conditional delivery, confirmation and shared mutation helpers are
unchanged. Context/auth/membership/format/player/editability order is preserved,
including 404 before editability for drafts without frozen player snapshots.

For example, a stored 20-stroke hole still reports 20 actual strokes and zero
native points. A complete test card reports 34 points, +2 comparison equivalent
and 88 actual strokes as separate fields. Pickup remains zero points without
strokes; unentered holes remain unresolved. None of those calculations changed.

Seventeen new PostgreSQL/API cases cover real expiry waits, authority races,
fresh context/name on retry, player/flight permissions, frozen signed handicaps,
point/stroke distinctions, hidden-input invariance, coherent confirmation facts,
cancellation, concurrent full JSON parity, bounded retries and no writes/SSE.
Final focused suites pass 36 tests. One initial fixture used an invalid format
enum; it was corrected without runtime changes. Independent read-only review
found no actionable source/test findings.

Formatting, 261 ordinary backend tests, strict Clippy and binary build pass.
The full PostgreSQL ladder passes 865 tests, with four existing performance probes
ignored. Explicit migration and seed pass at schema 39. The
[validation report](validation/stableford-read-session-2026-10-14/README.md) records
commands, the fixture correction and final evidence. Owned local validation
services were stopped and removed.
Chrome passes six member/scoring flows at 320, 390 and 1280px, including real typed
values/private 200 and 401 replies, loading, populated, empty, denied and error/retry
states. Long names fit; no score writes, page exceptions or unexpected network
failures occur. Local health/readiness pass at schema 39.

No formulas, success payloads, schema, dependencies or frontend source changed.
Stableford remains 18 holes, with a hidden front-nine projection. Each success adds
one final authority query; no speedup is claimed. Six P2 and three P3 inventory
routes remain alongside the other queued findings and existing device/hosting
limits. [PLANS.md](PLANS.md) owns the next bounded candidate; no queued repair is
part of A1i.
