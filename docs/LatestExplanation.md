# Latest explanation

## Read-only tournament management workspace

Tournament administrators now have one private management index at
`/manage/tournaments/:tournamentId`. It organizes the existing tournament,
roster, round, course-presence, pairing-link, lifecycle, and invitation facts
into seven semantic sections without pretending that future editors or provider
integration already exist.

The route validates the identifier, loads the session-owned membership list and
private tournament detail, and enables roster and round reads only after both
facts confirm the exact tournament and an `admin` membership. Invalid, missing,
forbidden, loading, retryable-error, empty, and populated states remain distinct.
This frontend gate improves presentation; existing backend authorization remains
the security boundary.

Invitation administration is linked from the workspace. Its labelled return
replaces the invitation history entry, then the asynchronously mounted workspace
validates the hash, scrolls to the Invitations section, and restores visible
focus. Browser Back therefore does not reopen the invitation page.

## Compact example

Secondary private reads cannot start from membership data alone:

```tsx
const access = resolveManagementAccess({ memberships, tournament, ...queryState })
const enabled = access.state === 'ready'

useQuery({ queryKey: tournamentKeys.rounds(userId, tournamentId), enabled })
```

## Invariants

- Tournament-specific backend membership remains authoritative for every read
  and mutation; global roles gain no bypass.
- The workspace is read-only and introduces no provider request, fake mutation,
  course-revision claim, or team-query fan-out.
- Tournament players remain independent of round teams, and handicap snapshots,
  score ownership, locked-round protection, audit history, and separate gross/net
  standings are unchanged.
- Private query keys remain session-owned and identity transitions still clear
  the workspace cache.

## Validation

- Focused management access, section, and hash tests pass 6 tests.
- The complete frontend suite passes 114 tests across 20 files.
- Strict typecheck, lint, and production build pass; Vite 7.3.6 transforms 1,800
  modules.
- Google Chrome 151.0.7922.173 passes the populated admin workspace at 375x812
  and 1440x900, plus phone-width empty, non-admin, signed-out, invalid-ID,
  missing, retryable-error, and delayed-loading scenarios.
- Chrome verifies all seven anchors and sections, 44-pixel navigation targets,
  visible hash focus, invitation round-trip and browser-Back behavior, long
  content, and no horizontal overflow, runtime exception, failed request, or
  unexpected console/network error.
- Independent review found and then verified fixes for invitation return/focus
  history and mutually exclusive roster async states; no findings remain.
