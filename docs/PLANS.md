# Plans

`PLANS.md` contains only current and queued work. Completed behavior belongs in
`Documentation.md`; durable technical decisions belong in `ARCHITECTURE.md`.

## Active step

None. The next candidate requires a separate bounded implementation decision.

## Later, as separate bounded steps

1. **Public result-sharing links:** Decide the exact public projection, displayed
   player identity, final-nine visibility, expiry and revocation before exposing
   any existing private reads. Recommended starting scope is a dedicated read-only
   leaderboard link created by an exact tournament admin; no account directory,
   score-entry authority or automatic public scorecard access.
2. **Offline scoring:** Define durable queued-write ownership, explicit unsynced
   status, retries/idempotency and score-conflict resolution. Reauthorize every
   replay and reject locked rounds or revoked access; clear or isolate private
   data on account changes. Current return-to-app refresh is not an offline queue.
3. **One open round per tournament:** Decide the product rule before adding a
   PostgreSQL constraint. Current reads deterministically select the highest-
   numbered open round. Any enforcement needs an existing-data preflight and
   concurrency validation without silently closing historical rounds.
4. **Additional formats:** After roadmap completion, performance work and security
   review, define four-ball, Stableford and match play as separate contracts.
