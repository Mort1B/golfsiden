# Fantasy browser acceptance repair

R1 from the pre-deployment assessment is fixed. The preset browser test now
classifies expected read errors by endpoint, method, status and decoded error
code. It does not reject a verified retryable projection conflict merely because
the case also injects a 503. Unread bodies require actual cancellation evidence;
unexpected errors remain failures.

Both affected boards must recover. A separate deterministic scenario injects
conflicts after initial reads settle and requires readable delivery on both
endpoints before accepting their recovery. Production retries, scores, queries
and write behavior are unchanged.

Validation passed: 1,102 frontend tests, typecheck, lint, strict browser TypeScript,
production build, 20 repeated recovery/conflict Chrome cases and two retained
copy/lost-response cases. Independent review found no remaining issues.
[Evidence and commands](validation/predeployment-fixes-2026-10-14/README.md).

The user's remaining authorized fixes are R2 (duplicate frontend refresh reads)
and R3 (unnecessary backend projection). They proceed as separate steps. Physical
iPhone Chrome/Safari and hosting acceptance remain after deployment.
