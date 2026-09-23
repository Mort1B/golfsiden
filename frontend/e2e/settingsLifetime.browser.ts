import { test, expect, type Page } from '@playwright/test'
import { routeWorkspace, trip } from './routeSplittingSupport'
import { round as initialRound, opening } from '../src/features/tournaments/lifecycle/__tests__/fixtures'
import type { Round } from '../src/api/types'
import { decodeObject, decodeNumber } from '../src/api/decoder'
const round: Round = { ...initialRound, scoring_format: 'individual_stableford' }
const url = `/manage/tournaments/${trip.id}#courses`
const settingPath = `/api/rounds/${round.id}/stableford/settings`
async function workspace(page: Page) {
 const f = await routeWorkspace(page)
 const state = { round: { ...round }, held: true, error: false, writes: 0, roundReads: 0 }
 let release: () => void = () => undefined
 const held = new Promise<void>(resolve => { release = resolve })
 await page.route('**/api/**', async route => {
  const path = new URL(route.request().url()).pathname
  if (path === `/api/tournaments/${trip.id}`) return route.fulfill({ json: { ...trip, status: 'draft' } })
  if (path === `/api/tournaments/${trip.id}/rounds`) { state.roundReads++; return route.fulfill({ json: [state.round] }) }
  if (path === `/api/rounds/${round.id}`) { state.roundReads++; return route.fulfill({ json: state.round }) }
  if (path === `/api/rounds/${round.id}/pairing-validation`) return route.fulfill({ json: opening })
  if (path === settingPath) {
   state.writes++
   const payload = decodeObject(route.request().postDataJSON(), 'settings')
   const captured = { ...state.round, handicap_allowance_percent: decodeNumber(payload.handicap_allowance_percent, 'allowance') }
   const fail = state.error
   if (state.held) await held
   if (fail) return route.fulfill({ status: 409, json: { error: { code: 'round_configuration_stale', message: 'Synthetic stale settings' } } })
   // Held responses model an earlier accepted write; later server state is independent.
   if (!state.held) state.round = captured
   return route.fulfill({ json: captured })
  }
  return route.fallback()
 })
 return { ...f, flow: state, release }
}
async function returnToPage(page: Page) {
 const response = page.waitForResponse(r => r.url().endsWith('/api/auth/session'))
 await page.evaluate(() => window.dispatchEvent(new PageTransitionEvent('pageshow', { persisted: true })))
 await response
}
async function renderSettled(page: Page) { await page.evaluate(() => new Promise<void>(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve())))) }
for (const width of [320, 390, 1280]) for (const error of [false, true]) test(`renewed session ignores late ${error ? 'conflict' : 'success'} at ${width}px`, async ({ page }) => {
 await page.setViewportSize({ width, height: width === 320 ? 600 : 900 })
 const f = await workspace(page)
 try {
  f.flow.error = error
  await page.goto(url)
  const field = page.getByLabel('Handicapandel (0–100 %)', { exact: true })
  await field.fill('50'); await page.getByRole('button', { name: 'Lagre Stableford-innstillinger', exact: true }).click()
  await expect.poll(() => f.flow.writes).toBe(1)
  await expect(page.locator('.stableford-settings').getByRole('button')).toHaveText('Lagrer …')
  f.flow.round = { ...round, handicap_allowance_percent: 80 }
  f.state.auth = { ...f.state.loginAs, csrf_token: 'renewed-synthetic-session' }
  await returnToPage(page)
  await expect(field).toHaveValue('80')
  await expect(page.getByRole('button', { name: 'Lagre Stableford-innstillinger', exact: true })).toBeEnabled()
  await field.fill('70')
  const reads = f.flow.roundReads
  const response = page.waitForResponse(r => new URL(r.url()).pathname === settingPath)
  f.release(); await (await response).finished(); await renderSettled(page)
  await expect(field).toHaveValue('70')
  await expect(page.locator('.stableford-settings').getByRole('status')).toHaveCount(0)
  await expect(page.locator('.stableford-settings').getByRole('alert')).toHaveCount(0)
  expect(f.flow.roundReads).toBe(reads)
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
  const save = page.getByRole('button', { name: 'Lagre Stableford-innstillinger', exact: true })
  expect((await save.boundingBox())?.height).toBeGreaterThanOrEqual(44)
  await save.click({ trial: true })
  await page.locator('.stableford-settings').screenshot({ path: `/tmp/golf-settings-lifetime/renewed-${error}-${width}.png` })
  if (error) { expect(f.errors).toEqual([`HTTP 409 ${settingPath}`]); f.errors.splice(0) }
  f.flow.error = false; f.flow.held = false
  await save.click()
  await expect(page.getByText('Stableford-innstillingene er lagret.', { exact: true })).toBeVisible()
  await expect(field).toHaveValue('70')
  expect(f.flow.writes).toBe(2)
 } finally { f.release(); await f.close() }
})
for (const departure of ['logout', 'account', 'navigation'] as const) test(`late settings success after ${departure} cannot restore the old receipt`, async ({ page }) => {
 const f = await workspace(page)
 try {
  await page.goto(url)
  await page.getByLabel('Handicapandel (0–100 %)', { exact: true }).fill('50')
  await page.getByRole('button', { name: 'Lagre Stableford-innstillinger', exact: true }).click()
  await expect.poll(() => f.flow.writes).toBe(1)
  f.flow.round = { ...round, handicap_allowance_percent: 80 }
  if (departure === 'logout') {
   await page.getByRole('button', { name: 'Logg ut', exact: true }).click()
   await expect(page.getByRole('heading', { name: 'Logg inn', exact: true })).toBeVisible()
  } else if (departure === 'account') {
   f.state.auth = { ...f.state.loginAs, user_id: '00000000-0000-0000-0000-000000000099', csrf_token: 'other-synthetic-session' }
   await returnToPage(page)
   await expect(page.getByLabel('Handicapandel (0–100 %)', { exact: true })).toHaveValue('80')
  } else {
   await page.getByRole('link', { name: 'Profil', exact: true }).click()
   await expect(page).toHaveURL('/profile')
  }
  const reads = f.flow.roundReads, response = page.waitForResponse(r => new URL(r.url()).pathname === settingPath)
  f.release(); await (await response).finished(); await renderSettled(page)
  expect(f.flow.roundReads).toBe(reads)
  await expect(page.getByText('Stableford-innstillingene er lagret.', { exact: true })).toHaveCount(0)
  if (departure === 'account') await expect(page.getByLabel('Handicapandel (0–100 %)', { exact: true })).toHaveValue('80')
 } finally { f.release(); await f.close() }
})
