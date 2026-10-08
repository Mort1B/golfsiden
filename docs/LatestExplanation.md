# Play Fantasy inside the tournament

FANTASY-5 adds the private Fantasy screen, linked from the tournament and its
management workspace. Members can join, select four golfers and a captain, save
before the deadline, inspect carry-forward provenance and browse manager/golfer
round and overall standings. Administrators can enable the game, set earlier
selection deadlines and record or correct non-finish dispositions. Schema remains
37; this step changes the frontend only.

The server remains authoritative for scores, deadlines and selections. Receipts
separate accepted choices from unsaved input. An uncertain save preserves its
exact request for deliberate replay, and an older receipt cannot replace a newer
accepted lineup. A valid current lineup takes precedence over the carry preview.
If a selected golfer becomes ineligible, they remain visible so the manager can
remove and replace them. Captain multipliers, including negatives, affect only
manager totals; every golfer has an independent base-points row.

Typed decoders reject mismatched round inventories and hidden-result details.
Queries are scoped to user and tournament and consume cancellation signals.
Score/match/visibility signals and browser return refresh the Fantasy family;
visibility changes, denied access and session transitions clear private data.
Callbacks check mounted lifetime and canonical user/CSRF identity. Only the exact
retryable read response `409 fantasy_conflict` receives two bounded retries;
mutations never retry automatically or queue offline.

Review and browser checks repaired an ineligible-pick removal dead end, fabricated
settlement owners for nonparticipants, and a refresh path that reset the selected
owner and erased non-finish save feedback. The settlement controller now survives
ordinary refresh while the source-bound reason/review form resets on changed
facts. Source corrections require renewed acknowledgement and a reason.

Validation:

- Final frontend suite: 133 files, 977 tests passed, including 35 new focused
  Fantasy tests. Coverage includes held callbacks across logout/CSRF renewal/
  unmount, exact uncertain replay, stale revisions, source corrections, hidden
  decoding, query cancellation, denial clearing and bounded read retries.
- Final typecheck, lint and production build passed.
- Three persisted real-Chrome tests passed against the local schema-37 API and
  disposable PostgreSQL. Layout assertions/screenshots cover 320, 390 and 1280
  pixels: disabled/empty, saved, populated, loading, error, denied and hidden states.
- The real-data flow exercised enabling, enrollment, four picks/captain, deadline,
  opening, 18 recorded scores, confirmation, three non-finish settlements, stale
  source correction, both boards, manager/golfer breakdowns and carry preview.
  Recorded points plus placement produced 22; correcting a captained non-finisher
  to −1 changed the manager total to 20 while the first golfer stayed at 22.
- A separate member account could not see another manager's draft. Hidden final
  scores stayed concealed through edits, release/re-hide and tab return; logout
  removed the private screen. Controlled 503/403 responses exercised retry/denial.
- Rapid fixture score updates produced four observed 409 responses, all verified
  as `fantasy_conflict` on result reads. Bounded retries recovered; final UI had
  no error alerts and the result read returned 200. No unexpected page errors or
  failed requests remained in the recorded main browser flow.

Read-only source and documentation review has no outstanding findings. Local
Markdown links (91), whitespace and production-file size checks passed. Browser screenshots were
inspected for readability and overflow. Backend/database validation ladders were
not rerun because no backend, migration or persistence source changed; the previous
FANTASY-4 step validated those contracts with 712 database-enabled tests. The local
API health and schema readiness passed for this UI run.

Reproduce the browser suite with the local API on port 3000 and Vite on 5173:
`GOLF_FANTASY_BROWSER=1 npx playwright test --config playwright.lifecycle.config.ts fantasy.browser.ts fantasyPrivacy.browser.ts`
from `frontend/`, using disposable data.

FANTASY-6 remains the next separate step: broader multi-round release acceptance
with nine-team and smaller/larger fields and the full format/lifecycle matrix.
This iteration does not claim that matrix, every failing-response/session-change
permutation, physical-device testing or hosted gg26.no acceptance. No production
deployment or new restore rehearsal was performed.
