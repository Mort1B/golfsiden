# Latest explanation

## One browser-compatible username constraint

Every username input now uses one shared HTML `pattern` value. The sign-in page,
invitation registration, invitation sign-in, and onboarding account creation all
emit the `/v`-compatible pattern `[A-Za-z0-9_\-]{3,32}`. Centralizing the value
prevents one account surface from reintroducing Chrome's character-class
compilation error while another remains valid.

The shared constant is deliberately limited to the browser constraint. Backend
normalization and validation remain authoritative, and onboarding's imperative
validation still trims user input before checking the same visible grammar.
Required fields, length attributes, submissions, error states, invitation
handling, tournament creation, authentication, and sessions are unchanged.

## Compact example

All four inputs import the same escaped value:

```tsx
export const USERNAME_HTML_PATTERN = '[A-Za-z0-9_\\-]{3,32}'

<input pattern={USERNAME_HTML_PATTERN} minLength={3} maxLength={32} required />
```

## Invariants

- Username inputs accept 3-32 ASCII letters, digits, underscores, or hyphens.
- Uppercase ASCII remains usable because the backend normalizes case.
- Dot, spaces, non-ASCII letters, other punctuation, and out-of-range lengths
  remain invalid at the browser boundary.
- Backend credential behavior, session ownership, invitation semantics, and
  onboarding payloads are unchanged.
- Tournament, round-team, handicap snapshot, score ownership, locking, audit,
  and gross/net standings invariants are unaffected.

## Validation

- Focused `/v` compilation and grammar coverage passes 10 tests.
- The complete frontend suite passes 108 tests across 19 files.
- Strict typecheck, lint, and production build pass; Vite 7.3.6 transforms 1,796
  modules.
- Google Chrome 151.0.7922.173 passes onboarding and both invitation account
  forms at 375x812 and 1440x900. All three fields expose the exact shared DOM
  pattern and pass the accepted/rejected boundary matrix.
- Chrome reports no pattern diagnostic, runtime exception, failed request,
  unexpected error response, or horizontal overflow. The unauthenticated session
  check returns its expected `401` response.
- Independent review found no correctness, security, accessibility, structural,
  or regression issue.
