# Latest iteration

RELEASE-REFREEZE replaces the outdated frozen handoff with exact accepted images
from committed recovery source `baeeda9d51fd730bffb32892bbcbc25a79a24079`.
Award-denial abandonment and Fantasy four-ball lock readiness now exist in both
the accepted source and delivered API/web. No application, Dockerfile, schema,
dependency, runtime-setting or backup-script change was made in this step.

The common tag is `release-20261016-35a7165802fc`. The
[release report](validation/release-refreeze-2026-10-16/README.md),
[manifest](validation/release-refreeze-2026-10-16/manifest.json),
[bundle receipt](validation/release-refreeze-2026-10-16/bundle.json) and
[operator handoff](validation/release-refreeze-2026-10-16/OPERATOR.md) identify the
replacement. The historical bundle and image identities are preserved. The
protected loader checks archives/config IDs/runtime files, protects existing
aliases, binds all five Compose services and validates private project/role
configuration. Startup uses no build or pull.

Fresh acceptance used real Chrome through HTTPS/Caddy and the restricted runtime
role: 23 packaged guards, 56 source/16 target checks, 7 separate boundary-browser
checks, five critical phases and 3 committed regressions at 320/390/1280. One case
removes admin access during an award request and safely leaves without another
mutation; another denies replay after a committed original and preserves its
history. Four-ball sporting confirmation accepts the blank partner, the lock
workflow guards it, and deliberate inputs plus renewed confirmation allow lock.
The actual backup/restore scripts preserve 61 tables, 1122 synthetic rows and
sequence state. Restored receipt replay, stream restart, outage/reconnect and
proxy-error redaction pass. Ten transfer controls pass. Owned local fixtures,
private configuration, browser profiles and temporary aliases are removed.

All three runtime package sets and the Caddy binary equal the earlier freeze.
The API binary/web assets changed; schema remains 39, PG17.11 and
Caddy2.11.7/Go1.27.2/x/net0.60.0/quic-go0.63.0 remain. No fresh advisory scan ran.
Native Docker Engine, public host/DNS/ACME/ports, actual playing phones and
protected off-host recovery remain operator gates; no hosted deployment occurred.
The prior source full PostgreSQL run still has its unrelated native-collation
assertion failure and unreached later assertions. That ladder was not rerun or
repaired by this delivery step. Existing P3 Fantasy read-retry and earlier
lifecycle-test limitations remain documented. No adjacent maintenance is selected.
