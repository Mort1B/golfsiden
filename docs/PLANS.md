# Plans

`PLANS.md` owns unresolved work and bounded next steps. Current behavior belongs
in `Documentation.md`; durable boundaries belong in `ARCHITECTURE.md`.

## Active step

None.

## Next candidate

### IPHONE-LAYOUT — iPhone 15 Pro and newer

**Status:** planning only; implementation awaits a subsequent request. The user
confirmed that dual-screen/Duo and foldable layouts are outside this step.

**Goal:** make existing flows comfortable and fully usable on single-screen
iPhones from iPhone 15 Pro/Pro Max onward, including newer standard and larger
models. Portrait is the primary layout; landscape must remain usable. Fit means
readable content and reachable controls with normal vertical scrolling, rather
than squeezing an entire page into one screen.

**Current evidence:** the shared shell already reserves bottom safe-area space,
but its 700px width breakpoint also selects the desktop rail on landscape phones.
The viewport meta tag uses the default safe-area behavior. Existing Fantasy
browser checks use Chrome at 320, 390 and 1280px widths with a fixed 900px height;
they do not establish iPhone Safari, software-keyboard or browser-toolbar behavior.
These are inspection targets, not confirmed layout defects.

**Scope and ownership:** one frontend layout step, owned by the primary agent.
Inspect the viewport configuration, shared shell/navigation/header CSS, then
change feature CSS or minimal semantic markup only for reproduced fit problems.
Cover sign-in/onboarding/join/profile, tournament and round management, scoring
and scorecards, standings/history/public results, Fantasy presets and breakdowns,
and the optional round-award controls. Add focused browser coverage and update
affected behavior/architecture docs plus LatestExplanation. Backend, API,
migrations, scoring rules, new navigation flows, visual rebranding, native apps,
installation/offline features and a separate tablet redesign are outside scope.

**Required behavior:**

- Use fluid layouts rather than device-name or user-agent branches. Preserve the
  existing 320px minimum and desktop usability while prioritizing modern iPhones.
- Keep content and navigation clear of the top cutout, rounded edges and home
  indicator in both orientations. Evaluate `viewport-fit=cover` together with
  all four safe-area insets; do not add it without corresponding content spacing.
  Retain ordinary padding when safe-area insets are zero.
- Verify the compact-height landscape layout before changing the 700px rail
  breakpoint. Navigation, headers and scoring controls must leave usable content
  space and keep the final action reachable at the end of a page.
- Accommodate Safari toolbar expansion/collapse, rotation and the software
  keyboard. Focused fields, validation messages and submit actions must remain
  reachable by scrolling. Choose viewport-height units from reproduced behavior;
  preserve pinch zoom and avoid forced focus/scroll workarounds without evidence.
- Prevent page-level horizontal overflow, clipped labels and overlapping fixed
  elements. Existing intentional scorecard scrollers remain contained and usable.
  Long Norwegian names, tournament titles and result values must wrap or have an
  accessible way to read the full content.
- Keep primary scoring/navigation targets at least 44x44 CSS pixels, visible
  focus, readable labels and usable text enlargement. Preserve loading, empty,
  error, pending, retry and locked states within the same layouts.

**Implementation sequence within this step:** reproduce and record a small
route/state/viewport defect list; fix shared geometry first; address only related
feature-level fit defects; add regression checks; review, validate and document.
Unrelated usability findings go into a separately bounded candidate.

**Validation:**

1. Use logical CSS viewports, not physical screen pixels. Start with 393x852 for
   iPhone 15 Pro and representative 390x844, 402x874, 430x932 and 440x956 sizes;
   these are coverage samples, not an exhaustive promise about future hardware.
   Check portrait and swapped landscape dimensions, shorter available heights,
   plus 320px and 1280px regression widths. Confirm actual device dimensions when
   running Safari acceptance.
2. Exercise each scoped flow in its relevant loading, error, empty, populated,
   long-content and disabled states. Assert overflow, control dimensions and
   fixed-element overlap; inspect viewport screenshots, console and failed
   requests. Include scrolling to the final control, field focus, rotation and
   retaining in-progress scoring/Fantasy input during resizing.
3. Run a real WebKit browser alongside retained Chrome coverage. Separately
   verify Safari on an iPhone 15 Pro and a newer/larger iPhone where available,
   recording device, iOS and browser versions. Check toolbars, keyboard, safe
   areas and 200% text/page zoom. Simulator evidence must be identified as such;
   resized Chrome and automated WebKit do not prove physical iPhone acceptance.
4. Run the full frontend ladder: tests, typecheck, lint and build, plus browser
   TypeScript checks and relevant existing browser regressions. Review the diff
   for layout-only scope and unchanged state ownership. Record exact unavailable
   device/tooling checks as acceptance gaps rather than claiming they passed.

**Invariants:** preserve all golf/Fantasy calculations, lock and administrator
rules, API contracts, privacy/session boundaries and exact-write recovery.
Responsive changes must not remount query/recovery providers, clear draft inputs,
trigger writes or weaken score-navigation guards. Retain semantic controls and
the existing visual language; keep source/CSS within repository size limits.

**Stop condition:** close after the scoped fixes, review, applicable validation
and documentation are resolved or explicitly recorded, then commit/push only
this step. Any missing Safari/device evidence remains an explicit acceptance gap.
Do not start a redesign, hosting work or another queued feature.

Design references: [Apple layout guidance](https://developer.apple.com/design/human-interface-guidelines/layout)
and [WebKit safe-area guidance](https://webkit.org/blog/7929/designing-websites-for-iphone-x/).

Every implementation step updates affected durable docs and LatestExplanation,
removes its completed plan item, commits only its scope and stops before the
next. No budgets, transfer penalties, newly generated picks, sporting team
generation, public Fantasy sharing, prizes/payments or offline queued selection
writes are planned.

## Later queue

### Previously deferred work

- Optional offline preparation for browser closure/eviction: requires a separately
  scoped app-shell and private scorecard-retention/reauthorization design before
  implementation. Current in-memory return and durable queued edits remain distinct.

- Deferred broader security assessment: further production proxy/database privilege,
  recovery and dependency-advisory investigation. Revisit for a specific finding
  or a separately requested assessment, rather than making another broad review
  a prerequisite for continued functional testing.

### Remaining deployment acceptance

The remaining PERSIST-1 same-account/actual-login scenarios may be revisited if
testing feedback warrants them; see the original
[validation limits](validation/match-note-retention-2026-09-23/README.md). Local
PostgreSQL validation is now available through rootless Podman. The functional
checks cover real database-backed match recovery after locking, but do not claim
every separate session-replacement scenario.

Public-host acceptance still needs a Docker Engine environment and public DNS/TLS
validation. Native 200% browser zoom and physical Android Chrome remain unverified.
Hosting and deployment checks belong to the user. These historical acceptance
limits do not expand the application-readiness step into server administration
or another assessment. Broader device coverage and polish can follow feedback.

No further sporting scoring rules are planned. No automatic opponents/byes/brackets, team match play,
extra holes, public match sharing, cold offline launch or background sync is included.
Fantasy remains separate from sporting scores and completion rules.
