const crypto = require('node:crypto');
const { assert, call, expect, shot } = require('./support.cjs');

module.exports = async function entry(f) {
  const index = process.argv[4] === '3' ? 3 : 2;
  const claim = await f.mutate(`/api/tournaments/${f.tid}/players/${f.players[index]}/claim`, {}, 'POST', 201);
  const page = await f.context(390), username = 'critical_claim_' + crypto.randomBytes(5).toString('hex');
  await page.goto(`/claim/${claim.claim_id}#token=${claim.token}`);
  await page.getByLabel('Brukernavn', { exact: true }).fill(username);
  await page.getByLabel('Passord', { exact: true }).fill(f.password);
  await page.getByRole('button', { name: 'Ta i bruk kontoen', exact: true }).click();
  await expect(page.getByText(/Kontoen er klar/)).toBeVisible();
  const auth = await call(page.request, '/api/auth/session');
  assert.equal(auth.player_id, f.players[index]); assert.equal(new URL(page.url()).hash, '');
  f.extraManagerIds = [...new Set([...(f.extraManagerIds || (f.extraManagerId ? [f.extraManagerId] : [])), auth.user_id])]; f.persist();
  const base = `/api/tournaments/${f.tid}/fantasy`;
  const targetRound = f.rounds[index === 3 ? 1 : 0];
  let entries = 0;
  f.acknowledgedVoid.add(base + '/entry');
  page.on('request', request => { if (new URL(request.url()).pathname === base + '/entry' && request.method() === 'POST') entries++; });
  await page.goto(`/tournaments/${f.tid}/fantasy`);
  if (index === 3) await page.getByLabel('Runde for Min firer').selectOption(targetRound.id);
  await page.getByRole('button', { name: 'Meld meg på Fantasy', exact: true }).click();
  const first = page.getByRole('checkbox', { name: f.names[0], exact: true });
  try { await expect(first).toBeEnabled({ timeout: 10000 }); }
  catch {
    // Only the documented acknowledged-write/read-failure state can use this
    // recovery branch. An unexplained disabled editor remains a failing test.
    await expect(page.getByText(/Handlingen er bekreftet, men visningen kunne ikke oppdateres/)).toBeVisible();
    assert.equal(entries, 1);
    await page.getByRole('button', { name: 'Oppdater Fantasy', exact: true }).click();
    await expect(first).toBeEnabled();
    f.pass('fresh_entry_acknowledged_read_failure_recovers_without_second_entry_write');
  }
  for (const name of f.names) await page.getByRole('checkbox', { name, exact: true }).check();
  await page.getByLabel('Kaptein · doble poeng').selectOption(f.players[index]);
  await page.getByRole('button', { name: 'Lagre firer og kaptein', exact: true }).click();
  await expect(page.getByText('Lagret lag · revisjon 1')).toBeVisible();
  assert.equal(entries, 1);
  f.acknowledgedVoid.delete(base + '/entry');
  const own = await call(page.request, `${base}/rounds/${targetRound.id}`);
  assert.deepEqual(own.selections.find(value => value.user_id === auth.user_id).receipt.picks.sort(), [...f.players].sort());
  await shot(page, 'claimed-player-fantasy', 390);
  f.pass('prepared_player_claim_first_fantasy_entry_and_selection_ui_one_entry_write');
};
