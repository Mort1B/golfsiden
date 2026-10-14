# iPhone layout plan

The next candidate in [PLANS.md](PLANS.md#next-candidate) covers the existing
application on iPhone 15 Pro/Pro Max and newer single-screen iPhones. The user
confirmed that Duo/dual-screen and foldable layouts are excluded. This iteration
is documentation-only; implementation has not started. Chrome and Safari on
iPhone are equal acceptance targets.

The plan prioritizes readable portrait layouts and usable landscape layouts,
safe-area spacing, Chrome/Safari toolbar and keyboard behavior, reachable navigation
and scoring controls, long content and accessible touch targets. Existing 320px
and desktop layouts remain regression requirements.

Inspection found an existing 700px desktop-navigation breakpoint and Chrome
viewport checks at fixed heights. Those facts identify what to investigate;
they do not establish an iPhone defect. For example, rotating a scorecard must
keep the current input and make its final action reachable, even when the
software keyboard reduces the visible area.

Implementation will first reproduce fit issues, then repair shared geometry and
only the affected feature layouts. Automated Chrome and WebKit checks supplement
separate acceptance in installed Chrome and Safari on iPhone. Each browser must
meet the same criteria; a pass in one does not establish a pass in the other.
Resized desktop Chrome is not iPhone Chrome evidence. Unavailable physical-device
checks must remain explicit gaps per browser. Backend contracts, sporting/Fantasy
rules and recovery state ownership are outside the change.

Planning validation consists of reviewing scope against the repository rules,
checking referenced paths/anchors and checking the documentation diff. No runtime
tests or device acceptance were run for this documentation-only iteration.
Current product behavior and deployment instructions remain unchanged.
