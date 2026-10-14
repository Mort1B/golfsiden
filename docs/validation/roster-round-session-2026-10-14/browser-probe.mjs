import { chromium, expect, request } from '../../../frontend/node_modules/@playwright/test/index.mjs';

const root = 'http://127.0.0.1:5173';
const trip = '00000000-0000-0000-0000-000000002001';
const round = '00000000-0000-0000-0000-000000004001';
const longPlayer = 'Syntetisk gjest med langt navn for kontroll av turneringsroster på mobil og stor skjerm';
const longCourse = 'Syntetisk golfbane med langt navn ved fjorden for høstturen';
const longTee = 'Langt navn på utslagssted for kontroll av linjebryting';
const flows = [
  {path: `/api/tournaments/${trip}/players`, url: `/tournaments/${trip}`, kind: 'roster'},
  {path: `/api/tournaments/${trip}/rounds`, url: `/tournaments/${trip}`, kind: 'round-list'},
  {path: `/api/rounds/${round}`, url: `/rounds/${round}`, kind: 'round'},
];
const browser = await chromium.launch({channel: 'chrome', headless: true});
const results = [];
try {
  // Prepare long content through the existing authorized fixture workflows.
  const fixture = await request.newContext({baseURL: root});
  const login = await fixture.post('/api/auth/login', {data: {username: 'admin', password: 'golf-dev-2026'}});
  expect(login.status()).toBe(200);
  const session = await login.json();
  const headers = {'x-csrf-token': session.csrf_token};
  const roster = await (await fixture.get(flows[0].path)).json();
  if (!roster.players.some(p => p.display_name === longPlayer)) {
    const created = await fixture.post(flows[0].path, {headers, data: {display_name: longPlayer, handicap_index: 12}});
    expect(created.status()).toBe(201); // Do not print the private claim token.
  }
  const original = await (await fixture.get(flows[2].path)).json();
  if (original.course_name !== longCourse) {
    const configured = await fixture.put(`/api/rounds/${round}/course-configuration`, {headers, data: {
      expected_round_updated_at: original.updated_at,
      selection: {source: 'manual', course_name: longCourse, location: 'Syntetisk testbane', tee: {
        category: 'male', name: longTee, course_rating: 72, slope_rating: 113,
        holes: Array.from({length: 18}, (_, i) => ({par: 4, stroke_index: i + 1, distance: null})),
      }},
    }});
    expect(configured.status()).toBe(200);
  }
  await fixture.post('/api/auth/logout', {headers}); await fixture.dispose();
  for (const width of [320, 390, 1280]) {
    const context = await browser.newContext({baseURL: root, viewport: {width, height: 900}});
    const page = await context.newPage();
    const errors = [], unexpected = [], expectedFailures = new Set();
    page.on('pageerror', e => errors.push(e.message));
    page.on('console', m => {
      if (m.type() === 'error' && !m.text().startsWith('Failed to load resource:')) errors.push(m.text());
    });
    page.on('response', r => {
      const path = new URL(r.url()).pathname;
      if (r.status() >= 400 && !expectedFailures.has(`${r.status()} ${path}`) && !(path === '/api/auth/session' && r.status() === 401)) unexpected.push(`${r.status()} ${path}`);
    });
    page.on('requestfailed', r => {
      if (!r.failure()?.errorText.includes('ERR_ABORTED')) unexpected.push(`${r.failure()?.errorText} ${new URL(r.url()).pathname}`);
    });
    await page.goto('/login?returnTo=%2Ftournaments');
    await page.getByLabel('Brukernavn', {exact: true}).fill('admin');
    await page.getByLabel('Passord', {exact: true}).fill('golf-dev-2026');
    await page.getByRole('button', {name: 'Logg inn', exact: true}).click();
    await expect(page).toHaveURL(/\/tournaments$/);
    const session = await (await page.request.get('/api/auth/session')).json();
    const scopes = [];
    for (const flow of flows) {
      console.error(`${width}px ${flow.kind}`);
      let release, started;
      const gate = new Promise(r => release = r), seen = new Promise(r => started = r);
      let mode = 'delay', injected = 0;
      await page.route('**' + flow.path, async route => {
        if (mode === 'delay') { started(); await gate; return route.continue(); }
        if (mode === 'error' || mode === 'denied') {
          const status = mode === 'error' ? 503 : 403;
          expectedFailures.add(`${status} ${flow.path}`); injected++;
          return route.fulfill({status, headers: {'cache-control': 'private, no-store'}, json: {error: {
            code: status === 503 ? 'service_unavailable' : 'forbidden',
            message: status === 503 ? 'service is not ready' : 'session is not authorized for this tournament',
          }}});
        }
        if (mode === 'empty') return route.fulfill({json: flow.kind === 'roster' ? {players: [], handicap_correction: {state: 'editable'}} : []});
        return route.continue();
      });
      const scope = flow.kind === 'roster' ? page.locator('.tournament-players') : flow.kind === 'round-list' ? page.locator('.round-list') : page.locator('.round-details');
      async function populated() {
        if (flow.kind === 'roster') { await expect(scope.locator('.tournament-player-row')).toHaveCount(9); await expect(scope.getByText(longPlayer, {exact: true})).toBeVisible(); }
        else if (flow.kind === 'round-list') { await expect(scope.locator('.round-row')).toHaveCount(5); await expect(scope).toContainText(longCourse); }
        else { await expect(page.getByRole('heading', {level: 1, name: 'Åpningsrunden'})).toBeVisible(); await expect(scope).toContainText(longCourse); }
      }
      await page.goto(flow.url); await seen;
      await expect(page.getByRole('status').filter({hasText: 'Laster …'}).first()).toBeVisible();
      await page.screenshot({path: `/tmp/golf-a1c-${flow.kind}-loading-${width}.png`, fullPage: true});
      mode = 'pass'; release(); await populated();
      expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
      await page.screenshot({path: `/tmp/golf-a1c-${flow.kind}-populated-${width}.png`, fullPage: true});
      const r = await page.request.get(flow.path);
      expect(r.status()).toBe(200); expect(r.headers()['cache-control']).toBe('private, no-store');
      const value = await r.json();
      if (flow.kind === 'roster') { expect(value.players.length).toBe(9); expect(value.handicap_correction.state).toBe('editable'); }
      else if (flow.kind === 'round-list') expect(value.map(r => r.round_number)).toEqual([1, 2, 3, 4, 5]);
      else { expect(value.tournament_id).toBe(trip); expect(value.course_name).toBe(longCourse); }
      mode = 'error'; await page.reload(); await expect(page.getByRole('alert').first()).toBeVisible();
      if (flow.kind === 'roster') await expect(page.locator('.tournament-player-row')).toHaveCount(0);
      if (flow.kind === 'round-list') await expect(page.locator('.round-row')).toHaveCount(0);
      if (flow.kind === 'round') await expect(page.getByRole('heading', {level: 1, name: 'Åpningsrunden'})).toHaveCount(0);
      await page.screenshot({path: `/tmp/golf-a1c-${flow.kind}-error-${width}.png`, fullPage: true});
      mode = 'pass'; await page.getByRole('button', {name: 'Prøv igjen', exact: true}).first().click(); await populated();
      if (flow.kind !== 'round') {
        mode = 'empty'; await page.reload();
        await expect(page.getByText(flow.kind === 'roster' ? 'Ingen spillere er påmeldt.' : 'Ingen runder er opprettet ennå. Arrangøren kan opprette runder i administrasjonen.', {exact: true})).toBeVisible();
        await page.screenshot({path: `/tmp/golf-a1c-${flow.kind}-empty-${width}.png`, fullPage: true});
      } else {
        mode = 'denied'; await page.reload(); await expect(page.getByRole('alert')).toBeVisible();
        await expect(page.getByRole('heading', {level: 1, name: 'Åpningsrunden'})).toHaveCount(0);
        await page.screenshot({path: `/tmp/golf-a1c-round-denied-${width}.png`, fullPage: true});
      }
      expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
      scopes.push({kind: flow.kind, path: flow.path, loading: true, valid_json_and_cache: true, long_content_no_horizontal_overflow: true, error_and_read_retry: true, empty_or_denied: true, injected_failures: injected});
      await page.unroute('**' + flow.path);
    }
    const logout = await page.request.post('/api/auth/logout', {headers: {'x-csrf-token': session.csrf_token}}); expect(logout.status()).toBe(204);
    for (const {path} of flows) {
      const r = await page.request.get(path); expect(r.status()).toBe(401); expect(r.headers()['cache-control']).toBe('private, no-store');
      expect((await r.json()).error.code).toBe('unauthenticated');
    }
    await page.goto('/tournaments'); await expect(page).toHaveURL(/\/login/);
    expect(errors).toEqual([]); expect(unexpected).toEqual([]);
    results.push({width, scopes, logged_out_all_three_private_401: true, page_errors: errors, unexpected_network_failures: unexpected});
    await context.close();
  }
  console.log(JSON.stringify(results, null, 2));
} finally { await browser.close(); }
