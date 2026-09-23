import { test, expect, type Page } from '@playwright/test'
import { routeWorkspace, matchCard, matchRound, matchScoreUrl } from './routeSplittingSupport'
import { noOverflow } from './matchSupport'
async function returnToPage(page: Page, reconnect = false) {
  const response = page.waitForResponse(r => r.url().endsWith('/api/auth/session'))
  await page.evaluate(online => { if (online) window.dispatchEvent(new Event('online')); window.dispatchEvent(new PageTransitionEvent('pageshow', { persisted: true })) }, reconnect)
  await response
}
async function guards(page: Page) {
  await expect(page.getByRole('button', { name: 'Logg ut', exact: true })).toBeDisabled()
  expect(await page.evaluate(() => { const event = new Event('beforeunload', { cancelable: true }); window.dispatchEvent(event); return event.defaultPrevented })).toBe(true)
  await page.getByRole('link', { name: 'Profil', exact: true }).click()
  await expect(page).toHaveURL(matchScoreUrl)
}
for (const width of [320, 390, 1280]) for (const failed of [false, true]) test(`same-account ${failed ? 'failed-save' : 'unsaved'} match input survives at ${width}px`, async ({ page }) => {
  await page.setViewportSize({ width, height: width === 320 ? 600 : 900 })
  const f = await routeWorkspace(page)
  try {
    await page.goto(matchScoreUrl)
    const input = page.getByLabel(`Notat · ${matchCard.opponents[0].display_name}`, { exact: true })
    await page.getByRole('combobox', { name: 'Hull', exact: true }).selectOption('3')
    await input.fill('7')
    if (failed) {
      await page.evaluate(() => {
        const put = IDBObjectStore.prototype.put
        IDBObjectStore.prototype.put = function(value, key) {
          if (this.name === 'matches') { IDBObjectStore.prototype.put = put; throw new DOMException('Synthetic one-write failure', 'QuotaExceededError') }
          return key === undefined ? put.call(this, value) : put.call(this, value, key)
        }
      })
      await page.getByRole('button', { name: 'Lagre notat', exact: true }).first().click()
      await expect(page.getByRole('alert')).toBeVisible()
    }
    await returnToPage(page)
    await expect(input).toHaveValue('7')
    f.state.auth = { ...f.state.loginAs, csrf_token: 'synthetic-replacement' }
    await returnToPage(page)
    await expect(input).toHaveValue('7')
    await expect(page.getByRole('combobox', { name: 'Hull', exact: true })).toHaveValue('3')
    if (failed) await expect(page.getByRole('alert')).toBeVisible()
    await guards(page)
    await noOverflow(page)
    await page.screenshot({ path: `/tmp/golf-match-retention/retained-${failed}-${width}.png`, fullPage: true })
    await page.getByRole('button', { name: 'Forkast ulagrede notater', exact: true }).click()
    await expect(page.getByRole('button', { name: 'Logg ut', exact: true })).toBeEnabled()
    await expect(input).toHaveValue('')
  } finally { await f.close() }
})
test('same-account denial retains only local recovery and account departure clears it', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 })
  const f = await routeWorkspace(page)
  try {
    await page.goto(matchScoreUrl)
    const input = page.getByLabel(`Notat · ${matchCard.opponents[0].display_name}`, { exact: true })
    await input.fill('7')
    f.state.membership = false
    f.state.auth = { ...f.state.loginAs, csrf_token: 'synthetic-denied-replacement' }
    await returnToPage(page)
    await expect(page.getByText('Lokalt notat 1: 7', { exact: true })).toBeVisible()
    await expect(input).toHaveCount(0)
    await expect(page.getByRole('button', { name: 'Rapporter avtalt resultat', exact: true })).toHaveCount(0)
    await guards(page)
    await noOverflow(page)
    await page.screenshot({ path: '/tmp/golf-match-retention/denied-390.png', fullPage: true })
    // An externally ended session must clear transient state despite local navigation guards.
    f.state.auth = null
    await page.route('**/api/auth/session', route => route.fulfill({ status: 401, json: { error: { code: 'unauthorized', message: 'Logg inn på nytt' } } }))
    await returnToPage(page)
    await expect(page.getByRole('heading', { name: 'Logg inn', exact: true })).toBeVisible()
    await page.unroute('**/api/auth/session')
    expect(f.errors.every(error => error === 'HTTP 401 /api/auth/session')).toBe(true)
    f.errors.splice(0)
    f.state.membership = true
    await page.getByLabel('Brukernavn', { exact: true }).fill('synthetic')
    await page.getByLabel('Passord', { exact: true }).fill('synthetic-unused-password')
    await page.getByRole('button', { name: 'Logg inn', exact: true }).click()
    await expect(input).toHaveValue('')
    await expect(page.getByRole('button', { name: 'Logg ut', exact: true })).toBeEnabled()
  } finally { await f.close() }
})
test('committed local queue survives a same-account replacement', async ({ page }) => {
  const f = await routeWorkspace(page)
  try {
    await page.goto(matchScoreUrl)
    const input = page.getByLabel(`Notat · ${matchCard.opponents[0].display_name}`, { exact: true })
    await expect(input).toBeEnabled()
    await page.evaluate(() => { Object.defineProperty(navigator, 'onLine', { configurable: true, get: () => false }); window.dispatchEvent(new Event('offline')) })
    await input.fill('7')
    await page.getByRole('button', { name: 'Lagre notat', exact: true }).first().click()
    const pending = page.getByRole('region', { name: 'Lokale matchendringer' })
    await expect(pending).toContainText('7 slag på hull 1')
    await page.route('**/commands', route => route.fulfill({ status: 503, json: { error: { code: 'unavailable', message: 'Synthetic delivery outage' } } }))
    await page.evaluate(() => { Object.defineProperty(navigator, 'onLine', { configurable: true, get: () => true }) })
    f.state.auth = { ...f.state.loginAs, csrf_token: 'synthetic-durable-replacement' }
    await returnToPage(page, true)
    await expect(pending).toContainText('7 slag på hull 1')
    await expect(page.getByRole('button', { name: 'Forkast ulagrede notater', exact: true })).toHaveCount(0)
    expect(f.errors.every(error => error === `HTTP 503 /api/rounds/${matchRound.id}/match-play/matches/${matchCard.match_id}/commands`)).toBe(true)
    f.errors.splice(0)
    // Stop delivery timers before inspecting the shared observer's final state.
    await page.evaluate(() => { Object.defineProperty(navigator, 'onLine', { configurable: true, get: () => false }); window.dispatchEvent(new Event('offline')) })
  } finally { await f.close() }
})

for (const suffix of ['/score/', '/SCORE']) test(`dirty notes block navigation on accepted route variant ${suffix}`, async ({ page }) => {
 const f = await routeWorkspace(page)
 try {
  const url = matchScoreUrl.replace(/\/score$/, suffix)
  await page.goto(url)
  await page.getByLabel(`Notat · ${matchCard.opponents[0].display_name}`, { exact: true }).fill('7')
  await page.getByRole('link', { name: 'Profil', exact: true }).click()
  await expect(page).toHaveURL(url)
  await page.getByRole('button', { name: 'Forkast ulagrede notater', exact: true }).click()
  await page.getByRole('link', { name: 'Profil', exact: true }).click()
  await expect(page).toHaveURL('/profile')
 } finally { await f.close() }
})
