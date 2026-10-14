import { chromium, expect, request } from '../../../frontend/node_modules/@playwright/test/index.mjs';

const root = 'http://127.0.0.1:5173';
const trip = '00000000-0000-0000-0000-000000002001';
const name = 'Guttas Golf – lang turneringstittel for høstturen med venner og gjester 2026';
const flows = [
  {path: '/api/me/tournaments', url: '/tournaments', kind: 'mine'},
  {path: `/api/tournaments/${trip}`, url: `/tournaments/${trip}`, kind: 'detail'},
  {path: '/api/tournaments', url: '/score', kind: 'list'},
];
const browser = await chromium.launch({channel: 'chrome', headless: true});
const results = [];
try {
  // Prepare long content through the existing authorized details workflow.
  const fixture = await request.newContext({baseURL: root});
  const login = await fixture.post('/api/auth/login', {data: {username: 'admin', password: 'golf-dev-2026'}});
  expect(login.status()).toBe(200);
  const session = await login.json();
  const original = await (await fixture.get(`/api/tournaments/${trip}`)).json();
  const changed = await fixture.patch(`/api/tournaments/${trip}/details`, {
    headers: {'x-csrf-token': session.csrf_token},
    data: {name, description: 'Syntetisk kontroll av private turneringslister og detaljer på mobil og stor skjerm. Lange navn og tekst skal brytes uten vannrett rulling.', start_date: original.start_date, end_date: original.end_date, expected_tournament_updated_at: original.updated_at},
  });
  expect(changed.status()).toBe(200);
  await fixture.post('/api/auth/logout', {headers: {'x-csrf-token': session.csrf_token}});
  await fixture.dispose();
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
        if (mode === 'empty') return route.fulfill({json: []});
        return route.continue();
      });
      async function populated() {
        if (flow.kind === 'mine') await expect(page.locator('.list-card')).toHaveCount(1);
        else if (flow.kind === 'detail') await expect(page.getByRole('heading', {level: 1, name})).toBeVisible();
        else await expect(page.getByText('Turneringen har ingen åpne, fullførte eller låste runder')).toBeVisible();
      }
      await page.goto(flow.url); await seen;
      await expect(page.getByRole('status').filter({hasText: 'Laster …'}).first()).toBeVisible();
      await page.screenshot({path: `/tmp/golf-a1b-${flow.kind}-loading-${width}.png`, fullPage: true});
      mode = 'pass'; release(); await populated();
      expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
      await page.screenshot({path: `/tmp/golf-a1b-${flow.kind}-populated-${width}.png`, fullPage: true});
      const r = await page.request.get(flow.path);
      expect(r.status()).toBe(200); expect(r.headers()['cache-control']).toBe('private, no-store');
      const value = await r.json();
      if (flow.kind === 'mine') { expect(value[0].tournament.name).toBe(name); expect(value[0].role).toBe('admin'); expect(value[0].player_id).toBeNull(); }
      else if (flow.kind === 'detail') expect(value.name).toBe(name);
      else expect(value[0].name).toBe(name);
      mode = 'error'; await page.reload();
      await expect(page.getByRole('alert').first()).toBeVisible();
      if (flow.kind === 'mine') await expect(page.locator('.list-card')).toHaveCount(0);
      if (flow.kind === 'detail') await expect(page.getByRole('heading', {level: 1, name})).toHaveCount(0);
      await page.screenshot({path: `/tmp/golf-a1b-${flow.kind}-error-${width}.png`, fullPage: true});
      mode = 'pass'; await page.getByRole('button', {name: 'Prøv igjen', exact: true}).first().click(); await populated();
      if (flow.kind !== 'detail') {
        mode = 'empty'; await page.reload();
        await expect(page.getByText(flow.kind === 'mine' ? 'Du er ikke med i noen turneringer ennå.' : 'Ingen turneringer er opprettet', {exact: true})).toBeVisible();
        await page.screenshot({path: `/tmp/golf-a1b-${flow.kind}-empty-${width}.png`, fullPage: true});
      } else {
        mode = 'denied'; await page.reload();
        await expect(page.getByRole('alert')).toContainText('Turneringen er ikke tilgjengelig for denne innloggingen.');
        await expect(page.getByRole('heading', {level: 1, name})).toHaveCount(0);
        await page.screenshot({path: `/tmp/golf-a1b-detail-denied-${width}.png`, fullPage: true});
      }
      expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
      scopes.push({kind: flow.kind, path: flow.path, loading: true, valid_json_and_cache: true, error_and_read_retry: true, empty_or_denied: true, no_horizontal_overflow: true, injected_failures: injected});
      await page.unroute('**' + flow.path);
    }
    const logout = await page.request.post('/api/auth/logout', {headers: {'x-csrf-token': session.csrf_token}});
    expect(logout.status()).toBe(204);
    for (const {path} of flows) {
      const r = await page.request.get(path);
      expect(r.status()).toBe(401); expect(r.headers()['cache-control']).toBe('private, no-store');
      expect((await r.json()).error.code).toBe('unauthenticated');
    }
    await page.goto('/tournaments'); await expect(page).toHaveURL(/\/login/);
    expect(errors).toEqual([]); expect(unexpected).toEqual([]);
    results.push({width, scopes, logged_out_all_three_private_401: true, page_errors: errors, unexpected_network_failures: unexpected});
    await context.close();
  }
  console.log(JSON.stringify(results, null, 2));
} finally { await browser.close(); }
