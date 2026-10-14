# Four-ball card reads recheck session expiry after loading

A1h closes the P2 late-expiry gap in the four-ball member/scoring card GETs. Both
original endpoints returned 200 after their active session expired while waiting
on the final confirmation query. The same regression now returns private 401
from both, with unchanged input/audit/receipt/confirmation state and no SSE.

Scoring already rechecked expiry during both-partner authorization, so its
membership-wait baseline was already denied. It still lacked a final check after
subsequent card loading. The repair adds that central database wall-clock check
after inputs, confirmation and visibility projection, immediately before commit.
It covers empty cards too, without claiming to freeze expiry during delivery.

Each complete repeatable-read attempt now permits one fresh retry after `40001`.
Stored context, role, partner ordering, both-partner scoring authority and facts
are rebuilt. A second conflict maps to generic private 503 through a GET-only
mapper; other errors keep their old mapping. Conditional input delivery,
confirmation, their shared helpers and mutation mappings remain unchanged.
The original context/auth/membership/format/partner/editability order and existing
all-status non-storeable headers are preserved.

For example, when an administrator is demoted to viewer while final visibility
changes from released to hidden, a member-card retry reloads both context and
role and returns nine visible holes with protected metadata null. The scoring
retry rejects that viewer. Neither can reuse the discarded authorization.

Seventeen new PostgreSQL/API cases cover expiry at real waits, authority writers,
fresh context and partner ordering, one-authorized-partner rejection, coherent
input/confirmation snapshots, signed frozen handicaps, ties/pickups/unentered
fields, cancellation, concurrency, exact retry counts and no writes/SSE.
Corrected focused suites pass 34 tests. An initial draft-card expectation was
corrected to assert the existing 409 before frozen partner snapshots exist.
Independent read-only review found no actionable source/test findings.

Formatting, 261 ordinary backend tests, strict Clippy and binary build pass.
The full PostgreSQL ladder passes 848 tests, with four existing performance probes
ignored. Explicit migration and seed pass at schema 39. The
[validation report](validation/four-ball-read-session-2026-10-14/README.md) records
commands, initial fixture correction and final evidence. Owned local validation
services were stopped and removed.
Chrome passes six member/scoring flows at 320, 390 and 1280px, including loading,
empty, populated, denied and error/retry states. Real typed clients and private
200/401 replies pass; long names fit, with no score writes, page exceptions or
unexpected network failures. Local health/readiness pass at schema 39.

No formula, payload, schema, dependency or frontend source changes. Four-ball
remains 18 holes; a hidden front nine is a projection, not nine-hole support.
Each successful read adds one final authority query; no speedup is claimed.
Eight P2 and three P3 inventory routes remain, alongside other queued findings
and existing device/hosting acceptance limits. [PLANS.md](PLANS.md) owns the next
bounded candidate; no queued repair is part of A1h.
