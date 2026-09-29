// Requires the isolated golf-score-read-20260929 container and seeded golf_browser
// database, the repaired API on 3000 and Vite on 5173. Never target a hosted site.
import { chromium, expect } from '../../../frontend/node_modules/@playwright/test/index.mjs'
import { spawn } from 'node:child_process'
import { mkdir, writeFile } from 'node:fs/promises'

const artifacts = '/tmp/golf-score-read-20260929'
await mkdir(artifacts, { recursive: true })
function postgres(variables = {}) {
  const args = ['exec', '-i', 'golf-score-read-20260929', 'psql', '-XqAt', '-v', 'ON_ERROR_STOP=1', '-U', 'golf', '-d', 'golf_browser']
  for (const [key, value] of Object.entries(variables)) args.push('-v', `${key}=${value}`)
  const child = spawn('podman', args, { stdio: ['pipe', 'pipe', 'pipe'] })
  let output = '', error = ''
  child.stdout.on('data', chunk => { output += chunk })
  child.stderr.on('data', chunk => { error += chunk })
  return {
    async command(sql) {
      output = ''
      child.stdin.write(`${sql}\n\\echo READ_TEST_READY\n`)
      await expect.poll(() => {
        if (child.exitCode !== null) throw new Error(`Local PostgreSQL control exited: ${error}`)
        return output.includes('READ_TEST_READY')
      }, { timeout: 10000 }).toBe(true)
      return output.replace('READ_TEST_READY', '').trim()
    },
    close() { child.stdin.end() },
  }
}
async function read(page, path) {
  const response = await page.request.get(path)
  expect(response.ok(), `${path}: ${response.status()}`).toBe(true)
  return response.json()
}
async function mutate(page, path, csrf, data = {}) {
  const response = await page.request.post(path, { headers: { 'x-csrf-token': csrf }, data })
  expect(response.ok(), `${path}: ${response.status()}`).toBe(true)
}
async function login(page) {
  await page.goto('/login')
  await page.getByLabel('Brukernavn', { exact: true }).fill('admin')
  await page.getByLabel('Passord', { exact: true }).fill('golf-dev-2026')
  await page.getByRole('button', { name: 'Logg inn', exact: true }).click()
  await expect(page).not.toHaveURL(/\/login/)
  return read(page, '/api/auth/session')
}
const browser = await chromium.launch({ channel: 'chrome', headless: true })
const checks = []
try {
  for (const [change, width] of [['valid', 390], ['logout', 320], ['membership', 1280]]) {
    const context = await browser.newContext({ baseURL: 'http://127.0.0.1:5173', viewport: { width, height: 900 } })
    const page = await context.newPage()
    page.setDefaultTimeout(15000)
    const errors = []
    page.on('pageerror', error => errors.push(error.message))
    page.on('console', message => {
      if (message.type() === 'error' && !message.text().startsWith('Failed to load resource:')) errors.push(message.text())
    })
    page.on('response', response => {
      if (response.status() >= 400 && ![401, 403].includes(response.status())) errors.push(`HTTP ${response.status()} ${new URL(response.url()).pathname}`)
    })
    page.on('requestfailed', request => {
      if (!request.url().includes('/live') && !request.failure()?.errorText.includes('ERR_ABORTED')) errors.push(new URL(request.url()).pathname)
    })
    const auth = await login(page)
    const tournament = (await read(page, '/api/tournaments')).find(item => item.name === 'Guttas Golf 2026')
    expect(tournament).toBeDefined()
    const setup = postgres({ actor: auth.user_id, tournament: tournament.id })
    try {
      await setup.command("UPDATE tournament_memberships SET role='admin' WHERE tournament_id=:'tournament'::uuid AND user_id=:'actor'::uuid;")
    } finally { setup.close() }
    const rounds = await read(page, `/api/tournaments/${tournament.id}/rounds`)
    const round = rounds[2]
    if (tournament.status === 'draft') await mutate(page, `/api/tournaments/${tournament.id}/start`, auth.csrf_token, { expected_tournament_updated_at: tournament.updated_at })
    if (round.status === 'draft') await mutate(page, `/api/rounds/${round.id}/open`, auth.csrf_token)
    const access = await read(page, `/api/rounds/${round.id}/score-access`)
    const owner = access.writable_owners[0]
    const path = `/api/rounds/${round.id}/scorecards/${owner.type}/${owner.id}/scoring`
    const url = `/score?${new URLSearchParams({ tournament: tournament.id, round: round.id, owner_type: owner.type, owner: owner.id, hole: '1', view: 'hole' })}`
    await page.goto(url)
    await expect(page.locator('.hole-entry')).toBeVisible()
    if (change === 'valid') {
      const initial = await read(page, path)
      const expected = (initial.holes[0].score?.gross_strokes ?? initial.holes[0].par) + 1
      await page.getByRole('button', { name: 'Legg til ett slag', exact: true }).click()
      await expect.poll(async () => (await read(page, path)).holes[0].score?.gross_strokes).toBe(expected)
      await expect(page.locator('.score-sync')).toHaveText('Lagret på serveren')
    }
    const before = await read(page, path)
    let armed = true, observedStatus = null
    await page.route(`**${path}`, async route => {
      if (!armed) return route.continue()
      armed = false
      const gate = postgres({ actor: auth.user_id, tournament: tournament.id })
      let observer
      try {
        const lock = change === 'logout'
          ? "SELECT id FROM user_sessions WHERE user_id=:'actor'::uuid AND revoked_at IS NULL FOR UPDATE;"
          : "SELECT user_id FROM tournament_memberships WHERE tournament_id=:'tournament'::uuid AND user_id=:'actor'::uuid FOR UPDATE;"
        const output = await gate.command(`BEGIN; SELECT pg_backend_pid(); ${lock}`)
        const pid = Number(output.split('\n')[0])
        expect(Number.isInteger(pid)).toBe(true)
        observer = postgres({ blocker: pid })
        // Forward the real GET unchanged; only its database authority row is held.
        const upstream = route.fetch()
        await expect.poll(async () => observer.command(
          "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND :blocker=ANY(pg_blocking_pids(pid)));",
        ), { timeout: 10000 }).toBe('t')
        const update = change === 'logout'
          ? "UPDATE user_sessions SET revoked_at=clock_timestamp() WHERE user_id=:'actor'::uuid AND revoked_at IS NULL;"
          : change === 'membership'
            ? "UPDATE tournament_memberships SET role='viewer' WHERE tournament_id=:'tournament'::uuid AND user_id=:'actor'::uuid;"
            : "UPDATE tournament_memberships SET role=role WHERE tournament_id=:'tournament'::uuid AND user_id=:'actor'::uuid;"
        await gate.command(`${update} COMMIT;`)
        const response = await upstream
        observedStatus = response.status()
        expect(response.headers()['cache-control']).toContain('no-store')
        if (change === 'valid') expect(await response.json()).toEqual(before)
        else expect((await response.json()).holes).toBeUndefined()
        await route.fulfill({ response })
      } finally {
        gate.close()
        observer?.close()
      }
    })
    await page.reload()
    const status = change === 'valid' ? 200 : change === 'logout' ? 401 : 403
    await expect.poll(() => observedStatus, { timeout: 30000 }).toBe(status)
    if (change === 'valid') {
      await expect(page.locator('.hole-entry')).toBeVisible()
      expect(await read(page, path)).toEqual(before)
    } else if (change === 'logout') {
      await expect(page.locator('.hole-entry')).toHaveCount(0)
      await expect(page.locator('.scorecard-summary')).toHaveCount(0)
    } else {
      // A viewer retains ordinary projected read access, but cannot score.
      await expect(page.getByText('Du kan se dette scorekortet, men ikke føre score for det.')).toBeVisible()
      await expect(page.getByRole('button', { name: 'Legg til ett slag', exact: true })).toHaveCount(0)
    }
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width)
    await page.screenshot({ path: `${artifacts}/${change}-${width}.png`, fullPage: true })
    expect(errors).toEqual([])
    checks.push({ change, width, status, privateResponse: true, errors })
    await context.close()
  }
  await writeFile(`${artifacts}/browser-checks.json`, JSON.stringify(checks, null, 2))
  console.log(JSON.stringify(checks, null, 2))
} finally {
  await browser.close()
}
