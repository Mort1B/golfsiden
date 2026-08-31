# Latest explanation

## Completed best-N contributions

Tournament gross and net standings now consume the persisted
`counted_rounds` setting. For each registered player, the backend constructs one
historical contribution per completed or locked round. Individual cards retain
their player owner; scramble and foursomes cards retain their exact historical
team owner and are copied only to the frozen members of that round.

Selection is metric-specific. Gross and net each sort contributions by their own
score-to-par, followed by round number and UUID for a deterministic cutoff, and
mark the lowest N as counted. Every contribution remains in the response so later
history and scorecard work can present both selected and discarded rounds.

```rust
candidates.sort_by(|left, right| {
    selected_score(left, metric)
        .cmp(&selected_score(right, metric))
        .then(left.round_number.cmp(&right.round_number))
        .then(left.value.round_id.cmp(&right.value.round_id))
});
```

## Ranking and response contract

Players with fewer counted results rank behind players with more until they
reach N. After that, the sum of selected score-to-par determines position.
Competition ties compare only those sporting facts; display name and UUID make
the output stable without breaking a tie. Players with no result remain present
and unranked, while `eligible` becomes true only after N completed attributed
results.

The metric-specific response exposes the required N, selected aggregate totals,
eligibility/progress, and every round-ordered contribution with its tagged owner,
gross/net/par facts, score-to-par, and counted state. Open rounds retain their
current-team metadata but do not contribute yet. Their stored facts are still
fully validated so corrupted open data cannot silently produce authoritative-
looking tournament metadata.

The frontend shows the selected score-to-par aggregate and compact best-N
progress. Its runtime decoder verifies not just field types but the aggregate:
counts and flags, eligibility, unique included/current rounds, owner identity,
metric score-to-par, and selected total sums must all agree before the response
enters the user-scoped query cache.

## Review and validation

Owner review found and resolved the loss of fail-closed open-round validation,
missing attribution regressions, and several cross-field decoder gaps. Focused
coverage now includes metric independence, count-first provisional ranking,
stable cutoff ties, competition ties, zero results, duplicate player/round
rejection, corrupt open facts, and exact foursomes owner attribution.

The final ladder passed formatting, 82 standard Rust tests, 82 unit plus 140
PostgreSQL integration tests, fresh migration and two idempotent seed runs, 164
frontend tests, strict TypeScript, ESLint, the production build, diff checks, and
production file-size checks. Strict all-feature Clippy remains red on exactly
three `result_large_err` warnings in untouched round-lifecycle files; no best-N
file has a Clippy finding. That baseline repair is recorded as the next bounded
plan item instead of being folded into this scoring change.

Real Chrome covered zero, fewer than N, exactly N, more than N, gross ties,
independent net selection, and a 62-character name at 320px, 375px, 390px, and
1440px. All captures had eight rows, no page or row overflow, no visible control
below 44px, and visible keyboard focus. Gross selected the deterministic first
three of four tied rounds; net selected different round sets and ranked from -70
to -33. Both endpoints returned `private, no-store`, and real score mutations
caused payload-free SSE refetches. No post-login request, console, or browser
error occurred.

Correction-specific SSE was not separately exercised; real save, confirmation,
and completion mutations proved the same invalidation/refetch path. Current-team
metadata was covered by database/API tests but not a final browser capture because
the team rounds had been completed for the more-than-N fixture.

## Roadmap order

Completed best-N selection is isolated from the next Phase 7 boundary. Open-round
provisional contributions and holes-played progress come next; final-nine role
visibility, the 24-hour embargo, and navigable scorecard histories remain later
steps. Additional play modes remain deferred until roadmap completion,
optimization, and security review.
