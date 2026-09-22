# Match history still pays for full round payloads

**Completed — READY WITH KNOWN LIMITATIONS.** The investigation confirms a
remaining payload/decoder cost after the lifecycle repairs. Production code and
contracts are unchanged at `d1e3d32`. The
[report and evidence](performance/history-payload/README.md) support a separately
approved player-filtered full-card read as the next bounded step.

In a populated synthetic three-round tournament with 24 matches per round, player
history displays three cards but transfers and strictly decodes all 72. The current
list payload totals 504,591 raw bytes or 21,491 gzip bytes. An offline subset using
the same full-card format is 21,267 raw bytes or 2,634 gzip bytes: **87.7% less gzip
body data**. Under 4x browser CPU throttling, warmed strict decoding has a median
of **7.57ms for the full collection versus 0.30ms for the subset**. Full JSON parsing
is measured separately at 2.95ms. These are potential byte/decoder savings, not
an implemented endpoint or demonstrated production latency improvement.

All-results and player-history routes currently transfer identical listings.
History already reduces rendered output: the populated match page has 45 DOM
descendants versus 832 for all results. Browser timings include network, parsing,
decoding, query notification and rendering; the report labels post-response tails
rather than calling them pure rendering time. Reducing cards after decoding does
not remove the transfer/decoder work.

The safest proposed slice preserves full-card coherence validation. Holes, notes
and event evidence verify the displayed result even when the list does not show
those details. A compact summary would need a separate validation contract.
Player-selected reads should instead use a distinct account+round+player cache
identity and leave unfiltered listings, management, assignment responses, table,
detail, scoring and recovery unchanged. Reusing the unfiltered key for a partial
list could give management incomplete pairings. Backend parity and current
membership/writable/final-visibility rules remain mandatory.

This introduces a navigation tradeoff: all-results and history currently can reuse
the same fresh full-list cache; a separate filtered key can need another request.
The next step must preserve malformed/noncanonical filter behavior, empty player
results, readiness gates, account isolation, cancellation, denials, freshness and
required live refreshes. It needs disposable PostgreSQL validation before being
called complete. Authorization-query optimization remains separate.

## Validation and limits

- Fresh production build, including TypeScript, passed. All **55 asset hashes**
  match the previous build and measured browser assets; production source is clean.
- **54 browser measurement cases** passed: four collection scenarios, all/direct
  filtered/delegated history routes, three repetitions, with the populated case at
  320/390/1280px. The measured refreshes completed **144 list and 54 table reads**.
  Browser raw/gzip body counts match the fixture server. No unexpected errors or
  horizontal overflow occurred.
- Four isolated real-decoder pairs passed exact selected-card/writable-ID parity,
  including the restricted final. Seven alternating batches of 20 iterations follow
  warmup; individual timings and ranges are retained.
- **94 focused tests across seven files passed**, covering decoders, cancellation,
  shared reads, private denials, live ownership and readiness/privacy/freshness.
- Independent read-only source/contract/harness review, artifact audit, script
  syntax, local links and diff checks passed. Phone/desktop screenshots and a
  supplemental restricted-final bottom-of-page pass were inspected.

No production repair was made. Full frontend unit/lint and backend/PostgreSQL
ladders were not repeated for this standalone documentation/measurement step.
Synthetic native HTTP/SSE, warmed probes and desktop viewport emulation establish
neither production speedup nor server/database authorization, latency, physical
phone or BFCache behavior. The table remains a full tournament read. The broader
database and security work remains queued.
