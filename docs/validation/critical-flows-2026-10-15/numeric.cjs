const { assert, call, expect, queueRows, shot, transition } = require('./support.cjs');

module.exports = async function numeric(f) {
  const page = f.organizer, member = f.member, round = f.rounds[0], rid = round.id;
  const manage = `/manage/tournaments/${f.tid}?round=${rid}#lifecycle`;
  const adminPath = `/api/rounds/${rid}/scorecards/player/${f.players[0]}`;
  const adminUrl = `/score?${new URLSearchParams({ tournament: f.tid, round: rid, owner_type: 'player', owner: f.players[0], view: 'summary' })}`;
  const fantasyPath = `/api/tournaments/${f.tid}/fantasy/rounds/${rid}/results`;
  if (!f.checks.player_gross_and_net_result_views_use_authoritative_preserved_cards) {
  await page.goto(manage);
  await expect(page.getByRole('button', { name: 'Åpne runden', exact: true })).toBeDisabled();
  await page.getByRole('button', { name: 'Start turneringen', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Turneringen er startet', exact: true })).toBeVisible();
  await transition(page, 'Åpne runden');
  await expect(page.getByText('Runden er åpnet.', { exact: true })).toBeVisible();
  f.pass('organizer_starts_tournament_and_opens_prepared_round_in_ui');

  const owner = { type: 'player', id: f.players[1] }, cardPath = `/api/rounds/${rid}/scorecards/player/${owner.id}/scoring`;
  const scoreUrl = `/score?${new URLSearchParams({ tournament: f.tid, round: rid, owner_type: 'player', owner: owner.id, hole: '1', view: 'hole' })}`;
  const conditionalPath = `/api/rounds/${rid}/scores/conditional`;
  f.allowLost.add(conditionalPath);
  const requests = [], receipts = [];
  await member.route(`**${conditionalPath}`, async route => {
    requests.push(route.request().postDataJSON());
    const response = await route.fetch(); assert.equal(response.status(), 200); receipts.push(await response.json());
    if (requests.length === 1) await route.abort('failed'); else await route.fulfill({ response });
  });
  await member.goto(scoreUrl);
  await expect(member.getByRole('button', { name: /Registrer par/ })).toBeEnabled();
  await member.getByRole('button', { name: /Registrer par/ }).click();
  await expect(member.getByText('Lagret på serveren', { exact: true })).toBeVisible({ timeout: 35000 });
  await expect.poll(() => requests.length).toBe(2);
  assert.deepEqual(requests[1], requests[0]); assert.deepEqual(receipts[1], receipts[0]);
  assert.equal((await call(page.request, cardPath)).holes[0].score.gross_strokes, 4);
  await member.unroute(`**${conditionalPath}`);
  f.allowLost.delete(conditionalPath);
  f.pass('player_score_lost_response_retries_exact_request_and_acknowledgement');

  f.offlineWindow = true;
  await member.context().setOffline(true);
  await member.getByRole('button', { name: 'Legg til ett slag', exact: true }).click();
  await expect.poll(async () => (await queueRows(member)).length).toBe(1);
  const serialized = JSON.stringify(await queueRows(member));
  assert(!serialized.includes(f.password) && !serialized.includes(f.memberAuth.csrf_token));
  const card = await call(page.request, cardPath);
  await f.mutate(`/api/rounds/${rid}/scores`, { owner, hole_id: card.holes[0].hole_id, gross_strokes: 7 }, 'PUT');
  f.expected.push({ path: conditionalPath, method: 'PUT', status: 409, code: 'score_version_conflict' });
  await member.context().setOffline(false);
  await expect(member.locator('.pending-scores summary')).toContainText('krever valg', { timeout: 35000 });
  await member.locator('.pending-scores summary').click();
  await member.getByRole('button', { name: 'Sammenlign scorer', exact: true }).click();
  await expect(member.getByText('Score på serveren:')).toContainText('7 slag');
  await shot(member, 'score-conflict', 320);
  await member.getByRole('button', { name: 'Behold serverscoren', exact: true }).click();
  await expect.poll(async () => (await queueRows(member)).length).toBe(0);
  await expect(member.locator('.score-stepper output strong')).toHaveText('7');
  await expect(member.getByText('Lagret på serveren', { exact: true })).toBeVisible();
  assert.equal((await call(page.request, cardPath)).holes[0].score.gross_strokes, 7);
  f.offlineWindow = false;
  f.pass('offline_durable_edit_conflicts_without_overwrite_and_explicit_server_choice');
  await shot(member, 'score-recovered', 390);

  // Prepare a deterministic complete par round through real authorized writes.
  for (const player of f.players) {
    const own = { type: 'player', id: player };
    const current = await call(page.request, `/api/rounds/${rid}/scorecards/player/${player}/scoring`);
    for (const hole of current.holes) if (hole.score?.gross_strokes !== 4) await f.mutate(`/api/rounds/${rid}/scores`, { owner: own, hole_id: hole.hole_id, gross_strokes: 4 }, 'PUT');
    if (player !== f.players[0]) await f.mutate(`/api/rounds/${rid}/scorecards/player/${player}/confirm`);
  }
  await page.goto(adminUrl);
  await page.getByRole('button', { name: 'Bekreft fullført scorekort', exact: true }).click();
  await expect(page.getByText('Scorekortet er bekreftet', { exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Korriger score', exact: true })).toBeEnabled();
  await page.goto(manage); await transition(page, 'Fullfør runden');
  await expect(page.getByText('Runden er fullført.', { exact: true })).toBeVisible();
  const before = await call(page.request, fantasyPath);
  assert.deepEqual(before.golfers.map(value => value.player_id).sort(), [...f.players].sort());
  assert.deepEqual(before.managers.map(value => value.user_id).sort(), [f.auth.user_id, f.memberAuth.user_id, ...(f.extraManagerIds || (f.extraManagerId ? [f.extraManagerId] : []))].sort());
  assert(before.golfers.every(golfer => golfer.points.state === 'settled' && golfer.points.total === 10));
  assert(before.managers.every(manager => manager.points.state === 'settled' && manager.points.total === 50));
  f.pass('confirmed_par_cards_complete_and_fantasy_settles_exact_10_and_50');
  await member.goto(`/leaderboard?tournament=${f.tid}&round=${rid}&scope=round&metric=gross`);
  const row = member.locator('.leaderboard-row-link').filter({ hasText: f.names[0] });
  await expect(row.locator('.leaderboard-score span')).toHaveText('72 brutto');
  await member.getByRole('button', { name: 'Netto', exact: true }).click();
  await expect(row.locator('.leaderboard-score span')).toHaveText('72 netto');
  await expect(member).toHaveURL(/metric=net/);
  await shot(member, 'net-results', 1280);
  f.pass('player_gross_and_net_result_views_use_authoritative_preserved_cards');
  }

  const existing = await call(page.request, adminPath + '/scoring');
  await page.goto(adminUrl);
  if (existing.gross_total === 72) {
  await page.getByRole('button', { name: 'Korriger score', exact: true }).click();
  await page.locator('.scorecard-holes li').first().getByRole('button').click();
  const applied = page.waitForResponse(response => new URL(response.url()).pathname === `/api/rounds/${rid}/scores/conditional` && response.request().method() === 'PUT' && response.status() === 200);
  await page.getByRole('button', { name: 'Legg til ett slag', exact: true }).click();
  await (await applied).finished();
  await expect(page.getByText('Lagret på serveren', { exact: true })).toBeVisible();
  }
  await expect.poll(async () => (await call(page.request, adminPath + '/scoring')).gross_total).toBe(73);
  const changed = await call(page.request, adminPath + '/scoring');
  assert.equal(changed.gross_total, 73); assert.equal(changed.net_total, 73); assert.equal(changed.confirmed, false);
  const readiness = await call(page.request, `/api/rounds/${rid}/completion-validation`);
  assert.equal(readiness.ready_to_lock, false);
  const provisional = await call(page.request, fantasyPath);
  assert.equal(provisional.managers.find(value => value.user_id === f.auth.user_id).points.state, 'provisional');
  if (existing.gross_total === 72) await page.getByRole('button', { name: 'Oppsummering', exact: true }).click();
  await page.getByRole('button', { name: 'Bekreft fullført scorekort', exact: true }).click();
  await expect(page.getByText('Scorekortet er bekreftet', { exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Korriger score', exact: true })).toBeEnabled();
  await page.goto(manage);
  for (const player of f.players) f.expected.push({ path: `/api/rounds/${rid}/scorecards/player/${player}/scoring`, method: 'GET', status: 403, code: 'forbidden' });
  await transition(page, 'Lås runden');
  await expect(page.getByText('Runden er låst.', { exact: true })).toBeVisible();
  for (const [actor, player] of [[f.mutate, f.players[0]], [f.memberMutate, f.players[1]]]) {
    const denied = await actor(`/api/rounds/${rid}/scores`, { owner: { type: 'player', id: player }, hole_id: changed.holes[0].hole_id, gross_strokes: 9 }, 'PUT', 409);
    assert.equal(denied.error.code, 'round_not_editable');
  }
  await f.memberMutate(`/api/rounds/${rid}/lock`, {}, 'POST', 403);
  const locked = await call(page.request, adminPath);
  assert.equal(locked.gross_total, 73); assert.equal(locked.net_total, 73);
  for (const metric of ['gross', 'net']) {
    const board = await call(page.request, `/api/rounds/${rid}/leaderboards/${metric}`);
    assert.equal(board.metric, metric);
    assert.equal(board.entries.find(value => value.owner.id === f.players[0])[metric + '_total'], 73);
  }
  const settled = await call(page.request, fantasyPath);
  assert.deepEqual(settled.managers.find(value => value.user_id === f.auth.user_id).points, { state: 'settled', total: 38 });
  assert.deepEqual(settled.managers.find(value => value.user_id === f.memberAuth.user_id).points, { state: 'settled', total: 44 });
  f.pass('completed_round_correction_reconfirms_then_locked_writes_deny_and_gross_net_preserved');
  f.pass('corrected_fantasy_round_settles_exact_38_and_44');
  await shot(page, 'locked-round', 390);
};
