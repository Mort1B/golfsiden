# Prepare players now and let them claim their accounts

Tournament admins can open **Administrasjon → Deltakere**, enter a name and
handicap, and share a personal account link. The player can be assigned to rounds
before registering. The recipient chooses a username and password; the account
links to the exact prepared player, preserving teams, scores and handicap facts.
No placeholder login or duplicate entrant is created.

Links expire after seven days and work once. Admins can replace or revoke them;
claimed accounts cannot receive another claim link. Active prepared entrants can
still claim after tournament completion/archival. This narrow exception does not
reopen ordinary registration. Links stay in temporary UI state, use URL fragments
and are removed from the address immediately. The server stores only token hashes.

**Fjern fra turneringen** withdraws a player with confirmation. It preserves
historical results, accounts and existing access/roles, including scorer access.
It requires removing draft assignments first and locking participating rounds.
Self/admin targets and completed/archived tournaments are protected. The database
records the responsible administrator and invalidates outstanding claim links.

The implementation uses a dedicated claim boundary and forward migration 0034.
Claiming, link replacement and withdrawal serialize on the same identity. Final
expiry checks cover database waits, username conflicts roll back the whole claim,
and contended withdrawal/account locks return a retryable conflict. Frontend
callbacks are fenced by mounted lifetime and canonical account/session identity,
including sign-out while a newer session appears. Unknown creation delivery asks
the admin to check the refreshed roster before retrying.

Example: create Kari with handicap 14.4, place her in the draft round, and send her
personal link. When Kari registers, her login owns the same player already on the
roster. If she drops out before play, remove her draft assignments and confirm
withdrawal; the record remains labelled **Trukket**.

Validation passes 641 Rust/database tests covering authority, concurrency and
history, plus 942 frontend tests, typecheck/lint/build, and three Chrome scenarios at mobile
and desktop widths. The seeded schema-33 upgrade to 34 preserves existing players;
fresh migrations and repeat seeding succeed. Independent source review has no
remaining actionable findings. Full results and limits are in the
[validation report](validation/player-claims-2026-10-08/README.md).

Deploy matching API/frontend with schema 34 and refreshed runtime grants. No
email service or new environment setting is needed. This step does not send
messages, merge accounts, revoke existing tournament access, restore withdrawals,
or support withdrawal during live rounds. Production hosting remains user-owned.
