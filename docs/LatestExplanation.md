# Playing-day frontend consistency

Writable stroke/team, four-ball and Stableford now follow the same sequence:
tournament/round and owner, current hole, format-specific input, save status,
previous/next and secondary controls. Quick card switching and hole/summary
controls sit above the full summary. **Kontroller manglende hull** or **Se over
scorekortet** follows actual distinct-hole progress, independent of the current
hole. Review moves keyboard focus to a retained progress region; confirmation
remains an explicit, server-authorized action with all existing pending guards.

The progress display combines the current card with scoped local intent while
separately reporting server-checked holes without pending edits. Correcting one
hole still counts one hole. Four-ball needs a numeric partner result; Stableford
pickups resolve a hole. These display counts never enter score totals, results or
confirmation state. Existing inputs, calculations, revision protocols and queues
are unchanged.

Fantasy separates **Min firer** and **Poengtavler**. A valid saved lineup shows a
compact receipt; **Endre valg** opens and focuses the editor. Drafts, uncertain
requests and conflict recovery remain in their existing provider across section
changes and reconnect. Warnings remain above the panels with a direct return to
the unresolved lineup. Protected query projections still clear; viewing state
contains only identifiers/preferences and resets with the session.

The tournament overview now has distinct loading, retryable error, empty and
populated states. One **Fortsett scoreføring** action uses fresh existing round/
access data and a contextual resume URL. It prefers a valid remembered round in
this tournament, otherwise the existing current-round preference. Disconnection,
refresh and session replacement require fresh checks. Other states explain how
to proceed. It checks one candidate, without scanning all rounds for permissions.

Secondary labels and instructions are more readable, with scoped 14px/16px text,
large controls and wrapping. Screenshot review also led to a narrow-screen
status row beneath long overview titles.

For example, enter two holes offline and correct the first: the screen still
shows two holes entered, zero checked on the server and two holes awaiting
verification. Review opens the summary without confirming. Reconnect drains the
existing queue and authoritative reads update the checked count and totals.

Three new parent regressions failed before implementation. Final frontend
validation passed 1,036 tests across 137 files, typecheck, lint and production
build. All 40 distinct affected Chrome scenarios passed at 320/390/1280px,
including explicit keyboard navigation, pending/offline changes, card switching
and review/confirmation. Read-only review found and resolved two focus issues; final review found
no remaining production defects. Browser results, commands, first-run failures,
screenshots and limits are in the [validation report](validation/playing-day-2026-10-09/README.md).
No backend or schema changes, hosted deployment or physical-device acceptance
are included. No queued work was started.
