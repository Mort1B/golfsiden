import { chromium, expect, request } from '../../../frontend/node_modules/@playwright/test/index.mjs';
import { execFileSync, spawn } from 'node:child_process';
import { writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { performance } from 'node:perf_hooks';

const cid = process.env.GOLF_A1T_CONTAINER;
if (!cid || !/^[0-9a-f]{64}$/.test(cid)) throw new Error('Owned container required');
const root = 'http://127.0.0.1:5173';
const output = { baseline: 'b6a81a3', fixture: { golfers: 8, teams_per_round: 4, holes: 18, managers: 2 }, samples: [] };
const psqlArgs = ['exec', '-i', cid, 'psql', '-X', '-U', 'golf', '-d', 'golf', '-v', 'ON_ERROR_STOP=1', '-At'];
function sql(query, variables = {}) {
  const args = [...psqlArgs];
  for (const [key, value] of Object.entries(variables)) args.push('-v', `${key}=${value}`);
  // stdin permits psql's typed/quoted variables; values never form SQL text.
  return execFileSync('podman', args, { input: query, encoding: 'utf8' }).trim();
}
function jsonSql(query, variables) { return JSON.parse(sql(query, variables)); }
function reset() { sql('SELECT pg_stat_statements_reset();'); }
function stats() {
  const rows = jsonSql(`SELECT coalesce(json_agg(s ORDER BY s.queryid),'[]'::json) FROM (
    SELECT queryid::text,query,calls,rows,total_exec_time,shared_blks_hit,shared_blks_read,
      shared_blk_read_time,shared_blk_write_time FROM pg_stat_statements
    WHERE dbid=(SELECT oid FROM pg_database WHERE datname=current_database())
      AND toplevel AND query NOT LIKE '%pg_stat_statements%'
      AND query NOT LIKE '%pg_stat_activity%' AND query NOT LIKE '%pg_blocking_pids%'
    ) s;`);
  return rows.map(({ queryid, query, ...row }) => ({
    fingerprint: query.replace(/'(?:''|[^'])*'/g, "'?'").replace(/--[^\n]*/g, '')
      .replace(/\/\*[\s\S]*?\*\//g, '').replace(/\s+/g, ' ').trim(), ...row,
  }));
}
function track(mode) {
  if (!['top', 'none'].includes(mode)) throw new Error('Invalid tracking mode');
  sql(`ALTER SYSTEM SET pg_stat_statements.track = '${mode}';`);
  sql('SELECT pg_reload_conf();');
}
async function pause(ms) { await new Promise(resolve => setTimeout(resolve, ms)); }
function lifetime(page) {
  const active = new Set(), live = new Set();
  let changed = performance.now();
  page.on('request', req => {
    const path = new URL(req.url()).pathname;
    if (path.startsWith('/api/') && !path.endsWith('/live')) { active.add(req); changed = performance.now(); }
  });
  page.on('response', res => { if (new URL(res.url()).pathname.endsWith('/live') && res.status() === 200) live.add(res.request()); });
  const done = req => { live.delete(req); if (active.delete(req)) changed = performance.now(); };
  page.on('requestfinished', done); page.on('requestfailed', done);
  return {
    pending: () => active.size, live: () => live.size,
    quiet: async () => {
      const limit = performance.now() + 10000;
      while (active.size || performance.now() - changed < 500) {
        if (performance.now() > limit) throw new Error('Finite requests did not quiesce');
        await pause(20);
      }
    },
  };
}
async function ok(response, expected) {
  if (expected === undefined) expect(response.ok()).toBe(true);
  else expect(response.status()).toBe(expected);
  return response;
}
function headers(response) { expect(response.headers()['cache-control']).toContain('no-store'); }
async function fixture(page, n) {
  const day = new Date(Date.now() + 86400000).toISOString().slice(0, 10);
  const names = Array.from({ length: 8 }, (_, i) => `Synthetic golfer ${i + 1}`);
  const created = await ok(await page.request.post('/api/onboarding/tournaments', { data: {
    creator: { account: { username: `a1t_${crypto.randomUUID().replaceAll('-', '').slice(0, 20)}`, password: crypto.randomUUID() },
      player: { display_name: names[0], handicap_index: 0 } },
    tournament: { name: 'A1t synthetic fixture', description: '', start_date: day, end_date: day,
      counted_rounds: 1, mandatory_round_number: null },
    rounds: Array.from({ length: n }, (_, i) => ({ round_number: i + 1, name: `Round ${i + 1}`,
      round_date: day, scoring_format: 'team_scramble' })),
  } }), 201);
  const { session: auth, tournament } = await created.json();
  const mutate = async (path, data = {}, method = 'POST') => ok(await page.request.fetch(path,
    { method, data, headers: { 'x-csrf-token': auth.csrf_token } }));
  const players = [auth.player_id];
  for (let i = 1; i < 8; i++) players.push((await (await mutate(`/api/tournaments/${tournament.id}/players`,
    { display_name: names[i], handicap_index: 0 })).json()).player_id);
  const rounds = await (await ok(await page.request.get(`/api/tournaments/${tournament.id}/rounds`))).json();
  const base = `/api/tournaments/${tournament.id}/fantasy`;
  await mutate(base, { enabled: true }, 'PUT');
  await mutate(`${base}/entry`);
  const teams = [];
  for (const r of rounds) {
    const configured = await (await mutate(`/api/rounds/${r.id}/course-configuration`, {
      expected_round_updated_at: r.updated_at, selection: { source: 'manual', course_name: 'Synthetic course',
        location: null, tee: { category: 'male', name: 'Synthetic tee', course_rating: 72, slope_rating: 113,
          holes: Array.from({ length: 18 }, (_, i) => ({ par: 4, stroke_index: i + 1, distance: null })) } },
    }, 'PUT')).json();
    const pairs = Array.from({ length: 4 }, (_, i) => ({ id: crypto.randomUUID(), name: `Team ${i + 1}`,
      members: players.slice(i * 2, i * 2 + 2).map(player_id => ({ player_id })), schedule_flight_id: null }));
    await mutate(`/api/rounds/${r.id}/pairings`, { expected_round_updated_at: configured.updated_at, teams: pairs,
      flights: [0, 1].map(i => ({ id: crypto.randomUUID(), name: `Flight ${i + 1}`, starting_hole: 1, tee_time: null,
        members: players.slice(i * 4, i * 4 + 4).map(player_id => ({ player_id })) })), legacy_conversions: [] }, 'PUT');
    teams.push(pairs);
  }
  const claim = await (await mutate(`/api/tournaments/${tournament.id}/players/${players[1]}/claim`)).json();
  const other = await request.newContext({ baseURL: root });
  const registered = await (await ok(await other.post(`/api/player-claims/${claim.claim_id}/register`, { data: {
    token: claim.token, account: { username: `a1t_${crypto.randomUUID().replaceAll('-', '').slice(0, 20)}`, password: crypto.randomUUID() },
  } }), 201)).json();
  await ok(await other.post(`${base}/entry`, { headers: { 'x-csrf-token': registered.session.csrf_token } }), 204);
  await ok(await other.put(`${base}/rounds/${rounds[0].id}/lineup`, { headers: { 'x-csrf-token': registered.session.csrf_token },
    data: { request_id: crypto.randomUUID(), expected_revision: 0, picks: players.slice(4, 8), captain: players[5] } }));
  return { page, auth, tournament, names, players, rounds, teams, base, mutate, other,
    path: `${base}/rounds/${rounds[0].id}/results`, url: `/tournaments/${tournament.id}/fantasy` };
}
function normalize(path, f) {
  let value = path.replaceAll(f.tournament.id, ':tournament');
  f.rounds.forEach((r, i) => { value = value.replaceAll(r.id, `:round${i + 1}`); });
  return value.replace(/[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}/gi, ':id');
}
function observer(page, f) {
  const entries = [], map = new Map(), checks = [], errors = [];
  const onRequest = req => {
    const path = new URL(req.url()).pathname;
    if (!path.startsWith('/api/')) return;
    const row = { method: req.method(), path: normalize(path, f), state: 'pending', start: performance.now() };
    map.set(req, row); entries.push(row);
  };
  const onResponse = res => {
    const row = map.get(res.request());
    if (!row) return;
    row.status = res.status();
    if (res.status() >= 400) checks.push((async () => {
      try { row.code = (await res.json()).error?.code; }
      catch { row.body_unreadable = true; }
    })());
  };
  const finish = (req, state) => {
    const row = map.get(req);
    if (!row) return;
    row.state = state; row.ms = performance.now() - row.start;
    if (state === 'failed') row.failure = req.failure()?.errorText;
  };
  const onFinish = req => finish(req, 'completed');
  const onFail = req => finish(req, 'failed');
  page.on('request', onRequest); page.on('response', onResponse);
  page.on('requestfinished', onFinish); page.on('requestfailed', onFail);
  return async () => {
    page.off('request', onRequest); page.off('response', onResponse);
    page.off('requestfinished', onFinish); page.off('requestfailed', onFail);
    await Promise.all(checks);
    for (const row of entries) {
      delete row.start;
      if (row.state === 'failed') {
        const aborted = ['net::ERR_ABORTED', 'Load request cancelled'].includes(row.failure);
        expect(aborted && row.method === 'GET').toBe(true);
        row.state = 'cancelled';
      }
      if (row.status >= 400) {
        const conflict = row.method === 'GET' && row.status === 409 && row.path.endsWith('/results');
        expect(conflict && (row.code === 'fantasy_conflict' || row.body_unreadable && row.state === 'cancelled')).toBe(true);
      }
      if (row.state === 'pending' && !row.path.endsWith('/live')) errors.push('finite request pending at cutoff');
    }
    expect(errors).toEqual([]);
    return entries;
  };
}
function receiptCount(f) {
  return Number(sql("SELECT count(*) FROM fantasy_lineups WHERE tournament_id=:'t'::uuid AND user_id=:'u'::uuid AND origin='submitted';",
    { t: f.tournament.id, u: f.auth.user_id }));
}
function materializationState(f) {
  return jsonSql(`SELECT json_build_object(
    'windows',(SELECT count(*) FROM fantasy_rounds WHERE tournament_id=:'t'::uuid),
    'locked_windows',(SELECT count(*) FROM fantasy_rounds WHERE tournament_id=:'t'::uuid AND locked_at IS NOT NULL),
    'selections',(SELECT count(*) FROM fantasy_selections WHERE tournament_id=:'t'::uuid),
    'locked_selections',(SELECT count(*) FROM fantasy_selections WHERE tournament_id=:'t'::uuid AND state='locked'),
    'carried_lineups',(SELECT count(*) FROM fantasy_lineups WHERE tournament_id=:'t'::uuid AND origin='carried_forward'));`,
  { t: f.tournament.id });
}
async function save(f, revision, mode) {
  const { page } = f;
  if (revision > 1) await page.getByRole('button', { name: 'Endre valg', exact: true }).click();
  else for (const name of f.names.slice(0, 4)) await page.getByRole('checkbox', { name, exact: true }).check();
  await page.getByLabel('Kaptein · doble poeng').selectOption(f.players[revision % 2]);
  const before = receiptCount(f);
  reset();
  const end = mode === 'full' ? observer(page, f) : null;
  const started = performance.now();
  await page.getByRole('button', { name: 'Lagre firer og kaptein', exact: true }).click();
  await expect(page.getByText(`Lagret lag · revisjon ${revision}`, { exact: true })).toBeVisible();
  const receipt_ms = performance.now() - started;
  await expect(page.getByRole('button', { name: 'Endre valg', exact: true })).toBeEnabled();
  await pause(1000);
  const pending_at_tail_cutoff = f.life.pending();
  await f.life.quiet();
  expect(f.life.live()).toBe(1);
  const sqlRows = stats();
  const requests = end ? await end() : null;
  expect(receiptCount(f) - before).toBe(1);
  if (requests) {
    const writes = requests.filter(r => r.method === 'PUT');
    expect(writes).toHaveLength(1); expect(writes[0].status).toBe(200); expect(writes[0].state).toBe('completed');
  }
  return { mode, revision, receipt_ms, tail_ms: 1000, pending_at_tail_cutoff, open_sse: f.life.live(),
    request_window: 'click through quiet barrier after receipt plus tail',
    quiet_barrier_passed: true, requests, sql: sqlRows, accepted_receipts: 1 };
}
async function settle(f) {
  const fresh = await (await ok(await f.page.request.get(`/api/tournaments/${f.tournament.id}`))).json();
  await f.mutate(`/api/tournaments/${f.tournament.id}/start`, { expected_tournament_updated_at: fresh.updated_at });
  for (const [i, r] of f.rounds.entries()) {
    await f.mutate(`/api/rounds/${r.id}/open`);
    for (const [j, team] of f.teams[i].entries()) {
      const path = `/api/rounds/${r.id}/scorecards/team/${team.id}`;
      const card = await (await ok(await f.page.request.get(`${path}/scoring`))).json();
      for (const [h, hole] of card.holes.entries()) await f.mutate(`/api/rounds/${r.id}/scores`,
        { owner: { type: 'team', id: team.id }, hole_id: hole.hole_id, gross_strokes: h === 0 ? 4 + j : 4 }, 'PUT');
      await f.mutate(`${path}/confirm`);
    }
    await f.mutate(`/api/rounds/${r.id}/complete`);
  }
}
async function read(f, expected) {
  const started = performance.now();
  const res = await ok(await f.page.request.get(f.path), 200);
  headers(res);
  const body = await res.json(), ms = performance.now() - started;
  if (expected) expect(body).toEqual(expected);
  return { body, ms, bytes: (await res.body()).length };
}
async function lockRead(f, expected) {
  // The only held-row value is passed through psql's quoted variable.
  const locker = spawn('podman', [...psqlArgs, '-v', `r=${f.rounds[0].id}`], { stdio: ['pipe', 'pipe', 'pipe'] });
  let buffer = '';
  const ready = new Promise((resolve, reject) => {
    locker.stdout.on('data', chunk => { buffer += chunk.toString(); if (buffer.includes('LOCK_READY')) resolve(); });
    locker.on('error', reject); locker.on('exit', code => { if (!buffer.includes('LOCK_READY')) reject(new Error(`Lock helper exit ${code}`)); });
  });
  const exited = new Promise(resolve => locker.on('exit', resolve));
  locker.stdin.write("BEGIN;\nSELECT id FROM rounds WHERE id=:'r'::uuid FOR UPDATE;\n\\echo LOCK_READY\n");
  try {
    await Promise.race([ready, pause(5000).then(() => { throw new Error('Lock helper timeout'); })]);
    const pending = read(f, expected);
    let blocked = false;
    for (let i = 0; i < 40 && !blocked; i++) {
      blocked = Number(sql("SELECT count(*) FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND cardinality(pg_blocking_pids(pid))>0;")) > 0;
      if (!blocked) await pause(5);
    }
    expect(blocked).toBe(true);
    locker.stdin.end('SELECT pg_sleep(0.2);\nROLLBACK;\n');
    const result = await pending;
    expect(await exited).toBe(0);
    return { blocked_observed: blocked, hold_after_observation_ms: 200, http_ms: result.ms };
  } finally { if (locker.exitCode === null) locker.stdin.end('ROLLBACK;\n'); }
}

const browser = await chromium.launch({ channel: 'chrome', headless: true });
try {
  output.environment = jsonSql("SELECT json_build_object('postgres',version(),'migrations',(SELECT count(*) FROM _sqlx_migrations WHERE success),'settings',(SELECT json_object_agg(name,setting) FROM pg_settings WHERE name IN ('shared_buffers','work_mem','jit','track_io_timing','pg_stat_statements.track','pg_stat_statements.track_planning')));");
  output.environment.browser = browser.version();
  const anonymous = await request.newContext({ baseURL: root });
  const outsider = await request.newContext({ baseURL: root });
  const day = new Date(Date.now() + 86400000).toISOString().slice(0, 10);
  await ok(await outsider.post('/api/onboarding/tournaments', { data: {
    creator: { account: { username: `a1t_${crypto.randomUUID().replaceAll('-', '').slice(0, 20)}`, password: crypto.randomUUID() }, player: { display_name: 'Synthetic outsider', handicap_index: 0 } },
    tournament: { name: 'Synthetic outsider fixture', description: '', start_date: day, end_date: day, counted_rounds: 1, mandatory_round_number: null },
    rounds: [{ round_number: 1, name: 'Round', round_date: day, scoring_format: 'individual_stroke_play' }],
  } }), 201);
  for (const n of [1, 5]) {
    const context = await browser.newContext({ baseURL: root, viewport: { width: 1280, height: 900 } });
    const page = await context.newPage(), browserErrors = [];
    const life = lifetime(page);
    page.on('pageerror', () => browserErrors.push('pageerror'));
    page.on('console', msg => { if (msg.type() === 'error' && !msg.text().includes('Failed to load resource')) browserErrors.push('console error'); });
    const f = await fixture(page, n);
    f.life = life;
    console.log(`Fixture ${n}: editable saves`);
    const result = { rounds: n, saves: [], reads: [], lock_reads: [] };
    output.samples.push(result);
    await page.goto(f.url);
    await expect(page.getByRole('checkbox', { name: f.names[0], exact: true })).toBeVisible();
    await pause(2000);
    await expect.poll(() => life.live(), { timeout: 25000 }).toBe(1);
    await life.quiet();
    result.warmup = await save(f, 1, 'minimal');
    const idleEnd = observer(page, f);
    await pause(3000);
    expect(life.live()).toBe(1);
    result.idle = { duration_ms: 3000, open_sse: life.live(), requests: await idleEnd() };
    for (let i = 0; i < 10; i++) result.saves.push(await save(f, i + 2, i % 2 === 0 ? 'full' : 'minimal'));
    const otherRound = await (await ok(await f.other.get(`${f.base}/rounds/${f.rounds[0].id}`))).json();
    expect(otherRound.selections.some(s => s.user_id === f.auth.user_id)).toBe(false);
    const otherResult = await (await ok(await f.other.get(f.path))).json();
    expect(otherResult.managers.find(m => m.user_id === f.auth.user_id).lineup).toBeNull();
    expect((await anonymous.get(f.path)).status()).toBe(401);
    expect([403, 404]).toContain((await outsider.get(f.path)).status());
    result.gates = { anonymous_denied: true, nonmember_denied: true, prelock_other_picks_absent: true };
    await page.goto('about:blank');
    await pause(300);
    console.log(`Fixture ${n}: populate sporting facts`);
    await settle(f);
    const before = materializationState(f);
    reset();
    const first = await read(f);
    const firstSql = stats(), after = materializationState(f);
    expect(before.locked_windows).toBe(0); expect(after.locked_windows).toBe(n);
    expect(after.locked_selections).toBe(2 * n); expect(after.carried_lineups).toBe(2 * (n - 1));
    result.first_observed_settled_read = { ms: first.ms, bytes: first.bytes,
      materialization_before: before, materialization_after: after, sql: firstSql };
    expect(first.body.golfers).toHaveLength(8); expect(first.body.managers).toHaveLength(2);
    expect(first.body.golfers.every(g => g.holes.length === 18 && g.points.state === 'settled')).toBe(true);
    expect(first.body.managers.every(m => m.points.state === 'settled')).toBe(true);
    for (let i = 0; i < 5; i++) await read(f, first.body);
    console.log(`Fixture ${n}: isolated reads and lock controls`);
    for (const mode of ['none', 'top', 'top', 'none']) {
      track(mode); await pause(150);
      await read(f, first.body); // reload acknowledgement/warmup, outside block
      reset();
      const times = [];
      for (let i = 0; i < 10; i++) times.push((await read(f, first.body)).ms);
      const rows = stats();
      if (mode === 'none') expect(rows).toEqual([]);
      else {
        const locks = rows.filter(row => row.fingerprint === 'SELECT id FROM rounds WHERE tournament_id=$1 ORDER BY id FOR UPDATE');
        expect(locks).toHaveLength(1); expect(locks[0].calls).toBe(10); expect(locks[0].rows).toBe(10 * n);
        expect(rows.filter(row => /^SELECT\b|^WITH\b/i.test(row.fingerprint)).reduce((sum, row) => sum + row.calls, 0)).toBe(210);
        const facts = rows.find(row => row.fingerprint.startsWith('WITH target AS'));
        expect(facts?.calls).toBe(10); expect(facts?.rows).toBe(40 * n);
      }
      result.reads.push({ mode, count: 10, http_ms: times, sql: rows });
    }
    track('top'); await pause(150);
    for (let i = 0; i < 3; i++) result.lock_reads.push(await lockRead(f, first.body));
    expect(materializationState(f)).toEqual(after);
    result.parity = { all_bodies_equal: true, cache_control_no_store: true, settled_golfers: 8, settled_managers: 2, holes_each: 18 };
    expect(browserErrors).toEqual([]); result.browser_errors = browserErrors;
    await f.other.dispose(); await context.close();
  }
  await anonymous.dispose(); await outsider.dispose();
  writeFileSync(fileURLToPath(new URL('./measurements.json', import.meta.url)), JSON.stringify(output, null, 2) + '\n');
  console.log('A1t measurements and all assertions completed');
} finally { await browser.close(); }
