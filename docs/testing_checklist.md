# Keep testing the application

Use a separate test tournament on your deployment at gg26.no. Keep it between
sessions so you can test returning to saved work. Hosting, deployment and server
operations remain yours; the existing [deployment guide](deployment_guide.md)
contains those procedures.

## A practical first session

1. **Sign in and set up.** Sign in as an organizer, create a tournament, add the
   intended rounds and select/configure their courses. Issue an invitation and
   join with a separate player account in another browser or on your phone.
2. **Prepare the round.** Assign players to flights and, for team formats, assign
   the teams yourself. Save the setup, start the tournament and open the round.
3. **Enter and edit scores.** Sign in as the player on your phone, open **Score**,
   enter a hole score, wait for **Lagret på serveren**, then change it. Check that
   you are still on the intended round and player/team.
4. **Check results.** Open **Resultater** in a second session. Check **Brutto** and
   **Netto**, the scorecard and the selected round. Net values use the round's
   preserved handicap. Final-round visibility can deliberately hide results.
5. **Return to saved work.** Reload, leave the score page and return, then sign
   out and sign in again after saving. The same score should remain. After your
   next ordinary deployment/restart, check it again without resetting the data.
6. **Finish a card and round.** Fill the remaining holes and confirm the scorecard.
   Once every required card is confirmed, complete and lock the round as organizer.
   Ordinary score editing should then be unavailable.
7. **Continue using the formats you need.** Try an individual round and a team
   round; include Stableford, four-ball or singles match play if you intend to use
   them. For offline testing, wait for the explicit saved-on-device state before
   leaving and check that reconnecting eventually saves to the server.

For a bug report, give the page/round/format, phone or browser, the steps taken,
what you expected and what happened. Include whether the app said saved on the
device or saved on the server. Do not include passwords or invitation/recovery
secrets. Prioritize an unusable flow, missing saved scores or incorrect access;
smaller improvements can wait.

## Current evidence and limitation

See the [functional-readiness report](validation/test-ready-2026-09-29/README.md)
for the local checks and their limits. That evidence does not verify your hosted
configuration or a physical phone.

One earlier local run recorded a transient HTTP 500 while reading a scorecard
from PostgreSQL. Saved scores and results remained correct, and a clean repeat
passed. The overlapping action was not proven; report it if it recurs during
normal scoring, especially if retry or returning to the page does not recover.
