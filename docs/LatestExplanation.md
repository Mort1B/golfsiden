# Latest iteration — visible mobile manual-hole labels

A1s closes the P3 presentation issue found during A1r browser acceptance. The
manual hole grid now shows Hull, Par, Slagindeks and Avstand (yards) headings
at mobile widths. The existing desktop heading rule becomes the base rule with
mobile row spacing; the desktop breakpoint retains its previous spacing and
appearance. Only two CSS rules changed. Component markup, accessible field/hole
names, parsing, validation, state, optional distances and save behavior remain
unchanged.

Real Chrome checks pass at 320, 390 and 1280px. Rendered heading lines remain
inside their columns and viewport, with clean distance-unit wrapping and
separation from the first row. Header/input edges align; directly measured
first-row inputs remain at least 44px. Empty, filled and validation-error rows
are readable, first-invalid focus and Tab order are preserved, and a nine-to-18
resize retains existing values while new rows remain empty.

Each width makes two real private-200 saves, one variable layout and one fixed
18-hole Stableford form. Receipts, collapse/focus and GET/reload verify returned
names/counts; exact stored hole facts are not separately queried. Clearly
injected denial/read-error/empty/long-preset states preserve existing behavior,
including mounted-form retention and no automatic PUT retry. Real logout and
the unchanged unusable-catalog fallback are covered without a remote provider
request.

All 1,133 frontend tests across 147 files, typecheck, lint and production build
pass. The unchanged API build and disposable PostgreSQL migration/seed checks
pass with all 39 migrations, eight players and five rounds. Initial fixture
connection and probe selector/capture issues were resolved before the final
complete browser run. No backend or SQLx regression ladder was rerun for this
CSS-only change. Owned services, container and private environment files were
removed. Independent source/browser and documentation/next-plan reviews found
no actionable findings. Final local links/source anchors, file-size/scope and
diff/EOF whitespace checks pass. The
[acceptance report](validation/manual-hole-labels-2026-10-15/README.md) retains
commands, screenshots and precise validation limits.

A separate measurement-only Fantasy result/refresh candidate is proposed next.
No performance investigation, runtime optimization, provider, dependency,
schema or hosted work began in A1s.
