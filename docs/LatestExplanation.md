# Keyboard focus clears the fixed navigation

The bounded accessibility repair closes the three UI findings from the friends'
deployment assessment. The changes are confined to shared CSS: native keyboard
scrolling reserves room for the mobile menu, back controls are 44×44px, and small
tournament section counts use a darker existing text color.

At 320×600, Tab previously placed the second profile tournament card partly
behind the fixed menu. Private-shell root scroll padding now keeps the focused
card visible. The spacing includes the bottom safe-area inset and changes at the
700px desktop navigation breakpoint. Public/sign-in pages retain their ordinary
scrolling. No focus event handler, router state or scoring behavior changed.

The three failures were reproduced before the repair. Chrome regressions now
require all three profile cards to receive focus forward and backward at 320,
390, 699, 700 and 1280px; they check geometry, hit testing and visible focus. Further
checks cover score controls, back-target dimensions, contrast, synthetic safe-area
clearance and sign-out into the public layout. Text contrast improved from 4.4705:1
to 5.5627:1. Screenshots and the exact boundaries are in the
[repair validation](validation/navigation-accessibility-2026-09-23/README.md).

All 751 frontend tests, typecheck, lint, build and strict browser-test TypeScript
passed. All 52 affected Chrome route cases passed, including the 10 new regressions.
Independent read-only review found no remaining production-diff issue.

The repair is **READY** within its scope. Overall deployment sign-off remains
**NOT READY**: the separate security review, deployment/restore rehearsal, native
200% zoom and physical Android checks remain outstanding. No backend or database
code changed, so their full ladders were not rerun. No application was deployed.
