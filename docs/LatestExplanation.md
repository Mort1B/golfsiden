# iPhone layout support

Existing flows now account for all four safe-area edges, with shared spacing for
content and separate spacing for fixed navigation. Short landscape views keep the
left navigation rail compact and scrollable. Portrait links stay at least 44px
high even with home-indicator padding. Enlarged navigation labels wrap, with
bottom clearance growing alongside text size.

Public/authentication pages use the small viewport height with a fallback, and
sign-in inputs use 1rem text. For example, at 852×260 the first navigation target
previously began above the viewport; all four now remain reachable. At 200% text
size, navigation labels wrap instead of being cut off. Normal vertical scrolling
remains available. No provider, draft, scoring rule, API or persistence behavior
changed. Chrome and Safari on iPhone remain equal acceptance targets; Duo and
foldable-specific layouts are excluded.

Validation passed: 1,100 frontend tests, typecheck, lint, production build and
strict browser TypeScript; 24 new Chrome cases, 23 WebKit cases, and 10 retained
keyboard/navigation cases. WebKit intentionally skips the Chrome-only CDP inset
simulation. The 14 retained scoring/Fantasy/award cases produced 13 passes and one
HTTP-status assertion failure; that case passed unchanged on a focused rerun.
Its initial unexpected status was not captured and its cause remains undiagnosed.
Independent read-only review found no remaining issues after the portrait touch
target fix. The [acceptance report](validation/iphone-layout-2026-10-14/README.md)
records commands, reproductions, screenshots, fixture repairs and limitations.

This is local implementation and browser evidence. Physical iPhone Chrome and
Safari still need separate checks for real keyboard/toolbars, safe areas and
native zoom on the target devices. No iPhone or iOS Simulator was available on
this Linux host. No hosted infrastructure was changed.
