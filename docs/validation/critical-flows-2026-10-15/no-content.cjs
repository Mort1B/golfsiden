// Diagnostic only: idempotent entry calls from a page with no tournament stream.
const { fs, path, state, open, assert } = require('./support.cjs');
(async () => {
  const f = await open();
  try {
    await f.authenticate(); const page = f.member;
    await page.goto('/profile');
    const endpoint = `/api/tournaments/${f.tid}/fantasy/entry`, events = [], pending = [];
    page.on('response', response => { if (new URL(response.url()).pathname === endpoint) events.push({ event: 'response', status: response.status() }); });
    page.on('requestfailed', request => {
      if (new URL(request.url()).pathname !== endpoint) return;
      pending.push((async () => events.push({ event: 'failed', error: request.failure()?.errorText, acknowledged_status: (await request.response())?.status() }))());
    });
    const statuses = [];
    for (let i = 0; i < 3; i++) statuses.push(await page.evaluate(async ({ endpoint, csrf }) => (await fetch(endpoint, {
      method: 'POST', credentials: 'include', headers: { 'x-csrf-token': csrf }, cache: 'no-store',
    })).status, { endpoint, csrf: f.memberAuth.csrf_token }));
    await page.waitForTimeout(1000); await Promise.all(pending);
    assert.deepEqual(statuses, [204, 204, 204]);
    assert.equal(events.filter(event => event.event === 'response' && event.status === 204).length, 3);
    assert.equal(events.filter(event => event.event === 'failed' && event.error === 'net::ERR_ABORTED' && event.acknowledged_status === 204).length, 3);
    const mode = process.argv[4]; assert(['baseline', 'default-flush'].includes(mode));
    fs.writeFileSync(path.join(state.directory, 'no-content-' + mode + '.json'), JSON.stringify({ statuses, events }), { mode: 0o600 });
    console.log(JSON.stringify({ mode, statuses, events }));
  } finally { await f.close(); }
})().catch(() => { process.exitCode = 1; });
