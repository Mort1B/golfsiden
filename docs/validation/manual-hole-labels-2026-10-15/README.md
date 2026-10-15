# A1s — visible mobile manual-hole labels

Base revision: `8d884f2cfe75c960507bbc04259229f5c242f65b`.
Scope: manual-hole heading presentation, frontend/browser acceptance and affected
documentation. The sole production change is two rules in
[course-configuration.css](../../../frontend/src/features/tournaments/course-configuration.css#L43).

## Result and baseline

The existing Hull/Par/Slagindeks/Avstand (yards) heading now displays at every
width. Its grid uses the same column sizes and 6px gaps as mobile input rows;
at 700px both retain their existing 8px gaps. All desktop heading properties are
unchanged. No component, accessible input name, parser, validation, state, save,
authorization, backend, schema or dependency implementation changed.

A1r already reproduced the **P3 low** defect in
[mobile](../course-write-session-2026-10-15/mobile-hole-labels.png) and
[desktop](../course-write-session-2026-10-15/desktop-hole-labels.png) screenshots.
The CSS blob at both A1r's base and this step's base is
`11c7b10d0e58ed3c839cb753794c78208605a98f`; that baseline applies unchanged.
Its heading was hidden below 700px while per-hole row labels were visually
hidden. This step reuses that recorded reproduction rather than claiming a new
original-code browser run.

## Browser evidence

The [real Chrome probe](browser-probe.mjs) and
[results](browser-results.json) pass at **320, 390 and 1280px**. The final run
uses compact native captures of the headings and first three rows:

- [320px empty rows](mobile-empty.png) and [filled rows](mobile-filled.png):
  visible labels, aligned columns, clean wrapping of the distance units.
- [320px validation](mobile-validation.png): readable field errors and visible
  focus on the first invalid Par input, with no horizontal page overflow.
- [390px fixed layout](mobile-fixed.png): the same labels with 18-hole readonly
  hole count and filled inputs.
- [1280px desktop](desktop-filled.png): preserved desktop heading/row geometry.

Checks directly measure each heading's rendered text-line bounds inside its
column and viewport, separation from the first row, matching column edges and
first-row input targets of at least 44px. All rows share the unchanged grid;
the probe does not independently measure every row. Per-hole accessible names
remain usable. Keyboard Tab order is Par → Slagindeks → Avstand.

The variable form changes from nine to 18 holes while retaining the first hole's
Par and optional distance; new rows remain empty until explicitly filled.
Client validation focuses the first invalid field and sends no PUT. Long
course/tee names, disabled saving controls, private success, current access-error
feedback and saved-picker loading/populated/error/retry/empty states are covered.
GET 503, empty/long preset responses and PUT 401 are clearly injected.

Each width performs **two real manual saves**: a variable-layout round and a
fixed-18-hole Stableford round. Real private 200 responses, receipt, collapse,
focus return and authoritative GET/reload verify persistence. The variable
round asserts course/tee names and hole count; the fixed round asserts course
name and hole count. Exact stored hole facts are not separately queried.
The injected denial preserves the authoritative round DTO and current mounted
form inputs, re-enables fields, and causes no automatic PUT retry. No retention
across mode changes, reload or actual auth-loss unmounts is claimed. Real logout
plus reload verifies private round GET 401 and login redirection. The actual
unusable catalog shows its fallback, with zero provider-detail requests; no
successful remote-provider flow is manufactured or claimed.

Initial probe diagnostics masked a selector mismatch after validation added
error text to a wrapped input label. Corrected diagnostics preserve the original
error; the focused-field assertion now uses the existing validation key.
Application form logic was unchanged. Initial long screenshots included fixed
navigation within stitched captures; compact native clips remove that evidence
artifact. The complete final probe, including those captures, passes all widths.

## Validation and fixture

| Check | Result |
| --- | --- |
| Full frontend suite | 1,133 tests passed across 147 files |
| Typecheck, lint and production build | Passed |
| Real Chrome, 320/390/1280px | Passed; six real saves in the final run |
| Independent source and browser-evidence review | No actionable findings |
| Existing API build, fixture migration and seed | Passed |
| Final documentation/next-plan review | No actionable findings |
| Final link, source-size and scoped-diff checks | Passed; 15 scoped files, sole production CSS owner remains 65 lines |

[Command results](validation-results.txt) retain the current-run outcomes.
Backend and SQLx regression ladders were not rerun: only CSS production source
changed. The unchanged API was built for the browser fixture; all 39 existing
migrations and seed ran against disposable PostgreSQL 17.10, with eight players
and five rounds. In that fixture only, the empty draft final round was changed
from individual stroke play to Stableford to exercise the existing fixed-18-hole
form. No published seed or migration changed. The first migration connection
was reset during fixture initialization; after PostgreSQL readiness was checked,
migration and seed completed successfully.

The owned container `golf-a1s-20261015-pg` used loopback port 55463. Browser
contexts and API/Vite services were closed, ports released, the disposable
container confirmed removed and both private environment files deleted.
Raw logs and additional captures remain under `/tmp/golf-a1s-*`; no credentials,
session cookies or database URLs are retained here.

Commands from the repository root, with fixture environment supplied privately:

```bash
cd frontend
npm run test
npm run typecheck
npm run lint
npm run build
```

```bash
cargo build --locked -p golf-api --bins
cargo run --locked -p golf-api --bin migrate
cargo run --locked -p golf-api --bin seed
node docs/validation/manual-hole-labels-2026-10-15/browser-probe.mjs
```

Browser reproduction needs the separate seeded database and Stableford fixture
adjustment, the API in development mode with HTTP cookies on 3000, existing Vite
on 5173, and `GOLF_A1S_DEV_PASSWORD` set privately. No runtime optimization,
dependency scan, hosted check or physical-device acceptance ran in this step.
