# Storage failures no longer continuously restart score delivery

PERSIST-2 is repaired. The score queue now starts another immediate delivery pass
only after a successful durable change and queue reload. A failed read or write,
or a claim that makes no progress, yields to the existing two-second timer or an
explicit retry/reconnect/page-return signal. Retained queue data alone cannot
restart the loop. This applies to ordinary, four-ball and Stableford scoring.

If acknowledgement storage fails, the original request and delivery lease remain
intact. Recovery replays that exact request after the lease expires; manual retry
cannot bypass the lease or silently change the original expected score. Healthy
multi-hole delivery, conflicts, session fencing and cross-tab updates are preserved.

For example, a device-saved `4` on hole 8 stays visible during a storage fault.
Chrome can still run browser tasks and show the retry control. After storage
recovers, retry sends the unchanged request and verification clears the queue.

The [validation report](validation/score-storage-retry-2026-09-23/README.md) records
three failing-first schedules, 14 new regressions, **782 passing tests**, typecheck,
lint, build and **16 passing Chrome scenarios**. At 320/390/1280px the bounded
browser outage now permits a zero-delay task after one failed storage access,
and recovery sends the retained request once. Independent review found no
remaining actionable defect. An existing match-retention test now waits for
replacement-queue loading before asserting that the input is enabled.

This repair is **READY** within its frontend scope. Chrome uses synthetic API
responses and real browser IndexedDB; actual-server delivery, physical disk
exhaustion, physical Android and native 200% zoom are outside this evidence.
No backend, migration, dependency or production configuration changes were needed.

Deployment remains **NOT READY** while PERSIST-3 and operational/public-host/device
acceptance remain open. PERSIST-3 is the next proposed bounded repair. Completed
validated work is committed to main and pushed to origin/main as requested.
