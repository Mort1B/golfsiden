# Implement the Fantasy domain foundation

FANTASY-2 adds pure backend scoring and lineup-resolution modules. The Fantasy
game is not yet available: no routes, persistence, migrations or UI are included,
and sporting behavior and schema 34 are unchanged.

Non-match scoring applies preserved net hole categories, physical-ace precedence,
pickup penalties, native-format placement and shared team attribution. Teams are
ranked once before both partners receive their points. Match play consumes the
accepted sporting result only: win +3, draw +1, loss -1. Captain multiplication
includes negative totals; win/draw/loss/win with the loser captained totals five.
Separate golfer and manager standings sum every expected round with shared ties.

Lineups require four unique golfers and a captain among them. A valid current
submission wins; otherwise the nearest earlier locked lineup carries forward
when all four remain eligible. It retains the captain but uses current-round
results. Missing or invalid fallbacks are explicit states, never generated picks.

The result types distinguish pending recorded points, provisional/final totals,
withheld results and future rounds. Explicit future rounds permit provisional
overall ranks; unexpected missing contributions do not become zero. Incomplete
cards stay unranked, and concealed results conservatively suppress affected
totals/ranks. Revision-bound non-finish handling retains recorded points without
placement or invented remaining-hole penalties. Source tokens and deadline facts
are adapter inputs; authorization, locking and persistence remain future work.

The [architecture](ARCHITECTURE.md#fantasy-competition-design) documents module
boundaries and limitations. [PLANS.md](PLANS.md#next-candidate) queues FANTASY-3
persistence and selection APIs, with projection/UI integration after that.

Validation passed:

- `cargo test -p golf-api domain::fantasy --lib`: 34 Fantasy tests.
- `cargo test --workspace --all-targets`: 249 tests (247 library, two CLI).
- `cargo fmt --all -- --check` and strict all-target/all-feature Clippy.
- Six added documentation links/anchors, source-size limits and whitespace.
- Independent read-only source and documentation review: no blocking findings.

The first full test attempt could not bind eight course-provider mock servers
inside the sandbox; rerunning with local loopback access passed. Clippy identified
one collapsible conditional, which was fixed; focused tests passed again afterward.
Only existing vendored SQLx warnings remain. Database-feature tests, browser and
frontend checks are not applicable to this pure-domain step; those layers did
not change and no such validation is claimed.
