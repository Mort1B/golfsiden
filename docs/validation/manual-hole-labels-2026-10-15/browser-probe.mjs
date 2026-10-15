import { chromium, expect, request } from '../../../frontend/node_modules/@playwright/test/index.mjs';

const root = 'http://127.0.0.1:5173';
const trip = '00000000-0000-0000-0000-000000002001';
const round = '00000000-0000-0000-0000-000000004001';
const fixedRound = '00000000-0000-0000-0000-000000004005';
const fixedWritePath = `/api/rounds/${fixedRound}/course-configuration`;
const writePath = `/api/rounds/${round}/course-configuration`;
const roundPath = `/api/rounds/${round}`;
const presetPath = `/api/tournaments/${trip}/course-presets`;
const url = `/manage/tournaments/${trip}?round=${round}#courses`;
const password = process.env.GOLF_A1S_DEV_PASSWORD;
if (!password) throw new Error('Set GOLF_A1S_DEV_PASSWORD for the disposable seeded fixture.');
const browser = await chromium.launch({channel: 'chrome', headless: true});
const results = [], runKey = Date.now().toString(36);
let diagnose = async () => {};

async function inspectHeading(form, width) {
  const heading = form.locator('.hole-grid-heading');
  await expect(heading).toBeVisible();
  await expect(heading).toHaveAttribute('aria-hidden', 'true');
  expect(await heading.locator('span').allTextContents()).toEqual(['Hull', 'Par', 'Slagindeks', 'Avstand (yards)']);
  const cells = await heading.evaluate(element => Array.from(element.children, cell => {
    const rect = cell.getBoundingClientRect(), range = document.createRange();
    range.selectNodeContents(cell);
    return {left: rect.left, right: rect.right, bottom: rect.bottom, lines: Array.from(range.getClientRects(), line => ({left: line.left, right: line.right, bottom: line.bottom}))};
  }));
  const first = form.locator('.manual-hole-row').first();
  const row = await first.boundingBox();
  for (const cell of cells) for (const line of cell.lines) {
    expect(line.left).toBeGreaterThanOrEqual(cell.left - 1);
    expect(line.right).toBeLessThanOrEqual(cell.right + 1);
    expect(line.left).toBeGreaterThanOrEqual(0);
    expect(line.right).toBeLessThanOrEqual(width);
    expect(line.bottom).toBeLessThanOrEqual(row.y);
  }
  const inputs = await first.locator('input').all();
  for (let n = 0; n < inputs.length; n++) {
    const box = await inputs[n].boundingBox();
    expect(Math.abs(box.x - cells[n + 1].left)).toBeLessThanOrEqual(1);
    expect(Math.abs(box.x + box.width - cells[n + 1].right)).toBeLessThanOrEqual(1);
    expect(box.width).toBeGreaterThanOrEqual(44); expect(box.height).toBeGreaterThanOrEqual(44);
  }
  return cells;
}
try {
  const health = await request.newContext({baseURL: root});
  expect((await health.get('/api/health')).status()).toBe(200);
  expect((await health.get('/api/ready')).status()).toBe(200);
  await health.dispose();
  for (const width of [320, 390, 1280]) {
    const context = await browser.newContext({baseURL: root, viewport: {width, height: 900}});
    const page = await context.newPage();
    const errors = [], unexpected = [], allowed = new Set();
    let loggedOut = false, puts = 0, fixedPuts = 0, providerDetails = 0, readMode = 'delay', putMode = 'delay';
    page.on('pageerror', error => errors.push(error.message));
    page.on('console', message => {if (message.type() === 'error' && !message.text().startsWith('Failed to load resource:')) errors.push(message.text());});
    page.on('response', response => {
      const path = new URL(response.url()).pathname;
      const key = `${response.request().method()} ${response.status()} ${path}`;
      if (response.status() >= 400 && !allowed.has(key) && !(response.status() === 401 && (path === '/api/auth/session' || loggedOut))) unexpected.push(key);
    });
    page.on('request', req => {
      const path = new URL(req.url()).pathname;
      if (path.includes('/course-provider/courses/')) providerDetails++;
      if (req.method() === 'PUT' && path === fixedWritePath) fixedPuts++;
    });
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
      await panel.screenshot({path: `/tmp/golf-a1s-failure-${width}.png`});
      console.error(JSON.stringify({width, errors, unexpected}));
    };
    const apply = saved.getByRole('button', {name: 'Bruk lagret bane på runden', exact: true});
    const save = manual.getByRole('button', {name: 'Lagre manuell bane', exact: true});
    const layout = async () => expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
    const shot = state => panel.screenshot({path: `/tmp/golf-a1s-${state}-${width}.png`});
    const shotRows = async (form, state) => {
      const holes = form.locator('.manual-holes');
      await holes.evaluate(element => element.scrollIntoView({block: 'start'}));
      const box = await holes.boundingBox(), last = await form.locator('.manual-hole-row').nth(2).boundingBox();
      await page.screenshot({path: `/tmp/golf-a1s-${state}-${width}.png`, clip: {x: box.x, y: box.y, width: box.width, height: last.y + last.height - box.y}});
    };
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
    await expect(manual.getByLabel('Par, hull 1', {exact: true})).toHaveValue('');
    const headingGeometry = await inspectHeading(manual, width);
    await shotRows(manual, 'empty-rows');
    await manual.getByLabel('Banenavn', {exact: true}).fill(longCourse);
    await manual.getByLabel('Utslagssted', {exact: true}).fill(longTee);
    await manual.getByLabel('Baneverdi', {exact: true}).fill('72');
    await manual.getByLabel('Slope', {exact: true}).fill('113');
    await save.click();
    await expect(manual.locator('[data-validation-key="holes.0.par"]')).toBeFocused();
    await expect(manual.locator('[data-validation-key="holes.0.par"]')).toHaveAttribute('aria-invalid', 'true');
    await expect(manual.getByRole('alert')).toContainText('Rett feltene');
    expect(puts).toBe(0); await layout(); await inspectHeading(manual, width);
    await shotRows(manual, 'validation');
    await manual.getByLabel('Antall hull', {exact: true}).fill('9');
    await expect(manual.locator('.manual-hole-row')).toHaveCount(9);
    for (let n = 1; n <= 9; n++) await manual.getByLabel(`Par, hull ${n}`, {exact: true}).fill('4');
    await manual.getByLabel('Avstand i yards, hull 1', {exact: true}).fill('401');
    await inspectHeading(manual, width); await layout();
    await manual.getByLabel('Antall hull', {exact: true}).fill('18');
    await expect(manual.locator('.manual-hole-row')).toHaveCount(18);
    await expect(manual.getByLabel('Par, hull 1', {exact: true})).toHaveValue('4');
    await expect(manual.getByLabel('Avstand i yards, hull 1', {exact: true})).toHaveValue('401');
    await expect(manual.getByLabel('Par, hull 10', {exact: true})).toHaveValue('');
    for (let n = 1; n <= 18; n++) {
      await manual.getByLabel(`Par, hull ${n}`, {exact: true}).fill('4');
      await expect(manual.getByLabel(`Slagindeks, hull ${n}`, {exact: true})).toHaveValue(String(n));
    }
    await manual.getByLabel('Par, hull 1', {exact: true}).focus();
    await page.keyboard.press('Tab'); await expect(manual.getByLabel('Slagindeks, hull 1', {exact: true})).toBeFocused();
    await page.keyboard.press('Tab'); await expect(manual.getByLabel('Avstand i yards, hull 1', {exact: true})).toBeFocused();
    await inspectHeading(manual, width);
    const headingDisplay = await manual.locator('.hole-grid-heading').evaluate(element => getComputedStyle(element).display);
    expect(headingDisplay).toBe('grid');
    await shotRows(manual, 'hole-labels');
    await layout(); await shot('long-manual');
    await save.scrollIntoViewIfNeeded(); expect((await save.boundingBox()).height).toBeGreaterThanOrEqual(44);
    const success = page.waitForResponse(response => new URL(response.url()).pathname === writePath && response.request().method() === 'PUT');
    await save.click(); await putSeen;
    await expect(manual.getByRole('button', {name: 'Lagrer …', exact: true})).toBeDisabled();
    await expect(manual.getByLabel('Banenavn', {exact: true})).toBeDisabled();
    await expect(panel.getByLabel('Registrer manuelt', {exact: true})).toBeDisabled();
    await inspectHeading(manual, width);
    await shot('saving'); putMode = 'pass'; releasePut();
    const committed = await success; expect(committed.status()).toBe(200); expect(committed.headers()['cache-control']).toBe('private, no-store');
    await expect(card.locator('.course-receipt')).toContainText(`er lagret med ${longCourse} · ${longTee}`);
    await expect(panel).toBeHidden(); await expect(card.getByRole('button', {name: 'Endre', exact: true})).toBeFocused();
    let stored = await (await page.request.get(roundPath)).json();
    expect(stored.course_name).toBe(longCourse); expect(stored.tee_name).toBe(longTee); expect(stored.number_of_holes).toBe(18); expect(puts).toBe(1);
    await page.reload(); await expect(panel).toBeVisible();
    await expect(card.locator('.round-course-summary')).toContainText(longCourse); await layout();
    await card.locator('header').screenshot({path: `/tmp/golf-a1s-summary-${width}.png`});
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
    await layout(); await manual.locator('.course-form-error').screenshot({path: `/tmp/golf-a1s-denial-${width}.png`});
    await panel.getByLabel('Velg lagret bane', {exact: true}).check();
    readMode = 'error'; await page.reload();
    await expect(saved.getByRole('alert')).toBeVisible({timeout: 10000}); await expect(apply).toBeDisabled(); await shot('read-error');
    readMode = 'pass'; await saved.getByRole('button', {name: 'Prøv igjen', exact: true}).click();
    await expect(saved.getByRole('combobox', {name: 'Lagret bane', exact: true})).toBeVisible();
    readMode = 'empty'; await page.reload();
    await expect(saved.getByText('Ingen lagrede baner er tilgjengelige.', {exact: true})).toBeVisible(); await expect(apply).toBeDisabled();
    await layout(); await shot('empty'); expect(puts).toBe(2); expect(providerDetails).toBe(0);
    readMode = 'pass';
    await page.goto(`/manage/tournaments/${trip}?round=${fixedRound}#courses`);
    const fixedPanel = page.locator(`#course-editor-${fixedRound}`);
    const fixedCard = page.locator('.round-course-card').filter({has: fixedPanel});
    await fixedPanel.getByLabel('Registrer manuelt', {exact: true}).check();
    const fixed = fixedPanel.locator('.manual-course-form');
    await expect(fixed.getByLabel('Antall hull', {exact: true})).toHaveValue('18');
    await expect(fixed.getByLabel('Antall hull', {exact: true})).toHaveAttribute('readonly', '');
    await expect(fixed.locator('.manual-hole-row')).toHaveCount(18);
    await inspectHeading(fixed, width);
    await fixed.getByLabel('Banenavn', {exact: true}).fill(longCourse + ' fast');
    await fixed.getByLabel('Utslagssted', {exact: true}).fill(longTee);
    await fixed.getByLabel('Baneverdi', {exact: true}).fill('72');
    await fixed.getByLabel('Slope', {exact: true}).fill('113');
    for (let n = 1; n <= 18; n++) await fixed.getByLabel(`Par, hull ${n}`, {exact: true}).fill('4');
    await layout(); await shotRows(fixed, 'fixed');
    const fixedSaved = page.waitForResponse(response => new URL(response.url()).pathname === fixedWritePath && response.request().method() === 'PUT');
    await fixed.getByRole('button', {name: 'Lagre manuell bane', exact: true}).click();
    const fixedResponse = await fixedSaved; expect(fixedResponse.status()).toBe(200); expect(fixedResponse.headers()['cache-control']).toBe('private, no-store');
    await expect(fixedCard.locator('.course-receipt')).toContainText(longCourse + ' fast');
    await expect(fixedPanel).toBeHidden(); await expect(fixedCard.getByRole('button', {name: 'Endre', exact: true})).toBeFocused();
    await page.reload();
    const fixedStored = await (await page.request.get(`/api/rounds/${fixedRound}`)).json();
    expect(fixedStored.course_name).toBe(longCourse + ' fast'); expect(fixedStored.number_of_holes).toBe(18);
    await expect(fixedCard.locator('.round-course-summary')).toContainText(longCourse + ' fast');
    expect(fixedPuts).toBe(1); expect(puts).toBe(2); await layout();
    readMode = 'pass'; putMode = 'pass'; await page.unroute('**' + writePath); await page.unroute('**' + presetPath);
    loggedOut = true;
    expect((await page.request.post('/api/auth/logout', {headers: {'x-csrf-token': session.csrf_token}})).status()).toBe(204);
    const lost = await page.request.get(roundPath); expect(lost.status()).toBe(401); expect(lost.headers()['cache-control']).toBe('private, no-store');
    await page.reload(); await expect(page).toHaveURL(/\/login/); await expect(page.locator('.course-editor')).toHaveCount(0);
    expect(errors).toEqual([]); expect(unexpected).toEqual([]);
    results.push({width, saved_loading_populated: true, injected_saved_long_names: true, manual_real_save_and_reload: true, real_ui_puts: 2,
      visible_aligned_unclipped_heading_lines: true, heading_geometry: headingGeometry, empty_and_filled_rows: true, client_validation_and_first_invalid_focus: true,
      variable_9_to_18_retains_values: true, keyboard_par_index_distance_order: true, fixed_18_readonly_real_save_reload: true,
      saving_state: true, private_success_and_denial: true, injected_put_401: true, injected_denial_preserves_server_round: true,
      current_access_feedback: true, draft_retained_in_same_mounted_form: true, form_reenabled_after_denial: true, no_automatic_put_retry: true,
      injected_read_error_retry_and_empty: true, long_content_no_horizontal_overflow: true, actual_logout_private_401_login_redirect: true,
      real_catalog_unusable_fallback: true, provider_detail_requests: providerDetails, manual_hole_column_headings_visible: headingDisplay !== 'none',
      page_errors: errors, unexpected_network_failures: unexpected});
    await context.close();
  }
  console.log(JSON.stringify(results, null, 2));
} catch (error) {try {await diagnose();} catch (captureError) {console.error(captureError.message);} throw error;}
finally {await browser.close();}
