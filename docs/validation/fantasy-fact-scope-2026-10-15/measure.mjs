import { chromium, expect, request } from '../../../frontend/node_modules/@playwright/test/index.mjs';
import { execFileSync } from 'node:child_process';
import { writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { performance } from 'node:perf_hooks';
import { fixture, settle, root, ok } from './fixtures.mjs';

const cid = process.env.GOLF_A1U_CONTAINER;
if (!cid || !/^[0-9a-f]{64}$/.test(cid)) throw new Error('Owned container required');
const here = fileURLToPath(new URL('.', import.meta.url));
const psql = ['exec', '-i', cid, 'psql', '-X', '-U', 'golf', '-d', 'golf', '-v', 'ON_ERROR_STOP=1', '-At'];
const output = { baseline: 'e3ee966', profile: 'debug', samples: [] };
function sql(query, variables = {}) {
  const args = [...psql];
  for (const [key, value] of Object.entries(variables)) args.push('-v', `${key}=${value}`);
  return execFileSync('podman', args, { input: query, encoding: 'utf8' }).trim();
}
function stats() {
  const values = JSON.parse(sql(`SELECT coalesce(json_agg(s ORDER BY s.queryid),'[]'::json) FROM (
    SELECT queryid::text,query,calls,rows,total_exec_time,shared_blks_hit,shared_blks_read
    FROM pg_stat_statements WHERE dbid=(SELECT oid FROM pg_database WHERE datname=current_database())
    AND toplevel AND query NOT LIKE '%pg_stat_statements%') s;`));
  return values.map(({ queryid, query, ...value }) => ({ fingerprint: query.replace(/'(?:''|[^'])*'/g, "'?'")
    .replace(/--[^\n]*/g, '').replace(/\/\*[\s\S]*?\*\//g, '').replace(/\s+/g, ' ').trim(), ...value }));
}
function reset() { sql('SELECT pg_stat_statements_reset();'); }
function pause(ms) { return new Promise(resolve => setTimeout(resolve, ms)); }
function api(variant) { execFileSync('python3', [here + 'run.py', '--api', variant], { timeout: 60000 }); }
function materialization(f) {
  return JSON.parse(sql(`SELECT json_build_object(
    'locked_windows',(SELECT count(*) FROM fantasy_rounds WHERE tournament_id=:'t'::uuid AND locked_at IS NOT NULL),
    'locked_selections',(SELECT count(*) FROM fantasy_selections WHERE tournament_id=:'t'::uuid AND state='locked'),
    'carried_lineups',(SELECT count(*) FROM fantasy_lineups WHERE tournament_id=:'t'::uuid AND origin='carried_forward'));`, { t: f.tournament.id }));
}
async function read(f, path, expected) {
  const started = performance.now();
  const response = await ok(await f.page.request.get(path), 200);
  expect(response.headers()['cache-control']).toBe('no-store');
  const body = await response.json(), ms = performance.now() - started;
  if (expected) expect(body).toEqual(expected);
  return { body, ms, bytes: (await response.body()).length };
}
function check(rows, count, n, ownerRows) {
  expect(rows.reduce((sum, r) => sum + r.calls, 0)).toBe(25 * count);
  expect(rows.filter(r => /^(SELECT|WITH)\b/.test(r.fingerprint)).reduce((sum, r) => sum + r.calls, 0)).toBe(21 * count);
  const locks = rows.find(r => r.fingerprint === 'SELECT id FROM rounds WHERE tournament_id=$1 ORDER BY id FOR UPDATE');
  const facts = rows.find(r => r.fingerprint.startsWith('WITH target AS'));
  expect(locks?.calls).toBe(count); expect(locks?.rows).toBe(count * n);
  expect(facts?.calls).toBe(count); expect(facts?.rows).toBe(count * ownerRows);
}
async function references(f, expected, n) {
  const paths = [f.base + '/results', ...f.players.map(p => f.base + '/results/golfers/' + p),
    f.base + '/results/managers/' + f.auth.user_id];
  // Discover the other manager through an already authorized result; do not
  // publish or persist identities or bodies in diagnostic output.
  const board = await (await ok(await f.page.request.get(f.path))).json();
  paths.push(f.base + '/results/managers/' + board.managers.find(m => m.user_id !== f.auth.user_id).user_id);
  const bodies = [], controls = [];
  for (const [i, path] of paths.entries()) {
    reset();
    bodies.push((await read(f, path, expected?.[i])).body);
    if ([0, 1, 9].includes(i)) {
      const rows = stats(); check(rows, 1, n, n * 4);
      controls.push({ kind: i === 0 ? 'overall' : i === 1 ? 'golfer' : 'manager', sql: rows });
    }
  }
  return { bodies, controls };
}
async function browserSmoke(f, reference, n) {
  const errors = [], failures = [], checks = [];
  f.page.on('pageerror', () => errors.push('pageerror'));
  f.page.on('console', message => { if (message.type() === 'error' && !message.text().includes('Failed to load resource')) errors.push('console error'); });
  f.page.on('requestfailed', req => {
    if (!['net::ERR_ABORTED', 'Load request cancelled'].includes(req.failure()?.errorText)) failures.push('unexpected request failure');
  });
  f.page.on('response', response => {
    if (response.status() < 400) return;
    checks.push((async () => {
      const req = response.request();
      let code, readable = true;
      try { code = (await response.json()).error?.code; } catch { readable = false; }
      const cancelled = ['net::ERR_ABORTED', 'Load request cancelled'].includes(req.failure()?.errorText);
      expect(req.method() === 'GET' && new URL(response.url()).pathname.endsWith('/results') && response.status() === 409
        && (readable ? code === 'fantasy_conflict' : cancelled)).toBe(true);
    })().catch(() => { errors.push('unexpected response/body failure'); }));
  });
  const widths = [];
  for (const width of [320, 1280]) {
    await f.page.setViewportSize({ width, height: 900 });
    await f.page.goto(f.url);
    await f.page.getByRole('button', { name: 'Poengtavler', exact: true }).click();
    await f.page.getByLabel('Vis poeng for').selectOption(f.rounds[0].id);
    await expect(f.page.locator('.fantasy-standings > li')).toHaveCount(2);
    await f.page.getByRole('button', { name: 'Spillerpoeng', exact: true }).click();
    await expect(f.page.locator('.fantasy-standings > li')).toHaveCount(8);
    expect(await f.page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
    await read(f, f.path, reference);
    widths.push({ width, manager_rows: 2, golfer_rows: 8, overflow: false });
  }
  await f.page.goto('about:blank');
  await pause(300);
  await Promise.all(checks); expect(errors).toEqual([]); expect(failures).toEqual([]);
  const anonymous = await request.newContext({ baseURL: root });
  expect((await anonymous.get(f.path)).status()).toBe(401);
  await anonymous.dispose();
  return { rounds: n, widths, anonymous_denied: true, errors, failures };
}

const browser = await chromium.launch({ channel: 'chrome', headless: true });
try {
  // Playwright detaches Chrome from Node's process group. Retain only the exact
  // owned browser PID privately so the runner can also clean up a killed probe.
  const session = await browser.newBrowserCDPSession();
  const info = await session.send('SystemInfo.getProcessInfo');
  const browserPid = info.processInfo.find(p => p.type === 'browser')?.id;
  expect(Number.isSafeInteger(browserPid) && browserPid > 0).toBe(true);
  writeFileSync(process.env.GOLF_A1U_TEMP + '/browser.pid', String(browserPid), { mode: 0o600 });
  await session.detach();
  output.browser = browser.version();
  output.database = JSON.parse(sql("SELECT json_build_object('postgres',version(),'migrations',(SELECT count(*) FROM _sqlx_migrations WHERE success),'settings',(SELECT json_object_agg(name,setting) FROM pg_settings WHERE name IN ('shared_buffers','work_mem','jit','track_io_timing','pg_stat_statements.track','pg_stat_statements.track_planning')));"));
  for (const n of [1, 5]) {
    api('baseline');
    const context = await browser.newContext({ baseURL: root, viewport: { width: 1280, height: 900 } });
    const page = await context.newPage(), f = await fixture(page, n);
    await f.mutate(`${f.base}/rounds/${f.rounds[0].id}/lineup`, { request_id: crypto.randomUUID(),
      expected_revision: 0, picks: f.players.slice(0, 4), captain: f.players[1] }, 'PUT');
    await settle(f);
    const before = materialization(f);
    expect(before).toEqual({ locked_windows: 0, locked_selections: 0, carried_lineups: 0 });
    const dump = execFileSync('podman', ['exec', cid, 'pg_dump', '-U', 'golf', '-Fc', 'golf'], { maxBuffer: 64 * 1024 * 1024 });
    const sample = { rounds: n, blocks: [], first_reads: [], controls: [], browser: null };
    output.samples.push(sample);
    let expected, full, baselineBudgets;
    for (const [index, variant] of ['baseline', 'candidate', 'candidate', 'baseline'].entries()) {
      console.log(`Fixture ${n}: ${index + 1}/4 ${variant}`);
      if (index === 1) {
        api('stop');
        execFileSync('podman', ['exec', cid, 'dropdb', '-U', 'golf', 'golf']);
        execFileSync('podman', ['exec', cid, 'createdb', '-U', 'golf', 'golf']);
        execFileSync('podman', ['exec', '-i', cid, 'pg_restore', '-U', 'golf', '-d', 'golf', '--no-owner'], { input: dump });
        expect(materialization(f)).toEqual(before);
      }
      api(variant);
      if (index < 2) {
        reset();
        const first = await read(f, f.path, expected), firstSql = stats();
        expect(first.body.golfers).toHaveLength(8); expect(first.body.managers).toHaveLength(2);
        expect(first.body.golfers.every(g => g.holes.length === 18 && g.points.state === 'settled')).toBe(true);
        expect(first.body.managers.every(m => m.points.state === 'settled')).toBe(true);
        expected ??= first.body;
        const after = materialization(f);
        expect(after).toEqual({ locked_windows: n, locked_selections: 2 * n, carried_lineups: 2 * (n - 1) });
        sample.first_reads.push({ variant, ms: first.ms, bytes: first.bytes, before, after, sql: firstSql, body_equal: true });
      }
      for (const mode of ['none', 'top']) {
        sql(`ALTER SYSTEM SET pg_stat_statements.track='${mode}';`);
        sql('SELECT pg_reload_conf();'); await pause(150);
        for (let i = 0; i < 5; i++) await read(f, f.path, expected);
        reset();
        const times = [];
        for (let i = 0; i < 10; i++) times.push((await read(f, f.path, expected)).ms);
        const rows = stats();
        if (mode === 'none') expect(rows).toEqual([]);
        else {
          check(rows, 10, n, variant === 'baseline' ? n * 4 : 4);
          const budgets = rows.filter(r => !r.fingerprint.startsWith('WITH target AS'))
            .map(r => ({ fingerprint: r.fingerprint, calls: r.calls, rows: r.rows }))
            .sort((a, b) => a.fingerprint.localeCompare(b.fingerprint));
          baselineBudgets ??= budgets;
          expect(budgets).toEqual(baselineBudgets);
        }
        sample.blocks.push({ index, variant, mode, http_ms: times, sql: rows, body_equal: true });
      }
      const reference = await references(f, full, n);
      full ??= reference.bodies;
      sample.controls.push({ variant, index, all_bodies_equal: true, query_budgets: reference.controls });
      expect(materialization(f)).toEqual({ locked_windows: n, locked_selections: 2 * n, carried_lineups: 2 * (n - 1) });
    }
    api('candidate');
    sample.browser = await browserSmoke(f, expected, n);
    await f.other.dispose(); await context.close();
  }
  writeFileSync(here + 'measurements.json', JSON.stringify(output, null, 2) + '\n');
  console.log('A1u comparison, body parity and browser assertions passed');
} finally { await browser.close(); }
