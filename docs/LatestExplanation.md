# Singles match-play contract definition

The user selected a separate match table awarding **1 point for a win, ½ for a
draw and 0 for a loss**, with draws after 18 holes. Match rounds contribute
nothing to gross/net overall totals. The full future contract and acceptance
examples are in [Architecture](ARCHITECTURE.md#planned-singles-match-play-contract).

This iteration changes documentation only. Match play is **not implemented or
selectable**. Four-ball, Stableford and match play now have definitions; their
implementation follows in bounded steps before the broader code, performance
and security work.

## Defined first version

The first variant is 18-hole singles with administrator-assigned opponents,
a shared tee and hole order 1–18. Every entrant needs exactly one opponent before
opening. The organizer chooses an official gross/net mode while draft, defaulting
to net; it and the handicap snapshots freeze on opening. Net play uses the full
100% singles handicap difference. For Playing Handicaps 10 and 18, the higher
player receives one stroke on stroke indexes 1–8 while the lower receives none.

Authoritative hole reports preserve agreed outcomes separately from numeric
notes. A lead greater than the remaining holes ends the match: three up after
16 is 3&2 and requires no entries on holes 17–18. Two up after 16 is still in
progress. Concessions and organizer awards have explicit provenance and never
fabricate stroke totals. A recorder attests to an actual communicated concession;
permission to enter scores does not let them concede for somebody else.

Real concessions cannot be withdrawn. An explicit audited administrator path
handles recording mistakes or organizer decisions, retaining superseded facts,
invalidating confirmation/points and revalidating any changed finish. Ordinary
numeric edits cannot rewrite agreed outcomes. The app records rulings; it does
not adjudicate golf rules. Accepted online result confirmation is the defined
official reporting point and covers both opponents. Round completion requires
all matches confirmed, including matches that ended early.

Reports, concessions, awards, corrections and confirmation require connectivity
in this first version. Numeric notes on an already-open match card may be durable
offline drafts, clearly pending; they do not award points or publish outcomes.
Reconnection requires revision checks and explicit conflict resolution. A finished
match blocks stale writes and retains conflicting drafts for review. Existing
stroke queues, request fingerprints and receipts must remain compatible.

## Results and boundaries

The private match table sums confirmed results with played/win/draw/loss counts
and shared places on equal points. No unconfirmed match receives provisional draw
points. Gross/net display changes cannot produce a second official match winner.
Match-only tournaments have no overall stroke configuration; mixed tournaments
validate best-N and mandatory rounds against contributing formats only.

The existing final-round tie-break keeps its exact final scheduled round. If
that round is match play, an overall tie stays shared; neither an earlier round
nor match points silently replaces the comparator. An open match round also
cannot displace an eligible provisional stroke contribution.

Hidden-final projections derive only from the permitted prefix and withhold any
outcome dependent on hidden facts. The non-admin match table excludes the entire
hidden final round until release, preventing partial awards or counts leaking
results. Public links keep their existing gross/net overall scope and gain no
match records. Match-only public sharing is unavailable initially.

Nine-hole/shotgun/extra-hole play, partner variants, automatic brackets/byes,
public match sharing, offline authoritative reports and handicap-system submission
are excluded initially. The first match implementation candidate is pure domain
allocation, ordered outcome/terminal derivation and exact half-point arithmetic.
The first queued implementation overall remains the four-ball domain foundation;
no new format becomes selectable until its full vertical path is ready.

## Validation

- Checked the rules against [R&A Rule 3.2](https://www.randa.org/en/rog/the-rules-of-golf/rule-3),
  [NGF handicap guidance](https://www.golfforbundet.no/spiller/regler/world-handicap-system/test)
  and [NGF Handicapreglene 2024](https://www.golfforbundet.no/files/documents/whs-handicapreglene-2024-%E2%80%93-ny-versjon-juni-2024.pdf).
- Read-only code tracing covered opponent readiness, starting-hole metadata,
  stroke-based completion, scoring authority, overall selection and final privacy.
  These paths need a dedicated match aggregate and format eligibility boundary.
- Standalone Python assertions passed for 10/18, −2/14, −4/−1 and 0/40 handicap
  allocations; net hole comparison; 3&2, two-up-with-two-left, 2&1 and 18-hole
  draws; exact half-point totals; visible-prefix arithmetic and eligible-round
  examples. These check the definition's arithmetic, not production behavior.
- Backend, PostgreSQL, frontend and browser ladders were not run because no runtime
  source, schema or UI changed. The full affected ladders, migrations, compatibility,
  authority/concurrency and hidden-result tests remain required in implementation.
- Read-only contract review found no material issue or unresolved product choice.
  Documentation references and diff checks passed.

**READY FOR LATER IMPLEMENTATION PLANNING:** the definition is complete. This is
not an implementation or deployment verdict. The previously recorded nine-hole
handicap boundary remains unchanged.
