import { chromium, expect, request } from '../../../frontend/node_modules/@playwright/test/index.mjs';

const root = 'http://127.0.0.1:5173';
const trip = '00000000-0000-0000-0000-000000002001';
const round = '00000000-0000-0000-0000-000000004001';
const writePath = `/api/rounds/${round}/course-configuration`;
const roundPath = `/api/rounds/${round}`;
const presetPath = `/api/tournaments/${trip}/course-presets`;
const url = `/manage/tournaments/${trip}?round=${round}#courses`;
const password = process.env.GOLF_A1R_DEV_PASSWORD;
if (!password) throw new Error('Set GOLF_A1R_DEV_PASSWORD for the disposable seeded fixture.');
const browser = await chromium.launch({channel: 'chrome', headless: true});
const results = [], runKey = Date.now().toString(36);
let diagnose = async () => {};
try {
  const health = await request.newContext({baseURL: root});
  expect((await health.get('/api/health')).status()).toBe(200);
  expect((await health.get('/api/ready')).status()).toBe(200);
  await health.dispose();
  for (const width of [320, 390, 1280]) {
    const context = await browser.newContext({baseURL: root, viewport: {width, height: 900}});
    const page = await context.newPage();
    const errors = [], unexpected = [], allowed = new Set();
    let loggedOut = false, puts = 0, providerDetails = 0, readMode = 'delay', putMode = 'delay';
    page.on('pageerror', error => errors.push(error.message));
    page.on('console', message => {if (message.type() === 'error' && !message.text().startsWith('Failed to load resource:')) errors.push(message.text());});
    page.on('response', response => {
      const path = new URL(response.url()).pathname;
      const key = `${response.request().method()} ${response.status()} ${path}`;
      if (response.status() >= 400 && !allowed.has(key) && !(response.status() === 401 && (path === '/api/auth/session' || loggedOut))) unexpected.push(key);
    });
    page.on('request', req => {if (new URL(req.url()).pathname.includes('/course-provider/courses/')) providerDetails++;});
    page.on('requestfailed', req => {if (!req.failure()?.errorText.includes('ERR_ABORTED')) unexpected.push(`${req.failure()?.errorText} ${new URL(req.url()).pathname}`);});
    await page.goto('/login?returnTo=%2Ftournaments');
    await page.getByLabel('Brukernavn', {exact: true}).fill('admin');
    await page.getByLabel('Passord', {exact: true}).fill(password);
    await page.getByRole('button', {name: 'Logg inn', exact: true}).click();
    await expect(page).toHaveURL(/\/tournaments$/);
    const session = await (await page.request.get('/api/auth/session')).json();
    const presets = await (await page.request.get(presetPath)).json();
    expect(presets.length).toBeGreaterThan(0);
    const longCourse = `Syntetisk bane med et langt navn for kontroll av baneoppsettet ved ${width} piksler ${runKey}`;
    const longTee = `Langt syntetisk utslagssted for kontroll ved ${width} piksler`;
    let releaseRead, markRead, releasePut, markPut;
    const readGate = new Promise(resolve => releaseRead = resolve), readSeen = new Promise(resolve => markRead = resolve);
    const putGate = new Promise(resolve => releasePut = resolve), putSeen = new Promise(resolve => markPut = resolve);
    await page.route('**' + presetPath, async route => {
      if (readMode === 'delay') {markRead(); await readGate; return route.continue();}
      if (readMode === 'error') {
        allowed.add(`GET 503 ${presetPath}`);
        return route.fulfill({status: 503, headers: {'cache-control': 'private, no-store'}, json: {error: {code: 'service_unavailable', message: 'service is not ready'}}});
      }
      if (readMode === 'empty') return route.fulfill({headers: {'cache-control': 'private, no-store'}, json: []});
      if (readMode === 'long') return route.fulfill({headers: {'cache-control': 'private, no-store'}, json: [{...presets[0], course_name: longCourse, tee: {...presets[0].tee, name: longTee}}]});
      return route.continue();
    });
    await page.route('**' + writePath, async route => {
      puts++;
      if (putMode === 'delay') {markPut(); await putGate; return route.continue();}
      if (putMode === 'reject') {
        allowed.add(`PUT 401 ${writePath}`);
        return route.fulfill({status: 401, headers: {'cache-control': 'private, no-store'}, json: {error: {code: 'unauthenticated', message: 'authentication required'}}});
      }
      return route.continue();
    });
    const panel = page.locator(`#course-editor-${round}`);
    const card = page.locator('.round-course-card').filter({has: panel});
    const saved = panel.locator('.saved-course-picker'), manual = panel.locator('.manual-course-form');
    diagnose = async () => {
      await page.screenshot({path: `/tmp/golf-a1r-failure-${width}.png`, fullPage: true});
      console.error(JSON.stringify({width, errors, unexpected, saved_picker_text: await saved.innerText()}));
    };
    const apply = saved.getByRole('button', {name: 'Bruk lagret bane på runden', exact: true});
    const save = manual.getByRole('button', {name: 'Lagre manuell bane', exact: true});
    const layout = async () => expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
    const shot = state => panel.screenshot({path: `/tmp/golf-a1r-${state}-${width}.png`});
    await page.goto(url); await readSeen;
    await expect(saved.getByRole('status')).toHaveText('Laster …'); await expect(apply).toBeDisabled();
    await shot('loading'); readMode = 'pass'; releaseRead();
    await saved.getByRole('combobox', {name: 'Lagret bane', exact: true}).selectOption(presets[0].id);
    await expect(saved.getByRole('heading', {name: presets[0].course_name, exact: true})).toBeVisible();
    await saved.getByText('Vis par og slagindeks for alle hull', {exact: true}).click();
    await expect(saved.locator('tbody tr')).toHaveCount(presets[0].tee.holes.length);
    await expect(apply).toBeEnabled(); await layout(); await shot('populated');
    readMode = 'long'; await page.reload();
    await saved.getByRole('combobox', {name: 'Lagret bane', exact: true}).selectOption(presets[0].id);
    await expect(saved.getByRole('heading', {name: longCourse, exact: true})).toBeVisible();
    await layout(); await shot('long-saved');
    readMode = 'pass';
    await panel.getByLabel('Velg fra katalog', {exact: true}).check();
    await expect(panel.getByText('Ingen katalogbaner har komplette leverandørfakta ennå. Bruk manuell registrering.', {exact: true})).toBeVisible();
    await expect(panel.getByRole('button', {name: 'Lagre valgt utslagssted', exact: true})).toBeDisabled();
    expect(providerDetails).toBe(0);
    await panel.getByLabel('Registrer manuelt', {exact: true}).check();
    await manual.getByLabel('Antall hull', {exact: true}).fill('18');
    await manual.getByLabel('Banenavn', {exact: true}).fill(longCourse);
    await manual.getByLabel('Utslagssted', {exact: true}).fill(longTee);
    await manual.getByLabel('Baneverdi', {exact: true}).fill('72');
    await manual.getByLabel('Slope', {exact: true}).fill('113');
    for (let n = 1; n <= 18; n++) {
      await manual.getByLabel(`Par, hull ${n}`, {exact: true}).fill('4');
      await expect(manual.getByLabel(`Slagindeks, hull ${n}`, {exact: true})).toHaveValue(String(n));
    }
    const headingDisplay = await manual.locator('.hole-grid-heading').evaluate(element => getComputedStyle(element).display);
    expect(headingDisplay === 'none').toBe(width < 700);
    await manual.locator('.manual-holes').screenshot({path: `/tmp/golf-a1r-hole-labels-${width}.png`});
    await layout(); await shot('long-manual');
    await save.scrollIntoViewIfNeeded(); expect((await save.boundingBox()).height).toBeGreaterThanOrEqual(44);
    const success = page.waitForResponse(response => new URL(response.url()).pathname === writePath && response.request().method() === 'PUT');
    await save.click(); await putSeen;
    await expect(manual.getByRole('button', {name: 'Lagrer …', exact: true})).toBeDisabled();
    await expect(manual.getByLabel('Banenavn', {exact: true})).toBeDisabled();
    await expect(panel.getByLabel('Registrer manuelt', {exact: true})).toBeDisabled();
    await shot('saving'); putMode = 'pass'; releasePut();
    const committed = await success; expect(committed.status()).toBe(200); expect(committed.headers()['cache-control']).toBe('private, no-store');
    await expect(card.locator('.course-receipt')).toContainText(`er lagret med ${longCourse} · ${longTee}`);
    await expect(panel).toBeHidden(); await expect(card.getByRole('button', {name: 'Endre', exact: true})).toBeFocused();
    let stored = await (await page.request.get(roundPath)).json();
    expect(stored.course_name).toBe(longCourse); expect(stored.tee_name).toBe(longTee); expect(stored.number_of_holes).toBe(18); expect(puts).toBe(1);
    await page.reload(); await expect(panel).toBeVisible();
    await expect(card.locator('.round-course-summary')).toContainText(longCourse); await layout();
    await card.locator('header').screenshot({path: `/tmp/golf-a1r-summary-${width}.png`});
    await panel.getByLabel('Registrer manuelt', {exact: true}).check();
    const unsaved = `Ulagret syntetisk bane etter avvist lagring ved ${width} piksler`;
    await manual.getByLabel('Banenavn', {exact: true}).fill(unsaved);
    await manual.getByLabel('Utslagssted', {exact: true}).fill(longTee);
    await manual.getByLabel('Baneverdi', {exact: true}).fill('72');
    await manual.getByLabel('Slope', {exact: true}).fill('113');
    for (let n = 1; n <= 18; n++) await manual.getByLabel(`Par, hull ${n}`, {exact: true}).fill('4');
    putMode = 'reject';
    const rejection = page.waitForResponse(response => new URL(response.url()).pathname === writePath && response.request().method() === 'PUT');
    await save.click(); const denied = await rejection;
    expect(denied.status()).toBe(401); expect(denied.headers()['cache-control']).toBe('private, no-store');
    await expect(manual.getByRole('alert')).toHaveText('Du har ikke lenger tilgang til å endre denne runden.');
    await expect(manual.getByLabel('Banenavn', {exact: true})).toHaveValue(unsaved);
    await expect(manual.getByLabel('Banenavn', {exact: true})).toBeEnabled(); await expect(save).toBeEnabled();
    expect(await (await page.request.get(roundPath)).json()).toEqual(stored); expect(puts).toBe(2);
    await layout(); await manual.locator('.course-form-error').screenshot({path: `/tmp/golf-a1r-denial-${width}.png`});
    await panel.getByLabel('Velg lagret bane', {exact: true}).check();
    readMode = 'error'; await page.reload();
    await expect(saved.getByRole('alert')).toBeVisible({timeout: 10000}); await expect(apply).toBeDisabled(); await shot('read-error');
    readMode = 'pass'; await saved.getByRole('button', {name: 'Prøv igjen', exact: true}).click();
    await expect(saved.getByRole('combobox', {name: 'Lagret bane', exact: true})).toBeVisible();
    readMode = 'empty'; await page.reload();
    await expect(saved.getByText('Ingen lagrede baner er tilgjengelige.', {exact: true})).toBeVisible(); await expect(apply).toBeDisabled();
    await layout(); await shot('empty'); expect(puts).toBe(2); expect(providerDetails).toBe(0);
    readMode = 'pass'; putMode = 'pass'; await page.unroute('**' + writePath); await page.unroute('**' + presetPath);
    loggedOut = true;
    expect((await page.request.post('/api/auth/logout', {headers: {'x-csrf-token': session.csrf_token}})).status()).toBe(204);
    const lost = await page.request.get(roundPath); expect(lost.status()).toBe(401); expect(lost.headers()['cache-control']).toBe('private, no-store');
    await page.reload(); await expect(page).toHaveURL(/\/login/); await expect(page.locator('.course-editor')).toHaveCount(0);
    expect(errors).toEqual([]); expect(unexpected).toEqual([]);
    results.push({width, saved_loading_populated: true, injected_saved_long_names: true, manual_real_save_and_reload: true, real_ui_puts: 1,
      saving_state: true, private_success_and_denial: true, injected_put_401: true, injected_denial_preserves_server_round: true,
      current_access_feedback: true, draft_retained_in_same_mounted_form: true, form_reenabled_after_denial: true, no_automatic_put_retry: true,
      injected_read_error_retry_and_empty: true, long_content_no_horizontal_overflow: true, actual_logout_private_401_login_redirect: true,
      real_catalog_unusable_fallback: true, provider_detail_requests: providerDetails, manual_hole_column_headings_visible: headingDisplay !== 'none',
      page_errors: errors, unexpected_network_failures: unexpected});
    await context.close();
  }
  console.log(JSON.stringify(results, null, 2));
} catch (error) {await diagnose(); throw error;}
finally {await browser.close();}
