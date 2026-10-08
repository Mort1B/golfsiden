# FANTASY-1 contract closure in progress

The user started the plan. FANTASY-1 is active and remains documentation-only;
no Fantasy runtime, migration or UI has been implemented. Match-result bonuses,
match net/concession scoring and missed-lineup behavior have been presented as
concrete choices and are awaiting user answers. FANTASY-2 has not started.

The contract now defines Fantasy-only non-finish settlement. An exact tournament
admin records a reasoned, revision-bound disposition for an individual or shared
team score owner. Recorded points remain, unplayed holes add nothing, and the
non-finisher receives no placement. Once every owner is confirmed complete or
explicitly disposed, Fantasy ranks complete finishers using the format's net
ranking policy. The golf round keeps its own state and completion rules.
Later source edits invalidate the disposition until re-attested, preserving
history and preventing a stale final result. Both partners retain the shared
result in team formats. All-DNF fields have no placement awards.

Lifecycle defaults use authoritative opening or an earlier published deadline,
private picks before lock, online-only atomic submissions and no admin pick
replacement. Expiry eligibility is preserved so a later withdrawal cannot
retroactively invalidate a locked selection. Disabling is allowed only before
any lineup locks and never deletes history. These boundaries are included in
the future persistence, projection and admin UI steps.

Source inspection confirmed separate match numeric/concession provenance,
opponent-relative match handicap allocation, zero playing-handicap snapshots in
gross mode, and distinct sporting completion requirements. Fantasy match scoring
must explicitly preserve any full net handicap it needs at opening, rather than
reuse zero gross-mode or opponent-relative values or today's handicap.

The [planned contract](ARCHITECTURE.md#fantasy-competition-design-planned-not-implemented)
and [active step](PLANS.md#active-step) distinguish settled technical defaults
from the product decisions still pending. Documentation checks passed for three
added links/anchors, four-document scope, DNF/placement examples and whitespace.
Read-only review identified a missing whole-card revision definition; the contract
now covers absence, both partners and retained owner generations, with acceptance
examples. Follow-up review confirmed closure with no remaining findings. No
application, database, browser or deployment tests are applicable.
