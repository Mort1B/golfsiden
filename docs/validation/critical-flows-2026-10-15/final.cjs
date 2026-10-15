const crypto = require('node:crypto');
const { assert, call, expect, shot, transition } = require('./support.cjs');

module.exports = async function final(f) {
  const page = f.organizer, member = f.member, round = f.rounds[1], rid = round.id;
  const base = `/api/tournaments/${f.tid}/fantasy`, roundPath = `${base}/rounds/${rid}`;
  const manage = `/manage/tournaments/${f.tid}?round=${rid}#lifecycle`;
  const visibility = page.getByRole('switch', { name: 'Frigi hull 10–18', exact: true });
  async function setReleased(released) {
    await expect(visibility).toBeEnabled();
    if (await visibility.isChecked() !== released) await visibility.click();
    await expect.poll(async () => (await call(page.request, `/api/tournaments/${f.tid}/final-round-visibility`)).back_nine_hidden).toBe(!released);
    await expect(visibility).toBeChecked({ checked: released });
  }
  if (!f.checks.final_open_materializes_member_carry_forward_and_rejects_new_selection) {
  const before = await call(member.request, roundPath);
  assert(!before.selections.some(selection => selection.user_id === f.memberAuth.user_id && selection.receipt));
  assert.equal(before.carry_forward_eligible, true);
  await page.goto(manage); await transition(page, 'Åpne runden');
  await expect(page.getByText('Runden er åpnet.', { exact: true })).toBeVisible();
  const locked = await call(member.request, roundPath);
  const selection = locked.selections.find(value => value.user_id === f.memberAuth.user_id);
  assert.equal(selection.state, 'locked'); assert.equal(selection.receipt.origin, 'carried_forward');
  assert.equal(selection.receipt.source_round, f.rounds[0].id); assert.equal(selection.receipt.captain, f.players[1]);
  assert.deepEqual([...selection.receipt.picks].sort(), [...f.players].sort());
  const denied = await f.memberMutate(`${roundPath}/lineup`, { request_id: crypto.randomUUID(), expected_revision: selection.receipt.revision, picks: f.players, captain: f.players[0] }, 'PUT', 409);
  assert.equal(denied.error.code, 'fantasy_closed');
  f.pass('final_open_materializes_member_carry_forward_and_rejects_new_selection');
  } else await page.goto(manage);
  await setReleased(false);

  const owner = { type: 'player', id: f.players[0] }, cardPath = `/api/rounds/${rid}/scorecards/player/${owner.id}`;
  const card = await call(page.request, cardPath + '/scoring');
  const save = (hole, gross_strokes) => f.mutate(`/api/rounds/${rid}/scores`, { owner, hole_id: card.holes[hole - 1].hole_id, gross_strokes }, 'PUT');
  await save(1, 4); await save(18, 3);
  async function checkHidden() {
    const read = await call(member.request, cardPath);
    // The frontend decoder adds its `projection` discriminant; it is not a wire field.
    assert.equal(read.visibility.mode, 'front_nine'); assert.equal(read.visible_hole_count, 9); assert.equal(read.holes.length, 9);
    assert.equal(read.complete, null); assert.equal(read.confirmed, null); assert(!('confirmed_by' in read));
    const results = await call(member.request, `${roundPath}/results`);
    assert.deepEqual(results.golfers.map(value => value.player_id).sort(), [...f.players].sort());
    assert.deepEqual(results.managers.map(value => value.user_id).sort(), [f.auth.user_id, f.memberAuth.user_id, ...(f.extraManagerIds || (f.extraManagerId ? [f.extraManagerId] : []))].sort());
    assert.equal(results.round.visibility.mode, 'front_nine'); assert.equal(results.round.sporting_status, null);
    assert.equal(results.round.points.state, 'withheld');
    for (const entry of [...results.golfers, ...results.managers]) { assert.equal(entry.points.state, 'withheld'); assert.equal(entry.rank, null); }
    const golfer = results.golfers.find(value => value.player_id === owner.id);
    assert.equal(golfer.holes.length, 18); assert.equal(golfer.holes.slice(9).length, 9);
    assert(golfer.holes.slice(9).every(hole => hole.net_strokes === null && hole.category === null && hole.points.state === 'withheld'));
    return results;
  }
  await checkHidden();
  await call(member.request, `/api/tournaments/${f.tid}/final-round-visibility`, undefined, 'GET', 403);
  await f.memberMutate(`/api/tournaments/${f.tid}/final-round-visibility`, { back_nine_hidden: false, expected_visibility_updated_at: round.updated_at }, 'PATCH', 403);
  await member.goto(`/tournaments/${f.tid}/fantasy`);
  async function showGolfer() {
    await member.getByRole('button', { name: 'Poengtavler', exact: true }).click();
    await member.getByRole('button', { name: 'Spillerpoeng', exact: true }).click();
    await member.locator('.fantasy-standings button').filter({ hasText: f.names[0] }).click();
  }
  await showGolfer();
  const last = member.locator('.fantasy-holes li').filter({ hasText: 'Hull 18 · par 4' }).last();
  await expect(last).toContainText('Skjult'); await expect(last).not.toContainText('netto');
  const hiddenText = await last.innerText();
  await save(18, 10);
  await checkHidden();
  await expect(last).toHaveText(hiddenText, { useInnerText: true });
  await shot(member, 'final-hidden', 320);
  await setReleased(true);
  await expect.poll(async () => (await call(member.request, `${roundPath}/results`)).round.visibility.mode).toBe('full');
  await showGolfer(); await expect(last).toContainText('netto 10');
  await shot(member, 'final-released', 390);
  await setReleased(false);
  await expect.poll(async () => (await call(member.request, `${roundPath}/results`)).round.visibility.mode).toBe('front_nine');
  await checkHidden(); await showGolfer(); await expect(last).toContainText('Skjult');
  await member.evaluate(() => window.dispatchEvent(new PageTransitionEvent('pageshow', { persisted: true })));
  await expect(last).toContainText('Skjult'); await expect(last).not.toContainText('netto');
  f.pass('player_final_privacy_withholds_numeric_and_fantasy_releases_and_rehides_on_return');

  for (const player of f.players) {
    const current = await call(page.request, `/api/rounds/${rid}/scorecards/player/${player}/scoring`);
    for (const hole of current.holes) if (hole.score?.gross_strokes !== 4) await f.mutate(`/api/rounds/${rid}/scores`, { owner: { type: 'player', id: player }, hole_id: hole.hole_id, gross_strokes: 4 }, 'PUT');
    await f.mutate(`/api/rounds/${rid}/scorecards/player/${player}/confirm`);
  }
  await transition(page, 'Fullfør runden');
  await expect(page.getByText('Runden er fullført.', { exact: true })).toBeVisible();
  for (const player of f.players) f.expected.push({ path: `/api/rounds/${rid}/scorecards/player/${player}/scoring`, method: 'GET', status: 403, code: 'forbidden' });
  await transition(page, 'Lås runden');
  await expect(page.getByText('Runden er låst.', { exact: true })).toBeVisible();
  await setReleased(true);
  await expect.poll(async () => (await call(member.request, `${roundPath}/results`)).round.visibility.mode).toBe('full');
  const totals = await call(member.request, `${base}/results`);
  assert.equal(totals.rules_version, 2);
  assert.deepEqual(totals.managers.find(value => value.id === f.auth.user_id).points, { state: 'settled', total: 88 });
  assert.deepEqual(totals.managers.find(value => value.id === f.memberAuth.user_id).points, { state: 'settled', total: 94 });
  const roundResults = await call(member.request, `${roundPath}/results`);
  assert.deepEqual(roundResults.golfers.map(value => value.player_id).sort(), [...f.players].sort());
  assert.deepEqual(roundResults.managers.map(value => value.user_id).sort(), [f.auth.user_id, f.memberAuth.user_id, ...(f.extraManagerIds || (f.extraManagerId ? [f.extraManagerId] : []))].sort());
  assert(roundResults.golfers.every(value => value.points.state === 'settled' && value.points.total === 10));
  assert(roundResults.managers.every(value => value.points.state === 'settled' && value.points.total === 50));
  await member.goto(`/tournaments/${f.tid}/fantasy`);
  await member.getByRole('button', { name: 'Poengtavler', exact: true }).click();
  await expect(member.locator('.fantasy-standings > li')).toHaveCount(2 + (f.extraManagerIds || (f.extraManagerId ? [f.extraManagerId] : [])).length);
  await expect(member.locator('.fantasy-standings button').filter({ hasText: f.names[1] })).toContainText('94 p · avgjort');
  await member.locator('.fantasy-standings button').filter({ hasText: f.names[1] }).click();
  await expect(member.locator('.fantasy-contributions li')).toHaveCount(8);
  await expect(member.getByText('Gjenbrukt fra tidligere låst runde', { exact: true })).toBeVisible();
  await shot(member, 'fantasy-settled-managers', 1280);
  await member.getByRole('button', { name: 'Spillerpoeng', exact: true }).click();
  await expect(member.locator('.fantasy-standings > li')).toHaveCount(4);
  await shot(member, 'fantasy-settled-golfers', 390);
  f.pass('two_round_fantasy_boards_settle_exact_88_and_94_with_captain_and_carry_forward');
  const outsider = await f.context();
  const { login } = require('./support.cjs');
  await login(outsider, f.outsiderName, f.password);
  for (const endpoint of [base, `${base}/results`, `${roundPath}/results`, `${base}/results/managers/${f.auth.user_id}`]) await call(outsider.request, endpoint, undefined, 'GET', 403);
  f.acknowledgedVoid.add('/api/auth/logout');
  await member.getByRole('button', { name: 'Logg ut', exact: true }).click();
  await expect(member.locator('.fantasy-page')).toHaveCount(0);
  assert.equal((await member.request.get(`${base}/results`)).status(), 401);
  f.pass('outsider_fantasy_denied_and_logout_clears_private_boards');
};
