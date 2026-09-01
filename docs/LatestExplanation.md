# Latest explanation

## A mandatory round reserves one of best N

A tournament admin can now combine a normal best-N rule with one optional round
that must count. In a four-round tournament using best three, selecting round
four means every eligible gross or net total contains round four plus that
metric's best two results from rounds one through three. If a player misses round
four, only two optional results can count and the player remains ineligible.

The same rule covers the edge case where N is one: only the mandatory result can
be ranked. Gross and net still choose their optional contributions independently,
and team-owned results remain attributed to the exact frozen members of that
round.

```rust
let optional_slots = if mandatory_round_id.is_some() {
    required.saturating_sub(1)
} else {
    required
};
```

## Configuration is atomic and permanently scoped

Migration `0016` adds nullable `tournaments.mandatory_round_id` with a deferred
same-tournament foreign key. The deferred constraint lets onboarding preallocate
round UUIDs and create the tournament and its selected future round in one
transaction, while rejecting cross-tournament IDs and direct deletion of a
selected round.

The existing configuration PATCH now requires counted N and an explicit UUID or
JSON `null`. It locks the tournament's rounds first, reauthorizes the exact
tournament admin, applies optimistic concurrency, and changes both facts or
neither. Start, opening, or a captured handicap snapshot freezes the pair
permanently. No-op requests preserve the timestamp and send no live event.

The frontend mirrors that contract in creator onboarding and tournament
settings. Stable draft keys keep the onboarding choice attached to the intended
round and clear it when that round is removed. Settings and standings validate a
non-null mandatory UUID against the exact decoded tournament round collection
before caching it. Standings show the selected round by name as completed or
missing in both gross and net views.

## Validation

Automated backend, PostgreSQL, and frontend checks cover set, replace, clear,
no-op, cross-target rejection, selection-versus-deletion races,
start/open/snapshot freezing, onboarding removal, N=1, metric-specific selection,
and both missing and completed label contracts. Chrome at phone and desktop sizes
verified onboarding removal, an atomic settings save, a locked tournament, long
names, gross/net switching, and the missing label with a clean console/network.

The next bounded roadmap step remains Phase 7 live best-N standings, final-nine
blackout, and scorecard drilldowns. Additional play modes remain deferred until
the roadmap, optimization, and security review are complete.
