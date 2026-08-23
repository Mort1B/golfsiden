# Latest explanation

## Immutable local course revisions

Manual course entry and GolfCourseAPI imports now share one persistence model.
The existing `courses`, `tees`, and `holes` UUID hierarchy remains authoritative,
so rounds, scorecards, lifecycle validation, and leaderboards will not need a
second set of revision identifiers.

A pure domain validator accepts source provenance, one course and selected tee,
rating and slope, plus 1-36 ordered holes. Every hole requires par and a unique
stroke index forming the complete `1..=hole_count` permutation; distance is
optional but must be positive when supplied. Provider revisions require an
opaque provider course ID, manual revisions forbid one, and no provider tee ID
is invented.

Persistence is deliberately transaction-composable. The repository inserts the
course, its one selected tee, and every hole into a transaction owned by its
caller, then finalizes the course with database time. The next round-
configuration workflow can therefore authorize, lock a draft round, create the
revision, and attach it without committing an orphan on failure.

PostgreSQL treats non-null source provenance as finalization. A deferred trigger
requires exactly one complete tee and exact hole/stroke-index ranges before the
transaction can commit. Finalized course, tee, and hole rows reject inserts,
updates, and deletes. Child writes lock ancestor course rows in UUID order, so a
concurrent change either completes before finalization and is validated, or
waits and is rejected afterward. Pre-migration rows retain null metadata as
explicit legacy data rather than receiving guessed provenance.

## Compact example

The repository leaves commit ownership with the future round transaction:

```rust
let revision = course_revisions::validate(command)?;
let stored = course_revisions::insert_in_transaction(&mut transaction, &revision).await?;
// The caller may now attach stored.course_id and stored.tee.tee_id before commit.
```

## Invariants

- Provider and manual facts use the same durable UUID graph.
- A finalized revision contains exactly one selected tee and complete ordered
  hole facts.
- Hole distance remains nullable; rating, slope, par, and stroke index do not.
- Finalized facts cannot drift, including under concurrent child writes.
- Legacy rows remain readable and retain null provenance.
- No HTTP route, provider request, round mutation, SSE event, frontend behavior,
  handicap snapshot, score ownership, or lifecycle transition changed.

## Validation

- Pure tests cover both provenance paths, normalization, invalid text/rating,
  duplicate stroke indexes, and invalid optional distance.
- Ten PostgreSQL tests cover exact provider/manual readback, nullable distance,
  rollback on child failure, incomplete and invalid provenance rejection,
  finalized hierarchy immutability, both finalization/mutation race orderings,
  legacy migration behavior, and clean plus upgraded idempotent seed behavior.
- The standard workspace suite passes 59 tests. The all-feature PostgreSQL suite
  passes 153 tests: 59 unit tests and 94 integration tests, including all ten
  focused course-revision cases.
- Formatting and all-target/all-feature Clippy with warnings denied pass.
- A fresh isolated PostgreSQL database migrated twice and seeded twice. It
  retained exactly one finalized manual course, one complete tee, 18 distinct
  hole numbers and stroke indexes, and five configured rounds; the disposable
  database was then removed.
- `git diff --check` passes. Frontend and browser checks are not applicable to
  this backend-only persistence step.

## Next boundary

Add the admin-only atomic draft-round configuration endpoint. It will choose
manual facts or one usable provider tee, validate and persist the revision in the
same locked transaction that attaches it to the draft round, then publish one
post-commit invalidation.
