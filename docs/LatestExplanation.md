# Latest explanation

## Browser-compatible sign-in username validation

The sign-in username field now uses a character class that is valid under the
HTML `pattern` attribute's `/v` regular-expression semantics. The hyphen is
escaped explicitly, removing Chrome's pattern-compilation diagnostic while
preserving the existing browser boundary: 3-32 ASCII letters, digits,
underscores, or hyphens.

Backend account normalization and validation are unchanged. Login still trims
and lowercases ASCII usernames before applying its authoritative grammar, and
invalid syntax still follows the same credential-failure path. No API, session,
database, tournament, handicap, team, score, locking, or standings behavior was
modified.

## Compact example

The JSX expression makes the intended DOM backslash unambiguous:

```tsx
<input pattern={'[A-Za-z0-9_\\-]{3,32}'} minLength={3} maxLength={32} required />
```

## Invariants

- Uppercase ASCII remains usable at sign-in because the backend normalizes it.
- Username length boundaries remain 3 and 32 characters.
- Dot, spaces, non-ASCII letters, and other punctuation remain invalid in the
  browser constraint.
- Authentication response behavior and session ownership are unchanged.
- All golf-domain invariants are unaffected.

## Validation

- Frontend Vitest passes 98 tests across 18 files.
- Frontend strict typecheck, lint, and production build pass; the build
  transforms 1,795 modules.
- Google Chrome 151.0.7922.173 passes at 375x812 and 1440x900 with no pattern
  diagnostic, runtime exception, failed request, unexpected error response, or
  horizontal overflow. The unauthenticated session request was deliberately
  fulfilled with the expected `401` response.
- Both viewports expose the exact DOM pattern `[A-Za-z0-9_\\-]{3,32}`. Chrome
  accepts `abc`, `Player_1`, `abc-def`, and the 32-character boundary; it rejects
  two characters, 33 characters, dot, a Norwegian letter, and whitespace.
