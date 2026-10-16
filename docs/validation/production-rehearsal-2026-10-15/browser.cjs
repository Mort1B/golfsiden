// Real Chrome, two distinct accounts, actual image/Caddy/API/PG boundary.
// Credentials, cookies, IDs and raw failures remain in the private fixture only.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const assert = require('node:assert/strict');
const { createRequire } = require('node:module');
const requireFrontend = createRequire(path.join(process.cwd(), 'frontend/package.json'));
const { chromium, request, expect } = requireFrontend('@playwright/test');
const controlPath = process.argv[2];
const state = JSON.parse(fs.readFileSync(controlPath));
const root = state.directory;
const origin = state.origin;
const here = process.env.GOLF_PRODUCTION_EVIDENCE_DIRECTORY || __dirname;
const checks = {};
const errors = [];
let stage = 'launch';
function passed(name) { checks[name] = true; console.log(JSON.stringify({ check: name, passed: true })); }
async function call(client, endpoint, data, method = 'GET', status = 200, csrf) {
  const result = await client.fetch(endpoint, { method, data, headers: csrf ? { 'x-csrf-token': csrf } : {} });
  assert.equal(result.status(), status, 'fixture API status');
  return status === 204 ? null : result.json();
}
function observe(page) {
  page.on('pageerror', () => errors.push('pageerror'));
  page.on('console', message => {
    if (message.type() === 'error' && !message.text().startsWith('Failed to load resource:')) errors.push('console');
  });
  page.on('response', response => {
    const url = new URL(response.url());
    if (response.status() >= 400 && !(response.status() === 401 && ['/api/auth/session', '/api/auth/login'].includes(url.pathname))) errors.push('http ' + response.status());
  });
  page.on('requestfailed', req => {
    if (req.failure()?.errorText !== 'net::ERR_ABORTED') errors.push('requestfailed');
  });
}
async function screenshot(page, name, width) {
  await page.setViewportSize({ width, height: width < 600 ? 844 : 900 });
  await expect.poll(() => page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.screenshot({ path: path.join(here, `${name}-${width}.png`), fullPage: true });
}
async function login(page, username, password) {
  await page.goto('/login');
  await page.getByLabel('Brukernavn', { exact: true }).fill(username);
  await page.getByLabel('Passord', { exact: true }).fill(password);
  await page.getByRole('button', { name: 'Logg inn', exact: true }).click();
  await expect(page).not.toHaveURL(/\/login/);
  const session = await call(page.request, '/api/auth/session');
  const cookies = await page.context().cookies(origin + '/api/auth/session');
  const cookie = cookies.find(value => value.httpOnly);
  assert(cookie && cookie.secure && cookie.sameSite === 'Lax' && cookie.path === '/api' && cookie.domain === 'localhost');
  return session;
}
async function main() {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const clientOptions = { baseURL: origin, ignoreHTTPSErrors: true };
  // Certificate exception is scoped to disposable contexts and local Caddy CA.
  const setup = await request.newContext(clientOptions);
  const registration = await request.newContext(clientOptions);
  const contexts = [];
  try {
    stage = 'synthetic accounts';
    const stamp = crypto.randomBytes(6).toString('hex');
    const password = crypto.randomBytes(24).toString('base64url');
    const organizerName = 'deploy_org_' + stamp, playerName = 'deploy_player_' + stamp;
    const display = 'Arrangør med et svært langt navn for produksjonsprøven';
    const day = new Date(Date.now() + 86400000).toISOString().slice(0, 10);
    const body = await call(setup, '/api/onboarding/tournaments', {
      creator: { account: { username: organizerName, password }, player: { display_name: display, handicap_index: 12 } },
      tournament: { name: 'Isolert produksjonsprøve med et langt turneringsnavn', description: '', start_date: day, end_date: day, counted_rounds: 1, mandatory_round_number: null },
      rounds: [{ round_number: 1, name: 'Runde med et langt navn', round_date: day, scoring_format: 'individual_stroke_play' }],
    }, 'POST', 201);
    const member = await call(registration, `/api/invitations/${body.invitation.id}/register`, {
      token: body.invitation.token, account: { username: playerName, password },
      player: { display_name: 'Spiller i separat nettleserøkt', handicap_index: 18 },
    }, 'POST', 201);
    const tid = body.tournament.id, first = body.rounds[0], rid = first.id;
    const owner = { type: 'player', id: body.session.player_id };
    const mutate = (endpoint, data = {}, method = 'POST', status = 200) => call(setup, endpoint, data, method, status, body.session.csrf_token);
    const round = await mutate(`/api/rounds/${rid}/course-configuration`, {
      expected_round_updated_at: first.updated_at, selection: { source: 'manual', course_name: 'Produksjonsprøvens bane', location: null,
        tee: { category: 'male', name: 'Gul', course_rating: 72, slope_rating: 113, holes: Array.from({ length: 18 }, (_, i) => ({ par: 4, stroke_index: i + 1, distance: null })) } },
    }, 'PUT');
    await mutate(`/api/rounds/${rid}/pairings`, { expected_round_updated_at: round.updated_at, teams: [], flights: [{
      id: crypto.randomUUID(), name: 'Flight 1', starting_hole: 1, tee_time: null,
      members: [{ player_id: owner.id }, { player_id: member.player_id }],
    }], legacy_conversions: [] }, 'PUT');
    const fresh = await call(setup, `/api/tournaments/${tid}`);
    await mutate(`/api/tournaments/${tid}/start`, { expected_tournament_updated_at: fresh.updated_at });
    await mutate(`/api/rounds/${rid}/open`);
    const cardPath = `/api/rounds/${rid}/scorecards/player/${owner.id}/scoring`;
    const card = await call(setup, cardPath);
    passed('real_runtime_dml_onboarding_invitation_course_round');
    stage = 'real organizer and player UI login';
    const organizerContext = await browser.newContext({ ...clientOptions, viewport: { width: 1280, height: 900 } });
    const playerContext = await browser.newContext({ ...clientOptions, viewport: { width: 390, height: 844 } });
    contexts.push(organizerContext, playerContext);
    const organizer = await organizerContext.newPage(), player = await playerContext.newPage();
    observe(organizer); observe(player);
    await organizer.goto('/login');
    await screenshot(organizer, 'login', 320);
    await screenshot(organizer, 'login', 1280);
    await organizer.getByLabel('Brukernavn', { exact: true }).fill(organizerName);
    await organizer.getByLabel('Passord', { exact: true }).fill('incorrect-rehearsal-password');
    await organizer.getByRole('button', { name: 'Logg inn', exact: true }).click();
    await expect(organizer.getByRole('alert')).toBeVisible();
    await organizer.getByLabel('Brukernavn', { exact: true }).fill('');
    await organizer.getByLabel('Passord', { exact: true }).fill('');
    await screenshot(organizer, 'login-error', 390);
    const organizerAuth = await login(organizer, organizerName, password);
    const playerAuth = await login(player, playerName, password);
    assert.notEqual(organizerAuth.user_id, playerAuth.user_id);
    assert.equal(organizerAuth.player_id, owner.id);
    assert.equal(playerAuth.player_id, member.player_id);
    passed('distinct_organizer_player_ui_login_secure_host_only_api_cookies');
    const unauthorized = await organizer.request.put(`/api/rounds/${rid}/scores`, { data: { owner, hole_id: card.holes[0].hole_id, gross_strokes: 4 } });
    assert.equal(unauthorized.status(), 403);
    assert.equal((await call(setup, cardPath)).holes_scored, 0);
    passed('csrf_absent_rejected_without_score_write');
    stage = 'native same-origin SSE';
    await player.addInitScript(() => {
      const Native = EventSource;
      window.EventSource = class extends Native {
        constructor(url, options) {
          super(url, options);
          this.addEventListener('open', () => { document.documentElement.dataset.deployLiveOpen = '1'; });
          this.addEventListener('score', () => { document.documentElement.dataset.deployScoreEvents = String(Number(document.documentElement.dataset.deployScoreEvents || '0') + 1); });
        }
      };
    });
    const liveOrigins = [];
    player.on('request', req => { if (new URL(req.url()).pathname.endsWith('/live')) liveOrigins.push(new URL(req.url()).origin); });
    await player.goto(`/leaderboard?tournament=${tid}&round=${rid}&scope=round&metric=gross`);
    await expect(player.locator('.leaderboard-row')).toHaveCount(2);
    await expect(player.locator('.leaderboard-row-link')).toHaveCount(0);
    await expect(player.getByText('Ingen score', { exact: true })).toHaveCount(2);
    await screenshot(player, 'leaderboard-empty', 390);
    await expect.poll(() => player.evaluate(() => document.documentElement.dataset.deployLiveOpen), { timeout: 30000 }).toBe('1');
    await mutate(`/api/rounds/${rid}/scores`, { owner, hole_id: card.holes[1].hole_id, gross_strokes: 5 }, 'PUT');
    const row = player.locator('.leaderboard-row-link').filter({ hasText: display });
    await expect(row.locator('.leaderboard-score span')).toHaveText('5 brutto');
    const before = await player.evaluate(() => Number(document.documentElement.dataset.deployScoreEvents || '0'));
    const response = await organizer.goto(`/score?${new URLSearchParams({ tournament: tid, round: rid, owner_type: 'player', owner: owner.id, hole: '1', view: 'hole' })}`);
    const headers = response.headers();
    assert(headers['content-security-policy'].includes("script-src 'self'"));
    assert(headers['strict-transport-security'].includes('max-age='));
    assert.equal(headers['x-content-type-options'], 'nosniff');
    assert.equal(headers['x-frame-options'], 'DENY');
    assert(!headers.server);
    await expect(organizer.getByRole('button', { name: /Registrer par/ })).toBeEnabled();
    await organizer.getByRole('button', { name: /Registrer par/ }).click();
    await expect(organizer.getByText('Lagret på serveren', { exact: true })).toBeVisible();
    await expect.poll(async () => (await call(setup, cardPath)).holes[0].score?.gross_strokes).toBe(4);
    await expect.poll(() => player.evaluate(() => Number(document.documentElement.dataset.deployScoreEvents || '0'))).toBeGreaterThan(before);
    await expect(row.locator('.leaderboard-score span')).toHaveText('9 brutto');
    // The distinct player receives the default final-round front-nine projection.
    await expect(row.locator('.leaderboard-progress')).toContainText('2 av 9 hull');
    assert(liveOrigins.length > 0 && liveOrigins.every(value => value === origin));
    await screenshot(organizer, 'score-saved', 320);
    await screenshot(player, 'leaderboard-live', 390);
    await screenshot(player, 'leaderboard-live', 1280);
    await player.reload();
    await expect(row.locator('.leaderboard-score span')).toHaveText('9 brutto');
    passed('native_sse_same_https_origin_distinct_player_observes_5_to_9_and_reload');
    passed('security_headers_and_mobile_desktop_long_content_no_overflow');
    assert.deepEqual(errors, []);
    passed('no_unexpected_browser_console_network_or_response_failures');
    stage = 'forged proxy identity quota';
    const anonymous = await request.newContext(clientOptions);
    try {
      const statuses = [];
      for (let i = 0; i < 11; i++) {
        const result = await anonymous.post('/api/auth/login', { data: { username: 'missing_' + stamp, password }, headers: {
          'x-forwarded-for': `198.51.100.${i + 1}`, forwarded: `for=198.51.100.${i + 1}`,
          'x-golf-client-ip': `203.0.113.${i + 1}`, 'x-golf-proxy-secret': 'forged-secret',
        } });
        statuses.push(result.status());
        if (i === 10) assert(Number(result.headers()['retry-after']) > 0);
      }
      assert.deepEqual(statuses, [...Array(10).fill(401), 429]);
      passed('forged_forwarding_and_internal_headers_do_not_evade_login_quota');
    } finally { await anonymous.dispose(); }
    state.checks = { ...state.checks, ...checks };
    fs.writeFileSync(controlPath, JSON.stringify(state), { mode: 0o600 });
    fs.writeFileSync(path.join(here, 'browser.json'), JSON.stringify({ browser: await browser.version(), images: state.images, checks, widths: [320, 390, 1280], accounts: 2, certificate_exception: 'disposable contexts for localhost only', unexpected_failures: errors.length }, null, 2) + '\n');
  } finally {
    await Promise.all(contexts.map(context => context.close()));
    await setup.dispose(); await registration.dispose(); await browser.close();
  }
}
main().catch(error => {
  fs.writeFileSync(path.join(root, 'browser-failure.log'), String(error.stack), { mode: 0o600 });
  console.error(JSON.stringify({ failed: error.name, stage, detail: 'private diagnostic only' }));
  process.exitCode = 1;
});
