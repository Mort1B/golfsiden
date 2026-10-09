# Keep testing the application

Use a separate test tournament on your deployment at gg26.no. Keep it between
sessions so you can test returning to saved work. Use one organizer account and
one player account in separate browsers or on separate devices. Record the
release Git SHA/image tag, browser and phone model with your results. Hosting and
server operations remain yours; see the [deployment guide](deployment_guide.md).

Start with the practical session below, then test the formats and optional flows
you actually intend to use. These are manual checks to perform, not a claim that
your hosted deployment has already passed them.

Fantasy is available from the tournament's **Fantasy · min firer og poengtavler**
link. Enable it before opening the first round; each manager must join and save
four golfers plus a captain. See the [playing/admin flow](Documentation.md#playing-and-administering-fantasy).

The [local release acceptance report](validation/fantasy-release-2026-10-08/README.md)
records the automated checks already run, including nine teams over three rounds.
Use the following checks to confirm your hosted release and devices.

## Playing-day consistency checks

- [ ] On stroke/team, four-ball and Stableford, compare context → hole → input →
  save state → previous/next → secondary controls at phone and desktop widths.
- [ ] Review an incomplete card from hole 18 and a complete card from another
  hole. The action must follow actual progress and never confirm automatically.
  Keyboard activation keeps focus visible; switching and view controls sit above
  the full summary. Pending changes continue to block confirmation.
- [ ] Offline, correct one entered hole, then enter a new hole: progress increases
  only for the new hole. Server-checked progress and totals remain distinct.
- [ ] Check long player/team names, 44px targets, wrapping and fixed-navigation
  overlap. Quick card switching opens the selected hole on the other card.
- [ ] Tournament overview: loading, retryable failure, no rounds, populated,
  draft/locked and unavailable access. Continue scoring appears only after fresh
  access and resumes within that tournament/round.
- [ ] Fantasy: save a lineup, reopen with **Endre valg**, change captain and switch
  **Min firer / Poengtavler**. Draft and uncertainty survive view/reconnect;
  warnings remain visible and exact reconciliation never changes request identity.

## Optional Fantasy session

- [ ] Enable while rounds are draft; join as two members, save different captains,
  and verify each sees only their own picks before lock, including the admin.
- [ ] Set an earlier deadline or open the round. Verify locked picks, the captain
  and automatic carry-forward preview/origin in the next round.
- [ ] Enter scores, check both **Fantasy-lag** and **Spillerpoeng**, and open round
  and overall explanations. Captain multipliers must affect only manager totals;
  both team partners must receive shared base points.
- [ ] Test a genuine non-finish in the separate test tournament. Confirm retained
  points, no remaining-hole penalty, and no placement. Change a source score and
  verify that the stale disposition requires a new reason and correction review.
- [ ] Check hidden-final results as the player, release/re-hide as admin, and
  return from another tab. No concealed result may remain visible.
- [ ] Verify failed/uncertain submissions, retry and expired-deadline feedback.
  Log out and use the other account; no previous member's private draft may remain.

## A practical first session

- [ ] **Create and invite.** As organizer, create a tournament with the intended
  rounds and courses. Issue an invitation and join as the separate player.
  Check that each account sees the correct tournament and role.
- [ ] **Edit draft details.** Open **Turneringsstyring → Innstillinger → Navn og
  datoer**. Change the name, description and date range, save and reload. The
  heading/list should show the new details, and existing rounds should stay
  unchanged. A range excluding a round date must be rejected.
- [ ] **Prepare pairings.** Assign flights and starting holes; assign two-player
  teams for team formats or opponents for singles. Save, reopen the editor and
  verify the assignments. Follow readiness messages for missing setup. Teams and
  opponents are assigned by you, not generated automatically.
- [ ] **Start and open.** Start the tournament, then open a ready round under
  **Rundestyring**. These are separate actions. Tournament details should now be
  read-only, and the round should show its preserved handicap information.
- [ ] **Enter and edit scores.** As player on the phone, open **Score**, enter a
  hole score, wait for **Lagret på serveren**, then change it and wait again.
  Confirm you are still on the intended round and player/team.
- [ ] **Check live results.** In the other session, open **Resultater**, check
  **Brutto** and **Netto**, and open the same scorecard. The values should match.
  Net values use preserved handicaps; hidden final-round results may deliberately
  be absent. Singles uses its separate match-points table.
- [ ] **Check navigation and return.** Switch tournaments/rounds in results,
  open a player's history, then use Back/Forward. Check the selected context.
  Return to **Score** online: it should fetch fresh data and choose the first
  missing hole, or the summary when complete.
- [ ] **Check persistence.** After saving to the server, reload and sign out/in.
  Scores should remain. After your next normal deployment or service restart,
  compare the same scores and results again without resetting or reseeding data.
- [ ] **Confirm and finish.** Complete the required cards and confirm online.
  As organizer, complete and lock the round when eligible. Ordinary score editing
  should then be unavailable. Once all required rounds are locked, complete and
  archive the tournament; members should still find its preserved results.

## Testing a coverage gap

Use an individual stroke, scramble/foursomes, four-ball or Stableford card. Keep
this test brief and keep the browser app open throughout.

1. Open the intended writable card while connected. Note the round, owner and
   current hole, then disable connectivity (ensure both Wi-Fi and mobile data
   are off if using airplane mode).
2. Enter two holes, waiting for the explicit saved-on-device state after each.
   Move to the next hole and note its number.
3. Navigate to the tournament list, then choose **Score → Tilbake til åpnet
   scorekort**. The same card and last visited hole should return, with wording
   that server verification is pending.
4. Enter another score and wait for device persistence. Confirmation and switching
   to other cards should remain unavailable while verification is pending.
5. Restore connectivity. Wait for pending entries to reach **Lagret på serveren**;
   the current hole should remain selected. Check all entered values from the
   second online session, then confirm when the card is complete.

If the return option is unavailable, reconnect and reopen the card. Where offered,
use **Hent scorekort på nytt**. Cached data can expire while away, and logout,
session replacement/expiry or observed access loss/locking removes eligibility.
The option disappearing must not be mistaken for pending entries being deleted.

The app does **not** support offline reload or reopening after closure or phone
browser eviction. Pending device edits can survive reload for later online
delivery, but the full card is not stored for offline reopening. Do not clear
site data while edits are pending. Singles match play supports offline numeric
notes only; reports, concessions, corrections and confirmation require a connection.

## Format-specific checks

Use these checks for the formats in your tournament. Four-ball, Stableford and
singles require 18-hole courses; stroke play, scramble and foursomes also support
nine-hole layouts.

| Format | What to check |
| --- | --- |
| Individual stroke play | Actual strokes and separate gross/net results match the card and its preserved handicap |
| Scramble or foursomes | The round has the intended two-player teams, one scorecard per team and contributions credited to the correct players |
| Four-ball | Each partner can enter scores or pickups; the team's best gross and net result is selected independently on each hole; confirmation acknowledges any blank partner fields |
| Stableford | Numeric scores and pickups yield the expected gross/net points; overall results label `36 − points` equivalents separately from actual strokes |
| Singles match play | Manually assigned opponents and gross/net mode are correct; report and confirm a result online; win/draw/loss contributes 1/½/0 match points and nothing to overall gross/net |

When teams change in a later round, confirm earlier results still use the original
team and handicap snapshots. Profile handicap changes must not rewrite past cards.

## Recovery and conflict checks

Use disposable test data for these additional cases:

- [ ] **Competing draft edit.** Open the same draft details in two sessions. Save
  a change in one, then try saving the older form in the other. The stale save
  should preserve your draft and offer explicit refresh/discard, without silently
  overwriting the first change. After tournament start, detail edits are blocked.
- [ ] **Score conflict.** Open the same authorized card in two sessions. In one,
  disconnect and save a device edit; in the other, change that same hole online.
  Reconnect the first. Review the local/server comparison and deliberately choose
  which value to keep; verify the final server value from both sessions.
- [ ] **Interrupted read/save.** Temporarily disconnect during a test read or
  save. Check the displayed state and reconnect/retry. A value is saved on the
  server only when the app reports that state. Preserve unresolved inputs rather
  than clearing storage or repeatedly submitting a new operation.
- [ ] **Account separation.** After a device edit is persisted, reconnect and
  switch to another account. It must not display or send the first account's
  pending edits. Return to the original account to check any remaining delivery;
  reopen the card online because prepared return does not survive logout.

## Optional access and sharing checks

- [ ] An unrelated account cannot open the private tournament or scorecards.
- [ ] For an 18-hole final, hide/release/re-hide the back nine as organizer. Check
  the corresponding member results after each change; scoring access is separate.
- [ ] If using public sharing, deliberately create a link for a test tournament,
  open it signed out and check only permitted overall standings are visible.
  Revoke it and check the next read is unavailable. Links do not grant private
  scorecard access, and singles points are not public overall standings.
- [ ] If using password recovery, use a designated ordinary test player. Check
  the organizer-issued private link, password reset and old-session invalidation.
  Administrator recovery uses the operator procedure in the deployment guide.

## Phone usability and bug reports

Check short phone screens and desktop: no horizontal overflow, reachable bottom
navigation and save/confirm buttons, readable long names, and score inputs that
remain usable when the keyboard is open. Include a slow connection and an empty
or incomplete round as well as populated scorecards.

For a bug report, include:

- deployed release, browser/phone, page, round and format;
- steps, expected result and actual result;
- whether connectivity was available and whether the app said saved on device,
  saved on server, pending verification or conflict;
- a screenshot or exact error text, with private links and personal details removed.

Never include passwords, session tokens or invitation/recovery secrets. Prioritize
blocked workflows, missing saved scores, incorrect results or incorrect access;
smaller improvements can follow normal testing feedback.

## Recorded evidence and limits

The [functional-readiness report](validation/test-ready-2026-09-29/README.md)
records local core flows and restart persistence. Its scorecard-read concurrency
path was subsequently [repaired](validation/score-read-retry-2026-09-29/README.md).
Later reports cover [pairing/start response handling](validation/management-lifetime-2026-09-29/README.md),
[draft tournament editing](validation/tournament-details-2026-09-29/README.md) and
[offline scorecard return](validation/offline-return-2026-09-29/README.md).

These local checks include Chrome at phone and desktop widths. They do not verify
the deployed gg26.no build, public hosting configuration, physical Android/iOS or
native 200% zoom. This checklist refresh adds no new runtime validation results.
