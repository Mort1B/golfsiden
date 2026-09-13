# Individual Stableford contract definition

Stableford is defined as an 18-hole individual format with separate gross/net
points. The user chose to include it in mixed-format overall standings using
**36 minus points** for each completed round: 40 points contributes −4. This
preserves Stableford's limited damage from bad holes and is explicitly a
points-derived contribution, not an actual stroke total.

The full future contract, sources and acceptance examples are in
[Architecture](ARCHITECTURE.md#planned-individual-stableford-contract).
This iteration changes documentation only. Stableford is **not implemented or
selectable**. Match play is the next separate definition; implementation follows
all three definitions, ahead of the broader code, performance and security work.

## Scoring and state decisions

For a numeric entry, gross points are `max(0, 2 + par - gross)` and net points are
`max(0, 2 + par - (gross - received_strokes))`. The server derives both views from
the same preserved gross input and handicap snapshot. An explicit pickup/no-score
returns zero points in both views. An unentered hole remains unresolved and cannot
be silently turned into a pickup at confirmation.

The first variant uses par as the target and defaults to the Norwegian 100%
individual allowance, configurable within the existing draft 0–100% range. It
retains the full unrounded calculation until final signed rounding. The new
format policy must preserve every existing format's calculations and historical
snapshots. Nine-hole play, per-player tees, modified/team Stableford, formal
rules adjudication and handicap-system integration remain excluded initially.

Progress counts resolved numeric or explicit no-score holes. Thus 18 zero-point
holes can be complete, while an empty card cannot. A full actual stroke total is
unavailable if any hole lacks numeric input. Correcting a numeric zero-point score
still advances its revision and invalidates confirmation even if points do not
change. Numeric/no-score transitions retain identity and audit history; no
physical score deletion or fabricated stroke score is allowed.

Confirmation stays online-only after pending delivery/verification has finished,
with fresh authorized data and the existing local confirmation lease. Offline
operations retain account isolation, immutable payloads, exact-version conflicts,
predecessor-based successors and explicit conflict choices. New outcome payloads
must preserve old serialized fingerprints, receipts and pending device requests.

## Standings and compatibility

Native round results rank points descending. Overall standings compare the
converted contribution ascending, independently for gross/net, while preserving
best-N, mandatory slots, qualification and the existing final-round tie policy.
For permitted partial cards use `2 * resolved_visible_holes - visible_points`,
not 36 minus a partial score. No resolved holes means no contribution.

The current pipeline treats gross/net totals as actual strokes in domain types,
selection, transport, frontend decoders and public rendering. It therefore needs
explicit result kinds and comparison values before supporting Stableford. Putting
`par + converted_value` into an actual-total field would misrepresent the result.
Native points, actual strokes and overall equivalents must remain distinct in
cards, history and public labels. Existing stroke-only contracts must be preserved
or intentionally versioned with matching decoders.

Hidden-final filtering happens before points, progress, comparisons or public
projection. Public sharing remains limited to overall results and existing display
names, with only the non-private value-basis metadata needed to label results
accurately. No hole states or private/account details are added.

The first bounded implementation candidate is the pure domain conversion and
result-type foundation with acceptance tests, keeping the format unavailable.
Playable support requires later coordinated persistence, lifecycle, API, result,
offline and UI slices before exposure.

## Validation

- Checked the rules against
  [R&A Rule 21.1](https://www.randa.org/rog/the-rules-of-golf/rule-21),
  [NGF Stableford guidance](https://www.golfforbundet.no/spiller/regler/world-handicap-system/godkjente-handicaptellende-spilleformer)
  and [NGF Handicapreglene 2024](https://www.golfforbundet.no/files/documents/whs-handicapreglene-2024-%E2%80%93-ny-versjon-juni-2024.pdf).
- Traced existing stroke-total identities, ranking direction, unstarted/progress
  handling, confirmation, receipt compatibility and private/public decoding.
- Independently checked the documented arithmetic with Python and exact fractions:
  points and signed handicap rounding, pickups versus blanks, net points above
  six, partial/hidden-prefix conversion, full zero-point completion, capped
  equivalent versus actual strokes, mixed best-N and final-round comparison.
- The six-hole example gives 6 gross/9 net points and equivalents +6/+3. A par-72
  round of 17 pars plus a 10 on a par-4 has actual gross 78 but 34 points and a +2
  contribution. The mixed best-two example yields A +3/−7 and B +1/−5.
- Read-only contract review found no material issue or unresolved product choice.
  Documentation references and diff checks passed.
- Backend, PostgreSQL, frontend and browser ladders were not run because no runtime
  source, schema or UI changed. Their affected checks, plus legacy receipt/queue
  upgrade and hidden-result regressions, are required during implementation.

**READY FOR LATER IMPLEMENTATION PLANNING:** the definition is complete. This is
not an implementation or deployment verdict. The existing nine-hole handicap
boundary recorded during four-ball definition remains unchanged.
