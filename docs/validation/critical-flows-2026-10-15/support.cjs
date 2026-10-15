const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const { createRequire } = require('node:module');
const { chromium, request, expect } = createRequire(path.join(process.cwd(), 'frontend/package.json'))('@playwright/test');
const here = process.env.GOLF_CRITICAL_EVIDENCE_DIRECTORY || __dirname;
const controlPath = process.argv[3];
const state = JSON.parse(fs.readFileSync(controlPath));
const privatePath = path.join(state.directory, 'critical-private.json');
function writePrivate(value) { fs.writeFileSync(privatePath, JSON.stringify(value), { mode: 0o600 }); }
async function call(client, endpoint, data, method = 'GET', status = 200, csrf) {
  const response = await client.fetch(endpoint, { method, data, headers: csrf ? { 'x-csrf-token': csrf } : {} });
  assert.equal(response.status(), status, `API status for ${method} operation`);
  return status === 204 ? null : response.json();
}
async function login(page, username, password) {
  await page.goto('/login');
  await page.getByLabel('Brukernavn', { exact: true }).fill(username);
  await page.getByLabel('Passord', { exact: true }).fill(password);
  await page.getByRole('button', { name: 'Logg inn', exact: true }).click();
  await expect(page).not.toHaveURL(/\/login/);
  return call(page.request, '/api/auth/session');
}
async function queueRows(page) {
  return page.evaluate(() => new Promise((resolve, reject) => {
    const open = indexedDB.open('golf-pending-scores-v1', 1);
    open.onerror = () => reject(new Error('IndexedDB unavailable'));
    open.onsuccess = () => {
      const database = open.result, tx = database.transaction('pending'), rows = tx.objectStore('pending').getAll();
      rows.onsuccess = () => resolve(rows.result); rows.onerror = () => reject(new Error('Queue unreadable'));
      tx.oncomplete = () => database.close();
    };
  }));
}
async function shot(page, name, width) {
  await page.setViewportSize({ width, height: width === 320 ? 700 : 900 });
  assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), 'horizontal overflow');
  await page.screenshot({ path: path.join(here, `${name}-${width}.png`), fullPage: true });
}
async function transition(page, label) {
  const button = page.getByRole('button', { name: label, exact: true });
  await expect(button).toBeEnabled(); await button.click();
  await expect(page.getByRole('button', { name: 'Avbryt', exact: true })).toBeFocused();
  await page.getByRole('button', { name: `Bekreft og ${label.toLocaleLowerCase('nb-NO')}`, exact: true }).click();
}
async function open() {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const options = { baseURL: state.origin, ignoreHTTPSErrors: true };
  const fixture = fs.existsSync(privatePath) ? JSON.parse(fs.readFileSync(privatePath)) : {};
  const contexts = [], errors = [], pending = [], statuses = {}, failures = {};
  const f = { ...fixture, browser, options, state, contexts, errors, pending, statuses, failures, expected: [], allowLost: new Set(), acknowledgedVoid: new Set(), acknowledgedVoidAborts: {}, offlineWindow: false };
  if (fixture.checks?.completed_round_correction_reconfirms_then_locked_writes_deny_and_gross_net_preserved) {
    for (const player of fixture.players) f.expected.push({ path: `/api/rounds/${fixture.rounds[0].id}/scorecards/player/${player}/scoring`, method: 'GET', status: 403, code: 'forbidden' });
  }
  f.persist = () => writePrivate(Object.fromEntries(Object.entries(f).filter(([key]) => ['username', 'password', 'names', 'players', 'rounds', 'tid', 'invitation', 'checks', 'memberName', 'outsiderName', 'extraManagerId', 'extraManagerIds'].includes(key))));
  f.pass = name => {
    f.checks = { ...f.checks, [name]: true }; f.persist();
    const current = JSON.parse(fs.readFileSync(controlPath)); current.checks = { ...current.checks, [name]: true };
    fs.writeFileSync(controlPath, JSON.stringify(current), { mode: 0o600 });
    console.log(JSON.stringify({ check: name, passed: true }));
  };
  f.context = async (width = 390) => {
    const context = await browser.newContext({ ...options, viewport: { width, height: 900 } });
    await context.addInitScript(() => {
      window.criticalLiveEvents = [];
      const Native = EventSource;
      window.EventSource = class extends Native {
        constructor(url, options) {
          super(url, options);
          for (const name of ['open', 'error']) this.addEventListener(name, () => window.criticalLiveEvents.push({ name, time: performance.now() }));
        }
      };
    });
    contexts.push(context); const page = await context.newPage();
    page.on('pageerror', () => errors.push('pageerror'));
    page.on('console', message => { if (message.type() === 'error' && !message.text().startsWith('Failed to load resource:')) errors.push('console error'); });
    page.on('requestfailed', req => {
      const endpoint = new URL(req.url()).pathname, failure = req.failure()?.errorText || 'unknown';
      failures[failure] = (failures[failure] || 0) + 1;
      if (failure === 'net::ERR_ABORTED' && req.method() === 'POST' && f.acknowledgedVoid.has(endpoint)) {
        const task = (async () => {
          assert.equal((await req.response())?.status(), 204);
          const key = endpoint === '/api/auth/logout' ? 'POST auth_logout acknowledged204' : 'POST fantasy_entry acknowledged204';
          f.acknowledgedVoidAborts[key] = (f.acknowledgedVoidAborts[key] || 0) + 1;
        })();
        pending.push(task.catch(() => { errors.push('no-content write aborted without acknowledged204'); }));
        return;
      }
      if (failure === 'net::ERR_ABORTED' && req.method() === 'GET') return;
      if (f.allowLost.has(endpoint) && ['net::ERR_CONNECTION_FAILED', 'net::ERR_FAILED'].includes(failure)) return;
      if (f.offlineWindow && ['net::ERR_INTERNET_DISCONNECTED', 'net::ERR_NETWORK_CHANGED'].includes(failure)) return;
      errors.push('unexpected request failure: ' + req.method() + ' ' + failure + ' ' + endpoint.replace(/[a-f0-9]{8}-[a-f0-9-]{27}/g, ':id'));
    });
    page.on('response', response => {
      if (response.status() < 400) return;
      const task = (async () => {
        const endpoint = new URL(response.url()).pathname;
        if (response.status() === 401 && endpoint === '/api/auth/session') return;
        const matched = f.expected.find(value => value.status === response.status() && value.path === endpoint && value.method === response.request().method());
        const projectionConflict = response.status() === 409 && response.request().method() === 'GET' && /\/fantasy\/(?:rounds\/[^/]+\/)?results$/.test(endpoint);
        assert(matched || projectionConflict, 'unexpected HTTP error');
        const finished = await response.finished();
        if (projectionConflict && response.request().failure()?.errorText === 'net::ERR_ABORTED') return;
        if (finished) throw finished;
        const body = await response.json();
        assert.equal(body.error.code, matched ? matched.code : 'fantasy_conflict');
        statuses[response.status()] = (statuses[response.status()] || 0) + 1;
      })();
      pending.push(task.catch(() => { errors.push('HTTP body/status assertion: ' + response.status() + ' ' + new URL(response.url()).pathname.replace(/[a-f0-9]{8}-[a-f0-9-]{27}/g, ':id')); }));
    });
    return page;
  };
  f.authenticate = async () => {
    f.organizer = await f.context(1280); f.member = await f.context(390);
    f.auth = await login(f.organizer, f.username, f.password);
    f.memberAuth = await login(f.member, f.memberName, f.password);
    assert.notEqual(f.auth.user_id, f.memberAuth.user_id);
    f.mutate = (endpoint, data = {}, method = 'POST', status = 200) => call(f.organizer.request, endpoint, data, method, status, f.auth.csrf_token);
    f.memberMutate = (endpoint, data = {}, method = 'POST', status = 200) => call(f.member.request, endpoint, data, method, status, f.memberAuth.csrf_token);
  };
  f.finish = async phase => {
    await Promise.all(pending); assert.deepEqual(errors, []);
    f.pass(phase + '_no_unexpected_console_network_or_http_body_failures');
    fs.writeFileSync(path.join(here, phase + '.json'), JSON.stringify({ browser: await browser.version(), images: state.images, checks: f.checks,
      expected_http_counts: statuses, request_failure_classes: failures, acknowledged_204_abort_records_by_route: f.acknowledgedVoidAborts, certificate_exception: 'localhost disposable contexts only' }, null, 2) + '\n');
  };
  f.close = async () => { await Promise.all(contexts.map(context => context.close())); await browser.close(); };
  return f;
}
module.exports = { fs, path, assert, request, expect, state, here, privatePath, call, login, queueRows, shot, transition, open };
