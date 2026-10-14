import { chromium, expect, request } from '../../../frontend/node_modules/@playwright/test/index.mjs';

const root = 'http://127.0.0.1:5173';
const trip = '00000000-0000-0000-0000-000000002001';
const round = '00000000-0000-0000-0000-000000004001';
const pairingPath = `/api/rounds/${round}/pairings`;
const teamsPath = `/api/rounds/${round}/teams`;
const longTeam = 'Syntetisk lag med langt navn for kontroll av spillegrupper på mobil og stor skjerm';
const longFlight = 'Syntetisk flight med langt navn for kontroll av linjebryting og bevart startoppsett';
const browser = await chromium.launch({channel: 'chrome', headless: true});
const results = [];
try {
  const fixture = await request.newContext({baseURL: root});
  const login = await fixture.post('/api/auth/login', {data: {username: 'admin', password: 'golf-dev-2026'}});
  expect(login.status()).toBe(200); const session = await login.json();
  const current = await (await fixture.get(pairingPath)).json();
  // Change only synthetic group names through the real draft replacement API.
  const changed = await fixture.put(pairingPath, {headers: {'x-csrf-token': session.csrf_token}, data: {
    expected_round_updated_at: current.updated_at,
    teams: current.teams.map((t, i) => ({id: t.id, name: i === 0 ? longTeam : t.name,
      members: t.members.map(m => ({player_id: m.player_id})), schedule_flight_id: null})),
    flights: current.flights.map((f, i) => ({id: f.id, name: i === 0 ? longFlight : f.name,
      starting_hole: f.starting_hole, tee_time: f.tee_time, members: f.members.map(m => ({player_id: m.player_id}))})),
    legacy_conversions: [],
  }});
  expect(changed.status()).toBe(200); const populatedModel = await changed.json();
  await fixture.post('/api/auth/logout', {headers: {'x-csrf-token': session.csrf_token}}); await fixture.dispose();
  for (const width of [320, 390, 1280]) {
    console.error(`${width}px pairing UI and team API`);
    const context = await browser.newContext({baseURL: root, viewport: {width, height: 900}});
    const page = await context.newPage();
    const errors = [], unexpected = [], expectedFailures = new Set();
    page.on('pageerror', e => errors.push(e.message));
    page.on('console', m => {if (m.type() === 'error' && !m.text().startsWith('Failed to load resource:')) errors.push(m.text());});
    page.on('response', r => {
      const path = new URL(r.url()).pathname;
      if (r.status() >= 400 && !expectedFailures.has(`${r.status()} ${path}`) && !(path === '/api/auth/session' && r.status() === 401)) unexpected.push(`${r.status()} ${path}`);
    });
    page.on('requestfailed', r => {if (!r.failure()?.errorText.includes('ERR_ABORTED')) unexpected.push(`${r.failure()?.errorText} ${new URL(r.url()).pathname}`);});
    await page.goto('/login?returnTo=%2Ftournaments');
    await page.getByLabel('Brukernavn', {exact: true}).fill('admin');
    await page.getByLabel('Passord', {exact: true}).fill('golf-dev-2026');
    await page.getByRole('button', {name: 'Logg inn', exact: true}).click(); await expect(page).toHaveURL(/\/tournaments$/);
    const session = await (await page.request.get('/api/auth/session')).json();
    let release, started; const gate = new Promise(r => release = r), seen = new Promise(r => started = r);
    let mode = 'delay', injected = 0;
    await page.route('**' + pairingPath, async route => {
      if (mode === 'delay') {started(); await gate; return route.continue();}
      if (mode === 'error' || mode === 'denied') {
        const status = mode === 'error' ? 503 : 403; expectedFailures.add(`${status} ${pairingPath}`); injected++;
        return route.fulfill({status, headers: {'cache-control': 'private, no-store'}, json: {error: {
          code: status === 503 ? 'service_unavailable' : 'forbidden',
          message: status === 503 ? 'service is not ready' : 'session is not authorized for this tournament',
        }}});
      }
      if (mode === 'empty') return route.fulfill({json: {...populatedModel, teams: [], flights: [], legacy_individual_groups: []}});
      return route.continue();
    });
    const teamsRegion = page.getByRole('region', {name: 'Lag', exact: true});
    const flightsRegion = page.getByRole('region', {name: 'Flighter', exact: true});
    async function populated() {
      await expect(teamsRegion.locator('.team-card')).toHaveCount(4); await expect(flightsRegion.locator('.team-card')).toHaveCount(2);
      await expect(teamsRegion.getByRole('heading', {name: longTeam, exact: true})).toBeVisible();
      await expect(flightsRegion.getByRole('heading', {name: longFlight, exact: true})).toBeVisible();
    }
    await page.goto(`/rounds/${round}`); await seen;
    await expect(page.getByRole('status').filter({hasText: 'Laster …'}).first()).toBeVisible();
    await page.screenshot({path: `/tmp/golf-a1d-loading-${width}.png`, fullPage: true});
    mode = 'pass'; release(); await populated();
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
    await page.screenshot({path: `/tmp/golf-a1d-populated-${width}.png`, fullPage: true});
    // No production UI consumer calls /teams. Exercise its real typed boundary
    // in this browser, and separately assert the actual HTTP/cache envelope.
    const decoded = await page.evaluate(async ({round, trip}) => {
      const {teamApi} = await import('/src/api/teams.ts'); return teamApi.list(round, trip);
    }, {round, trip});
    expect(decoded.length).toBe(4); expect(decoded.some(t => t.name === longTeam)).toBe(true);
    expect(decoded.every(t => t.round_id === round && t.tournament_id === trip && t.members.length === 2)).toBe(true);
    for (const path of [pairingPath, teamsPath]) {
      const r = await page.request.get(path); expect(r.status()).toBe(200); expect(r.headers()['cache-control']).toBe('private, no-store');
      const value = await r.json(); if (path === pairingPath) expect(value.round_id).toBe(round); else expect(value.length).toBe(4);
    }
    mode = 'error'; await page.reload(); await expect(page.getByRole('alert')).toBeVisible();
    await expect(page.locator('.round-group')).toHaveCount(0);
    await page.screenshot({path: `/tmp/golf-a1d-error-${width}.png`, fullPage: true});
    mode = 'pass'; await page.getByRole('button', {name: 'Prøv igjen', exact: true}).click(); await populated();
    mode = 'empty'; await page.reload();
    await expect(page.getByText('Ingen flighter er satt opp.', {exact: true})).toBeVisible();
    await expect(page.getByText('Ingen lag er satt opp.', {exact: true})).toBeVisible();
    await page.screenshot({path: `/tmp/golf-a1d-empty-${width}.png`, fullPage: true});
    mode = 'denied'; await page.reload(); await expect(page.getByRole('alert')).toBeVisible();
    await expect(page.locator('.round-group')).toHaveCount(0);
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
    await page.screenshot({path: `/tmp/golf-a1d-denied-${width}.png`, fullPage: true});
    await page.unroute('**' + pairingPath);
    const logout = await page.request.post('/api/auth/logout', {headers: {'x-csrf-token': session.csrf_token}}); expect(logout.status()).toBe(204);
    for (const path of [pairingPath, teamsPath]) {
      const r = await page.request.get(path); expect(r.status()).toBe(401); expect(r.headers()['cache-control']).toBe('private, no-store');
      expect((await r.json()).error.code).toBe('unauthenticated');
    }
    await page.goto(`/rounds/${round}`); await expect(page).toHaveURL(/\/login/);
    expect(errors).toEqual([]); expect(unexpected).toEqual([]);
    results.push({width, pairing_ui: {loading: true, populated: true, long_group_names_no_overflow: true, error_and_read_retry: true, empty_and_denied: true},
      teams_api: {typed_decoder: true, groups: decoded.length, member_and_round_identity: true, private_http_cache: true, ui_consumer: false},
      logged_out_both_private_401: true, injected_responses: injected, page_errors: errors, unexpected_network_failures: unexpected});
    await context.close();
  }
  console.log(JSON.stringify(results, null, 2));
} finally {await browser.close();}
