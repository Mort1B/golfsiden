import { test, expect, type Page } from '@playwright/test'
import { decodeAuthSession } from '../src/api/auth'
import { decodeTournament, decodeTournamentList } from '../src/api/tournaments/decoders'
import { tieBoard, tieRounds, tieTournamentId } from '../src/api/leaderboards/__tests__/tieBreakFixtures'
import { tournament, session } from '../src/features/tournaments/lifecycle/__tests__/fixtures'
import { liveServer } from './returnLoadingSupport'
import type { TournamentLeaderboard } from '../src/api/types'

test.skip(process.env.GOLF_TIE_BREAK_BROWSER !== '1', 'Requires disposable seeded API and GOLF_TIE_BREAK_BROWSER=1.')

function observe(page: Page) {
  const errors: string[] = []; const failures: number[] = []; const network: string[] = []
  page.on('pageerror', error => errors.push(error.message))
  page.on('console', message => { if (message.type() === 'error' && !message.text().startsWith('Failed to load resource:')) errors.push(message.text()) })
  page.on('response', response => { if (response.status() >= 400) failures.push(response.status()) })
  page.on('requestfailed', request => network.push(`${new URL(request.url()).pathname}: ${request.failure()?.errorText}`))
  return { errors, failures, network }
}
async function layout(page: Page, state: string, selector: string) {
  for (const [width, height] of [[320, 600], [390, 844], [1280, 900]] as const) {
    await page.setViewportSize({ width, height })
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width)
    for (const control of await page.locator(`${selector} button:visible, ${selector} select:visible, ${selector} a:visible`).all()) {
      expect((await control.boundingBox())?.height).toBeGreaterThanOrEqual(44)
      if (await control.isEnabled()) {
        await control.evaluate(element => element.scrollIntoView({ block: 'center' }))
        await control.click({ trial: true })
      }
    }
    await page.locator(selector).screenshot({ path: `/tmp/golf-tie-break-${state}-${width}.png` })
  }
}

test('real settings save, pending, failure, stale reconciliation and permanent start freeze', async ({ page }) => {
  const loggedIn = await page.request.post('/api/auth/login', { data: { username: 'admin', password: 'golf-dev-2026' } })
  expect(loggedIn.status()).toBe(200)
  const auth = decodeAuthSession(await loggedIn.json())
  const trip = decodeTournamentList(await (await page.request.get('/api/tournaments')).json()).find(item => item.status === 'draft')
  if (!trip) throw new Error('Requires freshly seeded draft tournament')
  const path = `/api/tournaments/${trip.id}/counted-rounds`
  const events = observe(page)
  await page.goto(`/manage/tournaments/${trip.id}#settings`)
  const field = page.getByLabel('Ved lik totalscore sammenlagt')
  await expect(field).toBeEnabled()
  await expect(field).toHaveValue('shared_positions')
  await layout(page, 'settings-default', '#settings')
  await field.selectOption('final_round_score')
  let fail = true
  await page.route(`**${path}`, route => {
    if (fail) { fail = false; return route.fulfill({ status: 503, json: { error: { code: 'unavailable', message: 'Test: tjenesten er utilgjengelig' } } }) }
    return route.continue()
  })
  await page.getByRole('button', { name: 'Lagre valg', exact: true }).click()
  await expect(page.getByRole('alert')).toContainText('Test: tjenesten er utilgjengelig')
  await layout(page, 'settings-error', '#settings')
  await page.unroute(`**${path}`)
  let release: (() => void) | undefined
  const wait = new Promise<void>(resolve => { release = resolve })
  await page.route(`**${path}`, async route => { await wait; await route.continue() })
  await page.getByRole('button', { name: 'Prøv lagring igjen', exact: true }).click()
  await expect(field).toBeDisabled()
  await expect(page.getByRole('button', { name: 'Lagrer …', exact: true })).toBeDisabled()
  await layout(page, 'settings-pending', '#settings')
  release?.()
  await expect(page.locator('.counted-rounds-receipt')).toContainText('Siste runde, deretter delt plass')
  await layout(page, 'settings-saved', '#settings')
  await page.unroute(`**${path}`)
  expect(decodeTournament(await (await page.request.get(`/api/tournaments/${trip.id}`)).json()).tie_break_policy).toBe('final_round_score')
  // Commit another organizer's update immediately before sending the old timestamp.
  await field.selectOption('shared_positions')
  await page.route(`**${path}`, async route => {
    const current = decodeTournament(await (await page.request.get(`/api/tournaments/${trip.id}`)).json())
    const update = await page.request.patch(path, { headers: { 'x-csrf-token': auth.csrf_token }, data: {
      counted_rounds: current.counted_rounds === 1 ? 2 : 1, mandatory_round_id: current.mandatory_round_id,
      tie_break_policy: 'final_round_score', expected_tournament_updated_at: current.updated_at,
    } })
    expect(update.status()).toBe(200)
    await route.continue()
  })
  await page.getByRole('button', { name: 'Lagre valg', exact: true }).click()
  await expect(page.getByRole('alert')).toContainText('Turneringen ble endret et annet sted')
  await expect(field).toHaveValue('final_round_score')
  await layout(page, 'settings-stale', '#settings')
  await page.unroute(`**${path}`)
  const current = decodeTournament(await (await page.request.get(`/api/tournaments/${trip.id}`)).json())
  const started = await page.request.post(`/api/tournaments/${trip.id}/start`, { headers: { 'x-csrf-token': auth.csrf_token }, data: { expected_tournament_updated_at: current.updated_at } })
  expect(started.status()).toBe(200)
  await page.reload()
  await expect(page.getByText('Lik totalscore: Siste runde, deretter delt plass.', { exact: true })).toBeVisible()
  await expect(field).toHaveCount(0)
  await layout(page, 'settings-frozen', '#settings')
  expect(events.errors).toEqual([])
  expect(events.failures.every(status => status === 503 || status === 409)).toBe(true)
  expect(events.network.filter(value => !value.includes('net::ERR_ABORTED'))).toEqual([])
})

test('server-ranked gross/net ties, missing and hidden finals, loading/error and empty states', async ({ page }) => {
  const events = observe(page)
  const live = await liveServer()
  let mode: 'ready' | 'loading' | 'error' | 'missing' | 'hidden' | 'empty' = 'ready'
  let unblock: (() => void) | undefined
  const blocked = new Promise<void>(resolve => { unblock = resolve })
  const trip = { ...tournament, id: tieTournamentId, name: 'Turnering med et svært langt navn og tre like sammenlagtresultater', number_of_rounds: 2, tie_break_policy: 'final_round_score' }
  await page.addInitScript(({ url }) => {
    const Native = window.EventSource
    window.EventSource = class extends Native { constructor(_url: string | URL, init?: EventSourceInit) { super(url, init) } }
  }, { url: live.url })
  await page.route('**/api/**', async route => {
    const path = new URL(route.request().url()).pathname
    if (!path.startsWith('/api/')) return route.continue()
    if (path === '/api/auth/session') return route.fulfill({ json: session })
    if (path === '/api/tournaments') return route.fulfill({ json: [trip] })
    if (path === '/api/me/tournaments') return route.fulfill({ json: [{ tournament: trip, role: 'admin', player_id: session.player_id }] })
    if (path.endsWith('/rounds')) return route.fulfill({ json: tieRounds })
    if (path.includes('/leaderboards/')) {
      if (mode === 'loading') await blocked
      if (mode === 'error') return route.fulfill({ status: 503, json: { error: { code: 'unavailable', message: 'Test: resultatene kunne ikke hentes' } } })
      const metric = path.endsWith('/net') ? 'net' : 'gross'
      let board: TournamentLeaderboard = tieBoard(metric)
      if (mode === 'missing' || mode === 'hidden') {
        board = tieBoard(metric, 'shared_positions'); board.tie_break_policy = 'final_round_score'
        if (mode === 'hidden') {
          board.visibility = { mode: 'front_nine' }; board.included_round_ids = board.included_round_ids.slice(0, 1)
          board.entries = board.entries.map(entry => ({ ...entry, completed_rounds: 1, contributions: entry.contributions.slice(0, 1) }))
        } else {
          const first = board.entries[0]
          if (!first) throw new Error('Missing fixture')
          first.completed_rounds = 1; first.contributions.pop()
        }
      }
      if (mode === 'empty') board.entries = []
      return route.fulfill({ json: board })
    }
    throw new Error(`Unexpected route ${path}`)
  })
  const url = `/leaderboard?tournament=${tieTournamentId}&scope=tournament&metric=gross`
  try {
    await page.goto(url)
    await expect(page.locator('.leaderboard-tie-break')).toHaveCount(3)
    await expect(page.locator('.leaderboard-position')).toHaveText(['1', 'T2', 'T2'])
    await layout(page, 'gross-resolved', '.leaderboard-page')
    await page.getByRole('button', { name: 'Netto', exact: true }).click()
    await expect(page.locator('.leaderboard-position')).toHaveText(['T1', 'T1', '3'])
    await expect(page.locator('.leaderboard-tie-break').first()).toContainText('+1 netto')
    await layout(page, 'net-resolved', '.leaderboard-page')
    mode = 'missing'; await page.reload()
    await expect(page.locator('.leaderboard-position')).toHaveText(['T1', 'T1', 'T1'])
    await expect(page.locator('.leaderboard-tie-break')).toHaveCount(0)
    await layout(page, 'missing-final', '.leaderboard-page')
    mode = 'hidden'; await page.reload()
    await expect(page.getByText(/Finalen viser bare synlige resultater/)).toBeVisible()
    await expect(page.locator('.leaderboard-tie-break')).toHaveCount(0)
    await layout(page, 'hidden-final', '.leaderboard-page')
    mode = 'loading'; await page.reload()
    await expect(page.locator('.leaderboard-page [role=status]').first()).toBeVisible()
    await layout(page, 'loading', '.leaderboard-page')
    mode = 'error'; unblock?.()
    await expect(page.getByRole('alert')).toContainText('Test: resultatene kunne ikke hentes')
    await layout(page, 'error', '.leaderboard-page')
    mode = 'ready'; await page.getByRole('button', { name: 'Prøv igjen', exact: true }).click()
    await expect(page.locator('.leaderboard-tie-break')).toHaveCount(3)
    mode = 'empty'; await page.reload()
    await expect(page.getByText('Ingen spillere er registrert i turneringen')).toBeVisible()
    await layout(page, 'empty', '.leaderboard-page')
    expect(events.errors).toEqual([])
    expect(events.failures.every(status => status === 503)).toBe(true)
    expect(events.network.filter(value => !value.includes('net::ERR_ABORTED'))).toEqual([])
  } finally { await page.close(); await live.close() }
})
