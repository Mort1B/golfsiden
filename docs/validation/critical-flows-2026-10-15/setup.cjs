const crypto = require('node:crypto');
const { assert, call, expect, shot } = require('./support.cjs');

module.exports = async function setup(f) {
  if (!f.tid) {
  f.username = 'critical_org_' + crypto.randomBytes(5).toString('hex');
  f.memberName = 'critical_member_' + crypto.randomBytes(5).toString('hex');
  f.outsiderName = 'critical_out_' + crypto.randomBytes(5).toString('hex');
  f.password = crypto.randomBytes(24).toString('base64url');
  f.names = ['Arrangør med et langt navn til søndagens akseptanse', 'Spiller med egen konto og et langt navn', 'Tredje spiller med bevart turneringshandicap', 'Fjerde spiller i den felles produksjonsprøven'];
  f.persist();
  const page = f.organizer = await f.context(390);
  await page.goto('/');
  await page.getByRole('link', { name: /Opprett turnering/ }).click();
  await page.getByLabel('Turneringsnavn', { exact: true }).fill('Søndagens produksjonsprøve med et langt turneringsnavn');
  await page.getByRole('button', { name: 'Neste', exact: true }).click();
  await page.getByRole('button', { name: 'Legg til runde', exact: true }).click();
  await expect(page.locator('.round-editor')).toHaveCount(2);
  await page.getByRole('button', { name: 'Neste', exact: true }).click();
  await page.getByLabel('Visningsnavn', { exact: true }).fill(f.names[0]);
  await page.getByLabel(/^Brukernavn/).fill(f.username);
  await page.getByLabel(/^Passord/).fill(f.password);
  await page.getByLabel(/^Handicapindeks/).fill('0');
  await page.getByRole('button', { name: 'Neste', exact: true }).click();
  const created = page.waitForResponse(response => new URL(response.url()).pathname === '/api/onboarding/tournaments' && response.request().method() === 'POST');
  await page.getByRole('button', { name: 'Opprett turnering', exact: true }).click();
  const response = await created; assert.equal(response.status(), 201);
  const body = await response.json();
  f.tid = body.tournament.id; f.rounds = body.rounds; f.players = [body.session.player_id]; f.invitation = body.invitation; f.persist();
  await expect(page.getByRole('link', { name: 'Åpne turneringen', exact: true })).toBeVisible();
  f.auth = await call(page.request, '/api/auth/session');
  f.mutate = (endpoint, data = {}, method = 'POST', status = 200) => call(page.request, endpoint, data, method, status, f.auth.csrf_token);
  f.member = await f.context(320);
  await f.member.goto(`/join/${f.invitation.id}#token=${f.invitation.token}`);
  const registration = f.member.locator('.join-section').filter({ has: f.member.getByRole('heading', { name: 'Ny spiller' }) });
  await registration.getByLabel('Visningsnavn', { exact: true }).fill(f.names[1]);
  await registration.getByLabel(/^Brukernavn/).fill(f.memberName);
  await registration.getByLabel(/^Passord/).fill(f.password);
  await registration.getByLabel(/^Handicapindeks/).fill('0');
  await registration.getByRole('button', { name: 'Opprett konto og bli med' }).click();
  await expect(f.member.getByRole('heading', { name: 'Du er med!', exact: true })).toBeVisible();
  assert.equal(new URL(f.member.url()).hash, '');
  f.memberAuth = await call(f.member.request, '/api/auth/session'); f.players.push(f.memberAuth.player_id); f.persist();
  f.memberMutate = (endpoint, data = {}, method = 'POST', status = 200) => call(f.member.request, endpoint, data, method, status, f.memberAuth.csrf_token);
  await shot(f.member, 'invitation-joined', 320);
  f.pass('organizer_wizard_two_rounds_and_distinct_player_invitation_ui');

  await page.goto(`/manage/tournaments/${f.tid}#players`);
  const management = page.locator('.player-management');
  for (const name of f.names.slice(2)) {
    await management.getByLabel('Spillerens navn', { exact: true }).fill(name);
    await management.getByLabel('Spillerens handicap', { exact: true }).fill('0');
    await management.getByRole('button', { name: 'Opprett spiller og kontolenke' }).click();
    await expect(management.locator('.managed-players').getByText(name, { exact: true })).toBeVisible();
    await management.getByRole('button', { name: 'Skjul lenken', exact: true }).click();
  }
  const roster = await call(page.request, `/api/tournaments/${f.tid}/players`);
  f.players = f.names.map(name => { const player = roster.players.find(value => value.display_name === name); assert(player); return player.player_id; });
  f.persist();
  await shot(page, 'roster', 1280);
  f.pass('organizer_adds_four_player_roster_in_ui_no_team_requirement');
  } else {
    assert(f.checks.organizer_adds_four_player_roster_in_ui_no_team_requirement);
    await f.authenticate();
  }
  const page = f.organizer;

  for (const round of f.rounds) {
    await page.goto(`/manage/tournaments/${f.tid}?round=${round.id}#courses`);
    await page.getByRole('link', { name: 'Baner', exact: true }).click();
    const course = page.locator('.round-course-card').nth(round.round_number - 1);
    const current = await call(page.request, `/api/rounds/${round.id}`);
    if (!current.course_id) {
    const configure = course.getByRole('button', { name: 'Konfigurer', exact: true });
    if (await configure.getAttribute('aria-expanded') !== 'true') await configure.click();
    await course.getByLabel('Registrer manuelt', { exact: true }).check();
    await course.getByLabel('Banenavn', { exact: true }).fill('Manuell bane for den isolerte produksjonsprøven');
    await course.getByLabel('Utslagssted', { exact: true }).fill('Gul');
    await course.getByLabel('Baneverdi', { exact: true }).fill('72');
    await course.getByLabel('Slope', { exact: true }).fill('113');
    for (let hole = 1; hole <= 18; hole++) {
      await course.getByLabel(`Par, hull ${hole}`, { exact: true }).fill('4');
      await course.getByLabel(`Slagindeks, hull ${hole}`, { exact: true }).fill(String(hole));
    }
    await course.getByRole('button', { name: 'Lagre manuell bane', exact: true }).click();
    await expect(course.locator('.course-receipt')).toContainText('er lagret med');
    }
    await page.goto(`/manage/tournaments/${f.tid}?round=${round.id}#pairings`);
    const editor = page.locator('.pairing-editor:visible');
    const existing = await call(page.request, `/api/rounds/${round.id}/pairings`);
    if (existing.flights.length === 0) {
    await editor.getByRole('button', { name: 'Legg til flight', exact: true }).click();
    await editor.getByLabel('Starthull', { exact: true }).fill('1');
    for (const name of f.names) await editor.locator('.pairing-assignments label').filter({ hasText: name }).locator('select').selectOption({ label: 'Flight 1' });
    await editor.getByRole('button', { name: 'Lagre hele oppsettet', exact: true }).click();
    await expect(editor.getByText('Synkronisert med serveren', { exact: true })).toBeVisible();
    }
    const persisted = await call(page.request, `/api/rounds/${round.id}/pairings`);
    assert.equal(persisted.teams.length, 0); assert.equal(persisted.flights[0].members.length, 4);
  }
  await shot(page, 'round-setup', 390);
  f.rounds = await call(page.request, `/api/tournaments/${f.tid}/rounds`); f.persist();
  f.pass('two_round_manual_courses_and_four_player_flights_saved_in_ui');

  const outsider = await f.context();
  const day = new Date().toISOString().slice(0, 10);
  await call(outsider.request, '/api/onboarding/tournaments', { creator: { account: { username: f.outsiderName, password: f.password }, player: { display_name: 'Utenforstående kontrollkonto', handicap_index: 0 } },
    tournament: { name: 'Separat kontrollturnering', description: '', start_date: day, end_date: day, counted_rounds: 1, mandatory_round_number: null },
    rounds: [{ round_number: 1, name: 'Kontrollrunde', round_date: day, scoring_format: 'individual_stroke_play' }] }, 'POST', 201);
  const anon = await f.browser.newContext(f.options); f.contexts.push(anon);
  await call(anon.request, `/api/tournaments/${f.tid}`, undefined, 'GET', 401);
  await call(outsider.request, `/api/tournaments/${f.tid}`, undefined, 'GET', 403);
  await f.memberMutate(`/api/rounds/${f.rounds[0].id}/open`, {}, 'POST', 403);
  await f.memberMutate(`/api/tournaments/${f.tid}/fantasy`, { enabled: true }, 'PUT', 403);
  f.pass('anonymous_outsider_and_player_admin_actions_denied');
  await f.member.goto(`/manage/tournaments/${f.tid}?round=${f.rounds[0].id}#lifecycle`);
  await expect(f.member.getByRole('heading', { name: 'Ingen tilgang', exact: true })).toBeVisible();
  await expect(f.member.locator('.round-lifecycle')).toHaveCount(0);
  await shot(f.member, 'denied-management', 320);
  f.pass('player_management_route_denies_admin_controls_in_browser');
};
