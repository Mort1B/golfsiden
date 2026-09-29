import { test, expect, type Page } from '@playwright/test'
import { routeWorkspace, trip } from './routeSplittingSupport'
import { round, opening } from '../src/features/tournaments/lifecycle/__tests__/fixtures'
import { decodeObject, decodeArray, decodeString } from '../src/api/decoder'
import type { RoundPairings } from '../src/api/pairings'
const artifacts = '/tmp/golf-admin-lifetime-20260929'
type Kind = 'pairing' | 'start'
const pairingPath = `/api/rounds/${round.id}/pairings`, startPath = `/api/tournaments/${trip.id}/start`
const startReceipt = 'Turneringen er startet. Alle rundene er fortsatt i kladd.'
const initialPairings: RoundPairings = { round_id: round.id, tournament_id: trip.id, status: 'draft', scoring_format: 'individual_stroke_play',
  updated_at: trip.updated_at, active_entrants: [], inactive_entrants: [], teams: [], legacy_individual_groups: [],
  flights: [{ id: '00000000-0000-0000-0000-000000000010', name: 'Flight med et langt navn for golfvennene', starting_hole: null, tee_time: null,
    created_at: trip.created_at, updated_at: trip.updated_at, members: [] }],
}
async function workspace(page: Page) {
  const f = await routeWorkspace(page)
  const flow = { pairings: initialPairings, started: false, held: true, outcome: 'success', writes: 0, reads: 0, csrf: [] as (string | undefined)[] }
  let release: () => void = () => undefined
  const held = new Promise<void>(resolve => { release = resolve })
  await page.route('**/api/**', async route => {
    const request = route.request(), path = new URL(request.url()).pathname
    if (path === `/api/tournaments/${trip.id}`) { flow.reads++; return route.fulfill({ json: { ...trip, status: flow.started ? 'active' : 'draft' } }) }
    if (path === `/api/tournaments/${trip.id}/rounds`) return route.fulfill({ json: [round] })
    if (path === `/api/rounds/${round.id}`) return route.fulfill({ json: round })
    if (path === `/api/rounds/${round.id}/pairing-validation`) return route.fulfill({ json: opening })
    if (path === `/api/tournaments/${trip.id}/players`) return route.fulfill({ json: { handicap_correction: { state: 'editable' }, players: [{
      tournament_id: trip.id, player_id: '00000000-0000-0000-0000-000000000004', display_name: 'Deltaker med et langt navn fra golfklubben',
      player_active: true, tournament_handicap: 10, seed: null, status: 'active', created_at: trip.created_at, updated_at: trip.updated_at,
    }] } })
    if (path === pairingPath && request.method() === 'GET') { flow.reads++; return route.fulfill({ json: flow.pairings }) }
    if (path !== pairingPath && path !== startPath) return route.fallback()
    flow.writes++; flow.csrf.push(request.headers()['x-csrf-token'])
    const outcome = flow.outcome, wait = flow.held
    let saved = flow.pairings
    if (path === pairingPath) {
      const body = decodeObject(request.postDataJSON(), 'pairings')
      const names = decodeArray(body.flights, 'flights', value => decodeString(decodeObject(value, 'flight').name, 'name'))
      saved = { ...flow.pairings, flights: flow.pairings.flights.map((flight, index) => ({ ...flight, name: names[index] ?? flight.name })) }
    }
    if (wait) await held
    if (outcome !== 'success') return route.fulfill({ status: outcome === 'stale' ? 409 : 500,
      json: { error: { code: outcome === 'stale' ? path === pairingPath ? 'round_pairings_stale' : 'tournament_start_stale' : 'internal_error', message: 'Synthetic management error' } } })
    if (!wait) { if (path === pairingPath) flow.pairings = saved; else flow.started = true }
    return route.fulfill({ json: path === pairingPath ? saved : { ...trip, status: 'active' } })
  })
  return { ...f, flow, release }
}
const url = (kind: Kind) => `/manage/tournaments/${trip.id}${kind === 'pairing' ? `?round=${round.id}#pairings` : '#lifecycle'}`
const control = (page: Page, kind: Kind) => page.locator(kind === 'pairing' ? '.pairing-editor' : '.tournament-start-panel')
const button = (page: Page, kind: Kind) => control(page, kind).getByRole('button', { name: kind === 'pairing' ? 'Lagre hele oppsettet' : /^(Start turneringen|Prøv å starte igjen)$/ })
async function submit(page: Page, kind: Kind) {
  if (kind === 'pairing') await control(page, kind).getByLabel('Navn', { exact: true }).fill('Gammelt utkast')
  await button(page, kind).click()
}
async function returnToPage(page: Page) {
  const response = page.waitForResponse(r => r.url().endsWith('/api/auth/session'))
  await page.evaluate(() => window.dispatchEvent(new PageTransitionEvent('pageshow', { persisted: true })))
  await response
}
async function settle(page: Page) {
  await page.evaluate(() => new Promise<void>(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve()))))
}
for (const kind of ['pairing', 'start'] as const) for (const width of [320, 390, 1280]) for (const outcome of ['success', 'stale']) {
  test(`${kind} ignores late ${outcome} after session renewal at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: width === 320 ? 600 : 900 })
    const f = await workspace(page)
    try {
      f.flow.outcome = outcome
      await page.goto(url(kind)); await submit(page, kind)
      await expect.poll(() => f.flow.writes).toBe(1)
      f.state.auth = { ...f.state.loginAs, csrf_token: 'renewed-synthetic-session' }
      await returnToPage(page)
      if (kind === 'pairing') await control(page, kind).getByLabel('Navn', { exact: true }).fill('Nytt utkast i ny økt')
      await expect(button(page, kind)).toBeEnabled()
      const reads = f.flow.reads, path = kind === 'pairing' ? pairingPath : startPath
      const response = page.waitForResponse(r => new URL(r.url()).pathname === path && r.request().method() !== 'GET')
      f.release(); await (await response).finished(); await settle(page)
      expect(f.flow.reads).toBe(reads)
      await expect(control(page, kind).getByRole('alert')).toHaveCount(0)
      await expect(page.getByText(startReceipt)).toHaveCount(0)
      if (kind === 'pairing') await expect(control(page, kind).getByLabel('Navn', { exact: true })).toHaveValue('Nytt utkast i ny økt')
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
      expect((await button(page, kind).boundingBox())?.height).toBeGreaterThanOrEqual(44)
      await button(page, kind).click({ trial: true })
      await control(page, kind).screenshot({ path: `${artifacts}/${kind}-${outcome}-${width}.png` })
      if (outcome === 'stale') { expect(f.errors).toEqual([`HTTP 409 ${path}`]); f.errors.splice(0) }
      f.flow.outcome = 'success'; f.flow.held = false
      await button(page, kind).click()
      if (kind === 'pairing') await expect(control(page, kind).getByText('Synkronisert med serveren')).toBeVisible()
      else await expect(page.getByText(startReceipt)).toBeVisible()
      expect(f.flow.csrf[1]).toBe('renewed-synthetic-session')
    } finally { f.release(); await f.close() }
  })
}
for (const kind of ['pairing', 'start'] as const) for (const departure of ['logout', 'account', 'navigation']) {
  test(`${kind} ignores late success after ${departure}`, async ({ page }) => {
    const f = await workspace(page)
    try {
      await page.goto(url(kind)); await submit(page, kind)
      await expect.poll(() => f.flow.writes).toBe(1)
      if (departure === 'logout') {
        await page.getByRole('button', { name: 'Logg ut', exact: true }).click()
        await expect(page.getByRole('heading', { name: 'Logg inn', exact: true })).toBeVisible()
      } else if (departure === 'account') {
        f.state.auth = { ...f.state.loginAs, user_id: '00000000-0000-0000-0000-000000000099', csrf_token: 'other-synthetic-session' }
        await returnToPage(page)
        if (kind === 'pairing') await expect(control(page, kind).getByLabel('Navn', { exact: true })).toHaveValue(initialPairings.flights[0]?.name ?? '')
        else await expect(button(page, kind)).toBeEnabled()
      } else {
        await page.getByRole('link', { name: 'Profil', exact: true }).click(); await expect(page).toHaveURL('/profile')
      }
      const reads = f.flow.reads, path = kind === 'pairing' ? pairingPath : startPath
      const response = page.waitForResponse(r => new URL(r.url()).pathname === path && r.request().method() !== 'GET')
      f.release(); await (await response).finished(); await settle(page)
      expect(f.flow.reads).toBe(reads)
      await expect(page.getByText(startReceipt)).toHaveCount(0)
      if (departure === 'account' && kind === 'pairing') await expect(control(page, kind).getByLabel('Navn', { exact: true })).toHaveValue(initialPairings.flights[0]?.name ?? '')
    } finally { f.release(); await f.close() }
  })
}
for (const kind of ['pairing', 'start'] as const) test(`${kind} preserves explicit retry after a current-session failure`, async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 900 })
  const f = await workspace(page)
  try {
    f.flow.held = false; f.flow.outcome = 'failure'
    await page.goto(url(kind)); await submit(page, kind)
    await expect(control(page, kind).getByRole('alert')).toBeVisible()
    await control(page, kind).screenshot({ path: `${artifacts}/${kind}-error-390.png` })
    expect(f.errors).toEqual([`HTTP 500 ${kind === 'pairing' ? pairingPath : startPath}`]); f.errors.splice(0)
    f.flow.outcome = 'success'; await button(page, kind).click()
    if (kind === 'pairing') await expect(control(page, kind).getByText('Synkronisert med serveren')).toBeVisible()
    else await expect(page.getByText(startReceipt)).toBeVisible()
  } finally { f.release(); await f.close() }
})
