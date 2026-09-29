import { test, expect, type Page } from '@playwright/test'
import { routeWorkspace, trip } from './routeSplittingSupport'
import { round as initialRound, opening } from '../src/features/tournaments/lifecycle/__tests__/fixtures'
import { decodeBoolean, decodeObject } from '../src/api/decoder'
const round = { ...initialRound, course_id: '00000000-0000-0000-0000-000000000010', tee_id: '00000000-0000-0000-0000-000000000011' }
const url = `/manage/tournaments/${trip.id}#lifecycle`
const path = `/api/tournaments/${trip.id}/final-round-visibility`
const artifacts = '/tmp/golf-visibility-20260929'
async function workspace(page: Page) {
  const f = await routeWorkspace(page)
  const flow = { hidden: true, held: true, outcome: 'success', writes: 0, reads: 0, csrf: [] as (string | undefined)[], readHeld: false }
  let release: () => void = () => undefined, releaseRead: () => void = () => undefined
  const held = new Promise<void>(resolve => { release = resolve })
  const readHeld = new Promise<void>(resolve => { releaseRead = resolve })
  await page.route('**/api/**', async route => {
    const request = route.request(), requestPath = new URL(request.url()).pathname
    if (requestPath === `/api/tournaments/${trip.id}/rounds`) return route.fulfill({ json: [round] })
    if (requestPath === `/api/rounds/${round.id}`) return route.fulfill({ json: round })
    if (requestPath === `/api/rounds/${round.id}/pairing-validation`) return route.fulfill({ json: opening })
    if (requestPath !== path) return route.fallback()
    if (request.method() === 'GET') {
      flow.reads++
      if (flow.readHeld) await readHeld
      return route.fulfill({ json: { tournament_id: trip.id, back_nine_hidden: flow.hidden, visibility_updated_at: trip.updated_at } })
    }
    flow.writes++
    flow.csrf.push(request.headers()['x-csrf-token'])
    const hidden = decodeBoolean(decodeObject(request.postDataJSON(), 'visibility').back_nine_hidden, 'hidden')
    const outcome = flow.outcome, wait = flow.held
    if (wait) await held
    if (outcome !== 'success') return route.fulfill({ status: outcome === 'stale' ? 409 : 500,
      json: { error: { code: outcome === 'stale' ? 'final_round_visibility_stale' : 'internal_error', message: 'Synthetic visibility failure' } } })
    // A held response is an earlier accepted write, separate from later server state.
    if (!wait) flow.hidden = hidden
    return route.fulfill({ json: { tournament_id: trip.id, back_nine_hidden: hidden, visibility_updated_at: trip.updated_at } })
  })
  return { ...f, flow, release, releaseRead }
}
async function returnToPage(page: Page) {
  const response = page.waitForResponse(r => r.url().endsWith('/api/auth/session'))
  await page.evaluate(() => window.dispatchEvent(new PageTransitionEvent('pageshow', { persisted: true })))
  await response
}
async function settle(page: Page) {
  await page.evaluate(() => new Promise<void>(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve()))))
}
for (const width of [320, 390, 1280]) for (const outcome of ['success', 'stale']) test(`visibility ignores late ${outcome} after session renewal at ${width}px`, async ({ page }) => {
  await page.setViewportSize({ width, height: width === 320 ? 600 : 900 })
  const f = await workspace(page)
  try {
    f.flow.outcome = outcome
    await page.goto(url)
    const control = page.locator('.final-visibility-control'), toggle = control.getByRole('switch')
    await toggle.click()
    await expect.poll(() => f.flow.writes).toBe(1)
    await expect(toggle).toBeDisabled()
    f.state.auth = { ...f.state.loginAs, csrf_token: 'renewed-synthetic-session' }
    await returnToPage(page)
    await expect(toggle).toBeEnabled()
    await expect(toggle).not.toBeChecked()
    const reads = f.flow.reads, response = page.waitForResponse(r => new URL(r.url()).pathname === path && r.request().method() === 'PATCH')
    f.release(); await (await response).finished(); await settle(page)
    await expect(toggle).not.toBeChecked()
    await expect(control.getByText(/Serverstatusen er bekreftet/)).toHaveCount(0)
    await expect(control.getByRole('alert')).toHaveCount(0)
    expect(f.flow.reads).toBe(reads)
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
    const target = control.locator('.final-visibility-switch')
    expect((await target.boundingBox())?.height).toBeGreaterThanOrEqual(44)
    await toggle.click({ trial: true })
    await control.screenshot({ path: `${artifacts}/renewed-${outcome}-${width}.png` })
    if (outcome === 'stale') { expect(f.errors).toEqual([`HTTP 409 ${path}`]); f.errors.splice(0) }
    f.flow.outcome = 'success'; f.flow.held = false
    await toggle.click()
    await expect(control.getByText('Hull 10–18 er frigitt. Serverstatusen er bekreftet.')).toBeVisible()
    await expect(toggle).toBeChecked()
    expect(f.flow.csrf[1]).toBe('renewed-synthetic-session')
    await toggle.click()
    await expect(control.getByText('Hull 10–18 er skjult igjen. Serverstatusen er bekreftet.')).toBeVisible()
    await expect(toggle).not.toBeChecked()
  } finally { f.release(); f.releaseRead(); await f.close() }
})
for (const departure of ['logout', 'account', 'navigation'] as const) test(`visibility ignores late success after ${departure}`, async ({ page }) => {
  const f = await workspace(page)
  try {
    await page.goto(url)
    await page.getByRole('switch', { name: 'Frigi hull 10–18' }).click()
    await expect.poll(() => f.flow.writes).toBe(1)
    if (departure === 'logout') {
      await page.getByRole('button', { name: 'Logg ut', exact: true }).click()
      await expect(page.getByRole('heading', { name: 'Logg inn', exact: true })).toBeVisible()
    } else if (departure === 'account') {
      f.state.auth = { ...f.state.loginAs, user_id: '00000000-0000-0000-0000-000000000099', csrf_token: 'other-synthetic-session' }
      await returnToPage(page)
      await expect(page.getByRole('switch')).toBeEnabled()
    } else {
      await page.getByRole('link', { name: 'Profil', exact: true }).click()
      await expect(page).toHaveURL('/profile')
    }
    const reads = f.flow.reads, response = page.waitForResponse(r => new URL(r.url()).pathname === path && r.request().method() === 'PATCH')
    f.release(); await (await response).finished(); await settle(page)
    expect(f.flow.reads).toBe(reads)
    await expect(page.getByText(/Serverstatusen er bekreftet/)).toHaveCount(0)
    if (departure === 'account') await expect(page.getByRole('switch')).not.toBeChecked()
  } finally { f.release(); f.releaseRead(); await f.close() }
})
test('visibility loading and failed save retain explicit retry at phone width', async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 600 })
  const f = await workspace(page)
  try {
    f.flow.readHeld = true
    await page.goto(url)
    const control = page.locator('.final-visibility-control')
    await expect(control.getByText('Henter synlighetsstatus …')).toBeVisible()
    f.releaseRead()
    await expect(control.getByRole('switch')).toBeEnabled()
    f.flow.held = false; f.flow.outcome = 'failure'
    await control.getByRole('switch').click()
    await expect(control.getByRole('alert')).toContainText('Synthetic visibility failure')
    await control.screenshot({ path: `${artifacts}/error-320.png` })
    expect(f.errors).toEqual([`HTTP 500 ${path}`]); f.errors.splice(0)
    f.flow.outcome = 'success'
    await control.getByRole('button', { name: 'Prøv lagring igjen' }).click()
    await expect(control.getByText('Hull 10–18 er frigitt. Serverstatusen er bekreftet.')).toBeVisible()
    await expect(control.getByRole('switch')).toBeChecked()
  } finally { f.release(); f.releaseRead(); await f.close() }
})
