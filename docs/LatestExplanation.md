# Public result-sharing scope proposal

The next roadmap item is public result-sharing after configurable tournament
tie-breaks. `docs/PLANS.md` now records a concrete proposed first slice: an exact
admin deliberately creates a revocable 30-day link to live overall gross/net
standings with existing player display names. The user-facing scope choice is
pending; this iteration changes documentation only and publishes no private data.

The proposed page exposes summary results without account details, contribution
histories, scorecards or scoring controls. It always uses the non-admin final-round
projection, including for an administrator opening the public link. Existing
private leaderboard responses cannot be reused directly: they include global
player identities, preserved owner history and current-team details. The current
repository helper with no user deliberately produces unrestricted results, so
public loading needs its own explicit authorized projection boundary.

The plan specifies hashed random capabilities, transactionally checked expiry and
revocation, a dedicated public cache and 15-second foreground refresh. It records
that an already delivered snapshot may remain visible until the next refresh;
new reads independently reauthorize. Tournament-owned grants survive issuer
role changes, while all management actions require current exact-admin authority.
Final implementation must preserve existing member-only endpoint behavior.

## Validation and status

Inspected the clean branch, repository workflow and active queue, leaderboard
assembly/visibility, public token handling, frontend routing and cache ownership.
Read-only review identified four required boundaries: explicit public visibility,
an allowlisted response, transactional capability checks and separate public
refresh/cache handling. All four are recorded in the proposed step.

`git diff --check` passed. No runtime source, migration or dependencies changed;
backend, database and browser ladders do not apply to this planning-only update.
Implementation awaits the user's public scope selection. Offline scoring and
other queued features remain deferred.
