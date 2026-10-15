import { chromium, expect, request } from '../../../frontend/node_modules/@playwright/test/index.mjs';

const root = 'http://127.0.0.1:5173';
const trip = '00000000-0000-0000-0000-000000002001';
const round = '00000000-0000-0000-0000-000000004001';
const path = `/api/rounds/${round}/pairings`;
const url = `/manage/tournaments/${trip}?round=${round}#pairings`;
const password = process.env.GOLF_A1Q_DEV_PASSWORD;
if (!password) throw new Error('Set GOLF_A1Q_DEV_PASSWORD for the disposable seeded fixture.');
const browser = await chromium.launch({channel: 'chrome', headless: true});
const results = [];
const runKey = Date.now().toString(36);
try {
  const health = await request.newContext({baseURL: root});
  expect((await health.get('/api/health')).status()).toBe(200);
  expect((await health.get('/api/ready')).status()).toBe(200);
  await health.dispose();
  for (const width of [320, 390, 1280]) {
    const context = await browser.newContext({baseURL: root, viewport: {width, height: 900}});
    const page = await context.newPage();
    const errors = [], unexpected = [], allowed = new Set();
    let loggedOut = false, puts = 0, readMode = 'delay', putMode = 'delay';
    page.on('pageerror', e => errors.push(e.message));
    page.on('console', m => {if (m.type() === 'error' && !m.text().startsWith('Failed to load resource:')) errors.push(m.text());});
    page.on('response', response => {
      const endpoint = new URL(response.url()).pathname;
      const key = `${response.request().method()} ${response.status()} ${endpoint}`;
      if (response.status() >= 400 && !allowed.has(key) && !(response.status() === 401 && (endpoint === '/api/auth/session' || loggedOut))) unexpected.push(key);
    });
    page.on('requestfailed', r => {if (!r.failure()?.errorText.includes('ERR_ABORTED')) unexpected.push(`${r.failure()?.errorText} ${new URL(r.url()).pathname}`);});
    await page.goto('/login?returnTo=%2Ftournaments');
    await page.getByLabel('Brukernavn', {exact: true}).fill('admin');
    await page.getByLabel('Passord', {exact: true}).fill(password);
    await page.getByRole('button', {name: 'Logg inn', exact: true}).click();
    await expect(page).toHaveURL(/\/tournaments$/);
    const session = await (await page.request.get('/api/auth/session')).json();
    let model = await (await page.request.get(path)).json();
    expect(model.status).toBe('draft'); expect(model.teams.length).toBeGreaterThan(0); expect(model.flights.length).toBeGreaterThan(0);
    let releaseRead, markRead, releasePut, markPut;
    const readGate = new Promise(r => releaseRead = r), readSeen = new Promise(r => markRead = r);
    const putGate = new Promise(r => releasePut = r), putSeen = new Promise(r => markPut = r);
    await page.route('**' + path, async route => {
      if (route.request().method() === 'PUT') {
        puts++;
        if (putMode === 'delay') {markPut(); await putGate; return route.continue();}
        if (putMode === 'reject') {
          allowed.add(`PUT 401 ${path}`);
          return route.fulfill({status: 401, headers: {'cache-control': 'private, no-store'}, json: {error: {code: 'unauthenticated', message: 'authentication required'}}});
        }
        return route.continue();
      }
      if (readMode === 'delay') {markRead(); await readGate; return route.continue();}
      if (readMode === 'error') {
        allowed.add(`GET 503 ${path}`);
        return route.fulfill({status: 503, headers: {'cache-control': 'private, no-store'}, json: {error: {code: 'service_unavailable', message: 'service is not ready'}}});
      }
      if (readMode === 'empty') return route.fulfill({headers: {'cache-control': 'private, no-store'}, json: {...model, active_entrants: [], inactive_entrants: [], teams: [], flights: [], legacy_individual_groups: []}});
      return route.continue();
    });
    const panel = page.locator(`#pairing-editor-${round}`);
    const teams = panel.getByRole('region', {name: 'Lag', exact: true});
    const flights = panel.getByRole('region', {name: 'Flighter', exact: true});
    const names = teams.getByLabel('Navn', {exact: true});
    const save = panel.getByRole('button', {name: 'Lagre hele oppsettet', exact: true});
    const shot = state => page.screenshot({path: `/tmp/golf-a1q-${state}-${width}.png`, fullPage: true});
    const layout = async () => expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
    const populated = async () => {
      await expect(teams.locator('.pairing-group')).toHaveCount(model.teams.length);
      await expect(flights.locator('.pairing-group')).toHaveCount(model.flights.length);
      await expect(panel.getByText('Synkronisert med serveren', {exact: true})).toBeVisible();
    };
    await page.goto(url); await readSeen;
    await expect(panel.getByText('Laster spillegrupper…', {exact: true})).toBeVisible();
    await shot('loading'); readMode = 'pass'; releaseRead(); await populated();
    const longTeam = `Syntetisk lag med langt navn for kontroll av spillegrupper ved ${width} piksler ${runKey}`;
    const longFlight = `Syntetisk flight med langt navn for kontroll av startplan ved ${width} piksler ${runKey}`;
    const teamId = model.teams[0].id, flightId = model.flights[0].id;
    await names.first().fill(longTeam);
    await flights.getByLabel('Navn', {exact: true}).first().fill(longFlight);
    await expect(names.first()).toHaveValue(longTeam); await layout(); await shot('long-content');
    await teams.locator('.pairing-group').filter({hasText: 'Syntetisk deltaker med et langt visningsnavn'}).screenshot({path: `/tmp/golf-a1q-long-group-${width}.png`});
    await save.scrollIntoViewIfNeeded(); expect((await save.boundingBox()).height).toBeGreaterThanOrEqual(44);
    const success = page.waitForResponse(r => new URL(r.url()).pathname === path && r.request().method() === 'PUT');
    await save.click(); await putSeen;
    await expect(panel.getByRole('button', {name: 'Lagrer…', exact: true})).toBeDisabled();
    await shot('saving'); putMode = 'pass'; releasePut();
    const saved = await success; expect(saved.status()).toBe(200); expect(saved.headers()['cache-control']).toBe('private, no-store');
    await expect(save).toBeDisabled(); await expect(panel.getByText('Synkronisert med serveren', {exact: true})).toBeVisible();
    model = await (await page.request.get(path)).json();
    expect(model.teams.find(t => t.id === teamId).name).toBe(longTeam);
    expect(model.flights.find(f => f.id === flightId).name).toBe(longFlight);
    expect(puts).toBe(1);
    await page.reload(); await populated(); await layout(); await shot('populated');
    const stored = JSON.stringify(model), unsaved = `Ulagret lokalt utkast etter avvist lagring ved ${width} piksler`;
    await names.first().fill(unsaved); putMode = 'reject';
    const rejected = page.waitForResponse(r => new URL(r.url()).pathname === path && r.request().method() === 'PUT');
    const refreshed = page.waitForResponse(r => new URL(r.url()).pathname === path && r.request().method() === 'GET');
    await save.click(); const denial = await rejected;
    expect(denial.status()).toBe(401); expect(denial.headers()['cache-control']).toBe('private, no-store');
    await expect(panel.locator('.pairing-save-error')).toHaveText('Tilgangen til oppsettet ble avvist. Oppdater siden eller logg inn på nytt.');
    await expect(panel.locator('.pairing-conflict')).toContainText('En nyere versjon finnes');
    expect((await refreshed).status()).toBe(200);
    await expect(names.first()).toHaveValue(unsaved); await expect(names.first()).toBeDisabled(); await expect(save).toBeDisabled();
    expect(JSON.stringify(await (await page.request.get(path)).json())).toBe(stored); expect(puts).toBe(2);
    const discard = panel.getByRole('button', {name: 'Forkast og last på nytt', exact: true});
    await discard.scrollIntoViewIfNeeded(); expect((await discard.boundingBox()).height).toBeGreaterThanOrEqual(44);
    await layout(); await shot('rejected');
    await panel.locator('.pairing-conflict').screenshot({path: `/tmp/golf-a1q-recovery-${width}.png`});
    await discard.click();
    await expect(panel.locator('.pairing-save-error')).toHaveCount(0); await expect(panel.locator('.pairing-conflict')).toHaveCount(0);
    await populated(); await expect(names.first()).toHaveValue(model.teams[0].name); expect(puts).toBe(2); putMode = 'pass';
    readMode = 'error'; await page.reload();
    await expect(panel.locator('.pairing-read-error')).toContainText('Spillegruppene kunne ikke lastes.');
    await expect(panel.locator('.pairing-group')).toHaveCount(0); await shot('read-error');
    readMode = 'pass'; await panel.getByRole('button', {name: 'Prøv igjen', exact: true}).click(); await populated();
    readMode = 'empty'; await page.reload();
    for (const text of ['Ingen aktive deltakere kan tildeles i denne runden.', 'Ingen lag er opprettet.', 'Ingen flighter er opprettet.']) await expect(panel.getByText(text, {exact: true})).toBeVisible();
    await expect(save).toBeDisabled(); await layout(); await shot('empty');
    readMode = 'pass'; await page.reload(); await populated();
    await page.unroute('**' + path); loggedOut = true;
    expect((await page.request.post('/api/auth/logout', {headers: {'x-csrf-token': session.csrf_token}})).status()).toBe(204);
    const lost = await page.request.get(path); expect(lost.status()).toBe(401); expect(lost.headers()['cache-control']).toBe('private, no-store');
    await page.reload(); await expect(page).toHaveURL(/\/login/); await expect(page.locator('.pairing-editor')).toHaveCount(0);
    expect(errors).toEqual([]); expect(unexpected).toEqual([]);
    results.push({width, loading: true, populated: true, long_content_no_horizontal_overflow: true, real_ui_puts: 1, saving_state: true,
      injected_put_401: true, injected_denial_keeps_server_state: true, draft_retained_while_gets_remain_authorized: true,
      explicit_discard_no_write: true, no_automatic_put_retry: true, read_error_and_retry: true, injected_empty: true,
      actual_logout_private_401_and_login_redirect: true, page_errors: errors, unexpected_network_failures: unexpected});
    await context.close();
  }
  console.log(JSON.stringify(results, null, 2));
} finally {await browser.close();}
