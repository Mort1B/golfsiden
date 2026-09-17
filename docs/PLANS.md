# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. The next bounded candidate below awaits an implementation instruction.

## Priorities

| Priority | ID | Finding | Repair order |
| --- | --- | --- | --- |
| Medium | M3 | A page return can be lost behind an unfinished earlier refresh | 1 |

## Medium — M3: refresh authority after overlapping page returns

**Evidence:** `frontend/src/api/liveInvalidation.ts:11-18` returns the pending
per-user promise for another `resume`, with no later refresh. An enabled score
button can still represent cached authority. The original frozen-page browser
case failed once in five unchanged repeats. The controlled
`returnOrdering.browser.ts` reproducer failed three of three runs: rounds,
completion and score-access responses completed as open; a scoring read stayed
pending; the page froze, the fixture locked the round, and persisted `pageshow`
shared the old refresh. Releasing the captured scoring response caused no fresh
authority requests, leaving editable controls instead of the read-only card.
No unauthorized server write or lost durable draft was demonstrated.

**Goal and scope:** ensure a distinct return arriving during an earlier refresh
results in a fresh session/authority pass after that pending work. Keep this
inside the shared return-invalidation boundary and focused lifecycle tests.
Preserve bounded coalescing of concurrent signals and a healthy EventSource;
avoid an unbounded refresh loop or new polling.
**Invariants:** revalidate identity before private reads; never revive a previous
account's data; preserve private projection clearing, device drafts, pending
verification, ordinary score-event invalidation and server mutation authority.
Retain local entry during pending recovery, with delivery/confirmation restrictions
unchanged. Once return verification settles, the editable view must reflect fresh
authority rather than responses captured before the second return.
**Validation:** turn the opt-in reproducer green without adding a pre-freeze wait
or weakening its read-only/no-edit/hole-selection assertions. Cover overlapping
return ordering, concurrent deduplication, expiry/account switch and failed reads
in unit tests. Repeat the original frozen-page case and run the complete return
browser suite plus the frontend ladder, with mobile/desktop evidence.
**Stop:** this return-refresh defect only; no queue, scoring, provider, backend,
performance or wider security redesign.

Reproducer command (currently expected to fail):

```bash
cd frontend
GOLF_RETURN_ORDERING_REPRO=1 npm run test:browser:lifecycle -- returnOrdering.browser.ts
```

## Later queue

1. **Performance work:** measure representative workloads before scoping changes,
   including the existing frontend bundle warning and bounded match-card reads.
2. **Security review:** perform the separately scoped wider application/operational
   review after the above correctness repairs.

No automatic opponents/byes/brackets, team match play, extra holes, new scoring
rules, public match sharing, cold offline launch or background sync is included.
