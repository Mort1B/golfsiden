# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None. See the [latest explanation](LatestExplanation.md) for the completed iteration.

## Next candidate

**Wider application and operational security review (awaiting approval).** Define
and review a bounded assessment of authorization, session handling, private-data
boundaries and production operations before implementation. Keep findings separate
from repairs; prioritize concrete reproducible risks and validation evidence.

## Later queue

No additional implementation step is currently approved.

### Next candidate: navigation and focus accessibility repairs

**Proposed; awaiting implementation approval.** Findings and reproduction are in
[the friends deployment assessment](validation/friends-2026-09-23/README.md).

Goal: keep keyboard-focused content readable above fixed navigation and meet the
existing touch-target and text-contrast requirements.

Scope: shared frontend focus/scroll clearance for the profile tournament cards,
the 40px back-navigation target, and low-contrast tournament section counts.
Preserve the visual language, navigation destinations and all scoring, account,
team and privacy behavior. No API, migration, new features or redesign.

Behavior: at 320×600, keyboard traversal must keep the focused card's identifying
content clear of the bottom menu without manual scrolling. Primary back controls
must measure at least 44×44 CSS pixels. Small section-count text must reach 4.5:1
contrast on its actual background.

Validation: begin with the recorded failures; add focused browser regressions for
keyboard focus and geometry, both sides of 700px, mobile/desktop and long content.
Check score/profile/management clearance, error/empty/populated states, focus
visibility and touch targets. Run the full frontend ladder and Chrome checks;
use native 200% zoom if available and explicitly record any remaining device gap.
Require read-only review and update affected behavior/validation documentation.

Invariants: preserve all root product rules, private-data isolation, contextual
navigation, pending-write guards and existing mutation targets.

Stop: publish only the reviewed, validated accessibility repair, then wait.
Deployment sign-off still requires the separate security assessment, recovery
validation and remaining browser/device gates from the report.

No automatic opponents/byes/brackets, team match play, extra holes, new scoring
rules, public match sharing, cold offline launch or background sync is included.
