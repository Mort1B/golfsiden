const { assert, call, expect, shot } = require('./support.cjs');

module.exports = async function fantasy(f) {
  const page = f.organizer, member = f.member, base = `/api/tournaments/${f.tid}/fantasy`;
  if (!f.checks.fantasy_rules2_two_round_presets_exact_lost_response_replay_same_receipt) {
  await page.goto(`/tournaments/${f.tid}/fantasy`);
  await expect(page.getByText('Fantasy er ikke aktivert for denne turneringen.')).toBeVisible();
  await page.getByRole('button', { name: 'Aktiver Fantasy', exact: true }).click();
  await expect.poll(async () => (await call(page.request, base)).rules_version).toBe(2);
  await page.getByRole('button', { name: 'Poengtavler', exact: true }).click();
  await expect(page.getByText('Ingen Fantasy-lag er påmeldt.')).toBeVisible();
  await shot(page, 'fantasy-empty', 320);
  await page.getByRole('button', { name: 'Min firer', exact: true }).click();
  f.acknowledgedVoid.add(base + '/entry');
  await page.getByRole('button', { name: 'Meld meg på Fantasy', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Planlegg alle runder', exact: true })).toBeVisible();
  for (const name of f.names) await page.getByRole('checkbox', { name, exact: true }).check();
  await page.getByLabel('Kaptein · doble poeng').selectOption(f.players[0]);
  await page.getByRole('button', { name: 'Kopier fireren til andre åpne runder' }).click();
  await page.getByRole('button', { name: 'Forbered valgte runder' }).click();
  const requests = [], receipts = []; let lose = true;
  const lineupPath = `${base}/rounds/${f.rounds[1].id}/lineup`;
  f.allowLost.add(lineupPath);
  await page.route('**/fantasy/rounds/*/lineup', async route => {
    requests.push({ path: new URL(route.request().url()).pathname, body: route.request().postDataJSON() });
    if (new URL(route.request().url()).pathname === lineupPath && lose) {
      lose = false; const accepted = await route.fetch(); assert.equal(accepted.status(), 200);
      receipts.push(await accepted.json()); await route.abort('connectionfailed');
    } else {
      const accepted = await route.fetch(); assert.equal(accepted.status(), 200);
      receipts.push(await accepted.json()); await route.fulfill({ response: accepted });
    }
  });
  await page.getByRole('button', { name: 'Lagre gjennomgåtte utkast (2)' }).click();
  await expect(page.getByText(/1 av 2 innsendinger bekreftet/)).toContainText('Lagringen er satt på pause');
  assert.equal(requests.length, 2);
  await page.getByLabel('Runde for Min firer').selectOption(f.rounds[1].id);
  await expect(page.getByText(/Lagringen er ikke bekreftet/)).toBeVisible();
  await shot(page, 'fantasy-uncertain', 390);
  await page.getByRole('button', { name: 'Avklar samme innsending', exact: true }).click();
  await expect(page.getByText('Handlingen er bekreftet. Visningen er oppdatert.')).toBeVisible();
  assert.equal(requests.length, 3); assert.deepEqual(requests[2], requests[1]); assert.deepEqual(receipts[2], receipts[1]);
  await page.unroute('**/fantasy/rounds/*/lineup');
  f.allowLost.delete(lineupPath);
  for (const round of f.rounds) {
    const own = await call(page.request, `${base}/rounds/${round.id}`);
    const selection = own.selections.find(value => value.user_id === f.auth.user_id);
    assert.equal(selection.receipt.revision, 1); assert.equal(selection.receipt.captain, f.players[0]);
  }
  f.pass('fantasy_rules2_two_round_presets_exact_lost_response_replay_same_receipt');
  }

  await member.goto(`/tournaments/${f.tid}/fantasy`);
  await expect(member.getByRole('heading', { name: 'Fantasy-oppsett', exact: true })).toHaveCount(0);
  const ownView = await call(member.request, `${base}/rounds/${f.rounds[0].id}`);
  f.acknowledgedVoid.add(base + '/entry');
  if (!ownView.entered) await member.getByRole('button', { name: 'Meld meg på Fantasy', exact: true }).click();
  if (!ownView.selections.some(value => value.user_id === f.memberAuth.user_id && value.receipt)) {
  const first = member.getByRole('checkbox', { name: f.names[0], exact: true });
  try { await expect(first).toBeEnabled(); }
  catch {
    await expect(member.getByText(/Handlingen er bekreftet, men visningen kunne ikke oppdateres/)).toBeVisible();
    await member.getByRole('button', { name: 'Oppdater Fantasy', exact: true }).click();
    await expect(first).toBeEnabled();
    f.pass('member_acknowledged_entry_read_retry_restores_editing');
  }
  for (const name of f.names) await member.getByRole('checkbox', { name, exact: true }).check();
  await member.getByLabel('Kaptein · doble poeng').selectOption(f.players[1]);
  await member.getByRole('button', { name: 'Lagre firer og kaptein', exact: true }).click();
  await expect(member.getByText('Lagret lag · revisjon 1')).toBeVisible();
  }
  f.acknowledgedVoid.delete(base + '/entry');
  // Leave member round 2 unsaved so actual carry-forward can be verified later.
  for (const [client, auth, other] of [[page.request, f.auth, f.memberAuth], [member.request, f.memberAuth, f.auth]]) {
    const view = await call(client, `${base}/rounds/${f.rounds[0].id}`);
    assert.equal(view.selection_availability, 'open');
    assert(view.selections.some(value => value.user_id === auth.user_id));
    assert(!view.selections.some(value => value.user_id === other.user_id));
    const results = await call(client, `${base}/rounds/${f.rounds[0].id}/results`);
    const otherManager = results.managers.find(value => value.user_id === other.user_id);
    assert(otherManager && otherManager.lineup === null); assert.deepEqual(otherManager.contributions, []);
  }
  await member.getByRole('button', { name: 'Poengtavler', exact: true }).click();
  await member.locator('.fantasy-standings button').filter({ hasText: f.names[0] }).click();
  await expect(member.getByText('Andre deltakeres valg er private fram til fristen.').first()).toBeVisible();
  await shot(member, 'fantasy-private-draft', 320);
  const privateRead = await member.request.get(`${base}/rounds/${f.rounds[0].id}`);
  assert(privateRead.headers()['cache-control'].includes('no-store'));
  f.pass('two_manager_draft_privacy_including_organizer_and_member_round2_unsaved');

  let release; const held = new Promise(resolve => { release = resolve; });
  const roundPath = `${base}/rounds/${f.rounds[0].id}`;
  await member.route(`**${roundPath}`, async route => {
    await held;
    if (route.request().failure()?.errorText === 'net::ERR_ABORTED') return;
    try { await route.continue(); }
    catch (error) { if (route.request().failure()?.errorText !== 'net::ERR_ABORTED') f.errors.push('held request continuation'); }
  });
  try {
    await member.reload();
    await expect(member.getByText('Henter runden …', { exact: true }).first()).toBeVisible();
    await shot(member, 'fantasy-loading', 390);
  } finally { release(); await member.unrouteAll({ behavior: 'wait' }); }
  await expect(member.getByRole('heading', { name: 'Planlegg alle runder', exact: true })).toBeVisible();
  await member.getByRole('button', { name: 'Poengtavler', exact: true }).click();
  await expect(member.locator('.fantasy-standings > li')).toHaveCount(2);
  const resultsPath = `${base}/results`;
  f.expected.push({ path: resultsPath, method: 'GET', status: 503, code: 'unavailable' });
  await member.route(`**${resultsPath}`, route => route.fulfill({ status: 503, json: { error: { code: 'unavailable', message: 'Midlertidig utilgjengelig i kontrollert prøve' } } }));
  await member.getByRole('button', { name: 'Oppdater Fantasy', exact: true }).click();
  await expect(member.getByText('Midlertidig utilgjengelig i kontrollert prøve').first()).toBeVisible();
  await shot(member, 'fantasy-read-error', 390);
  await member.unroute(`**${resultsPath}`);
  await member.getByRole('button', { name: 'Prøv igjen', exact: true }).first().click();
  await member.getByRole('button', { name: 'Poengtavler', exact: true }).click();
  await expect(member.locator('.fantasy-standings > li')).toHaveCount(2);
  await shot(member, 'fantasy-recovered', 1280);
  f.pass('fantasy_loading_read_failure_and_explicit_read_retry_recover_boards');
};
