# Fantasy lineups for every round

The Fantasy lineup workspace now lets a manager prepare every existing tournament
round in advance, including team rounds whose sporting teams have not yet been
assigned. **Planlegg alle runder** shows each round's picks, captain, deadline and
status. The selected round's four can be copied through an explicit preview;
existing unsaved destination choices require consent to replace. Copying creates
independent local drafts. A newly created round remains unprepared.

For example, prepare one four for three rounds, change round two's captain, then
review and save all three. Saves run in tournament order with a separate request,
revision and receipt for each round. The batch checks fresh authorization,
eligibility and revision before each dispatch. Draft edits and other Fantasy
writes are guarded until the sequence stops.

If round two's response is lost, round one remains confirmed, round two retains
its exact request for reconciliation, and round three stays unsent. After
resolving round two, deliberately save the remaining draft; round one is never
sent again. A failed read after acknowledgement also pauses the batch but
preserves the receipt and offers read-only retry. Disconnect/privacy clearing,
including an initial live connection opening, pauses further saves. Local
recovery remains scoped to tournament, account and session, while private server
projections are erased as before. Closed drafts can be discarded per round;
stale unsent drafts require explicit review and rebase before a new save.

The implementation reuses the existing API, immutable receipts and carry-forward
rules. There is no backend or schema change. Optional closest-to-pin/longest-drive
awards and the revised Fantasy scoring scale remain separate candidates in
[PLANS.md](PLANS.md).

Validation passed: 138 frontend test files / 1,052 tests, typecheck, lint, build,
strict standalone browser-test TypeScript, three new Chrome scenarios and 14
retained browser regressions. Rendered states cover 320/390/1280 pixels, long
names, keyboard copy-preview focus, loading/error recovery, closed destinations,
partial success, exact replay and second-account privacy. A read-only independent
review found no correctness or privacy defects. See the
[validation report](validation/fantasy-presets-2026-10-14/README.md) for commands,
coverage, test repairs and limits. Hosted deployment remains operator-owned.
