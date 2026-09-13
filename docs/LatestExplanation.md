# Four-ball contract definition

The next format is defined as 18-hole, two-player four-ball stroke play. Players
keep their own hole scores; the server derives the team's best gross and best net
result on each hole independently. The user chose to credit the team's round
result to both preserved partners in individual overall standings, just like
existing team rounds. Best-N and the tournament's mandatory/final-round policies
continue to apply.

The full future contract, sources, examples and implementation boundaries are in
[Architecture](ARCHITECTURE.md#planned-four-ball-stroke-play-contract). It is
explicitly **not implemented or selectable**. This iteration changes documentation
only; Stableford is the next separate definition, then match play. Implementation
of agreed formats precedes the later broad code, performance and security work.

## Defined behavior

- Teams remain administrator-managed and specific to one round. Each team has
  exactly two players in the same flight, using one shared course/tee layout.
- Each entered score belongs to its player. Team results and confirmation belong
  to the side. A team handicap must not be invented to fit today's result DTOs.
- The draft allowance defaults to 85% per player and remains configurable within
  the existing 0–100% range. Apply it to the unrounded Course Handicap, round once
  with signed halves toward positive infinity, and preserve the opening snapshot.
- One valid numeric partner entry makes a side-hole scorable. An unentered or
  picked-up partner does not need a fabricated score. Explicitly changing a
  recorded numeric score to no-score requires an audited, versioned operation;
  retained identity and receipts preserve offline conflict and replay guarantees.
- Team confirmation requires a valid result on every hole and fresh server data.
  On this account/device, pending edits on either partner block confirmation, and
  the local lease must cover both cards. Changes to either input invalidate the
  side confirmation, including previously nonwinning entries.
- Team history retains the correct round's partners. Hidden-final projections
  and public-sharing restrictions remain intact.

The key architectural extension is separating entered-score ownership from
competition-team and confirmation ownership. Today those are coupled in the
format policy, opening/pairing guards, result assembly, confirmation and SQL
triggers. Four-ball needs a coherent end-to-end extension; adding only an enum
would not supply playable support.

The first bounded implementation candidate is a pure domain foundation with
per-player allowance and side-hole aggregation types plus acceptance tests. It
must preserve all existing behavior and keep four-ball unavailable until the
subsequent persistence, lifecycle, projection and UI paths are complete. This is
queued after all three format definitions.

## Validation and limits

- Checked the scoring basis against
  [R&A Rule 23](https://www.randa.org/rog/the-rules-of-golf/rule-23), and the handicap
  default, rounding and nine-hole distinction against
  [NGF Handicapreglene 2024](https://www.golfforbundet.no/files/documents/handicapreglene-whs-2024.pdf)
  and [R&A Appendix C](https://www.randa.org/en/roh/appendices/appendix-c).
- Inspected the current format policy, snapshots, score authority, team
  attribution, confirmation and database boundaries without changing source.
- Independently checked the documented examples with exact-fraction arithmetic:
  different gross/net winners, equal winners, one/both missing partner scores,
  plus and disabled handicaps, allowance rounding and boundaries, mixed-format
  best-N and a final round outside best-N. The three-hole illustration totals
  gross 14 (+2) and net 11 (−1); separate complete-round examples cover qualification
  and changing partners.
- Read-only contract review found no material rule, ownership, authority,
  visibility or compatibility issues. Documentation diff checks passed.
- Backend, database, frontend and browser test ladders were not run: this step
  changes no production source, schema or user interface. Future implementation
  acceptance includes the full affected ladders and real mobile/desktop browsers.

**READY FOR LATER IMPLEMENTATION PLANNING:** the definition is complete, with no
pending product choice. This is not an implementation or deployment verdict.

The initial variant excludes nine-hole play, per-player tees, larger teams,
four-ball Stableford/match play, automatic pairing and formal disqualification or
withdrawal adjudication. A side without a valid result on every hole cannot be
confirmed/completed through the initial workflow. These limits are explicit in
the contract.

A separate existing limitation was found while tracing nine-hole handling: the
current handicap helper has no round-length parameter, and its generic allocator
does not establish NGF's full-18-card/subset policy. No existing round arithmetic
or historical data was changed. Nine-hole four-ball requires its own preserved
layout and handicap contract before support can be claimed.
