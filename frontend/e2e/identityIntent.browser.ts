import { test, expect } from '@playwright/test'
import { decodeAuthSession } from '../src/api/auth'
import { currentIsAnders, deferred, enterLogin, layout, observe, reviewCreation, unique } from './identitySupport'

test.skip(process.env.GOLF_IDENTITY_BROWSER !== '1', 'Requires disposable seeded API and GOLF_IDENTITY_BROWSER=1.')
test.use({ screenshot: 'off', trace: 'off' })

// These API-driver probes share the running page's real module/queue and cookie
// jar. They cover intents not simultaneously exposed by ordinary UI controls.
async function driver(page: import('@playwright/test').Page, operation: string) {
  await page.addScriptTag({ type: 'module', content: `
    import {api} from '/src/api/client.ts';
    import {waitForIdentity} from '/src/api/identityQueue.ts';
    document.documentElement.dataset.identityProbe='pending';
    void(async()=>{try{${operation};await waitForIdentity();document.documentElement.dataset.identityProbe='done'}
      catch{document.documentElement.dataset.identityProbe='failed'}})();
  ` })
  await expect(page.locator('html')).toHaveAttribute('data-identity-probe', 'pending')
}

test('queued logout clears a departed creation even while canonical identity is anonymous', async ({ page }) => {
  const events = observe(page)
  const committed = deferred<void>(), release = deferred<void>()
  let logoutRequests = 0
  page.on('request', request => { if (new URL(request.url()).pathname === '/api/auth/logout') logoutRequests++ })
  await page.route('**/api/onboarding/tournaments', async route => {
    const response = await route.fetch()
    expect(response.status()).toBe(201)
    committed.resolve()
    await release.promise
    await route.fulfill({ response })
  })
  try {
    await reviewCreation(page, unique())
    await page.getByRole('button', { name: 'Opprett turnering', exact: true }).click()
    await committed.promise
    await page.goBack()
    await driver(page, 'await api.logout()')
    expect(logoutRequests).toBe(0)
    release.resolve()
    await expect(page.locator('html')).toHaveAttribute('data-identity-probe', 'done')
    expect(logoutRequests).toBe(1)
    expect((await page.request.get('/api/auth/session')).status()).toBe(401)
    await expect(page.getByRole('heading', { name: 'Start golfturen her' })).toBeVisible()
    await layout(page, 320)
    expect(events.errors).toEqual([])
    expect(events.failures).toEqual([])
  } finally { release.resolve() }
})

test('same-user queued renewals preserve the latest cookie and canonical CSRF for a real write', async ({ page }) => {
  const events = observe(page)
  await page.goto('/')
  await page.getByRole('link', { name: 'Logg inn' }).click()
  await page.getByLabel('Brukernavn', { exact: true }).fill('anders')
  await page.getByLabel('Passord', { exact: true }).fill('golf-dev-2026')
  await page.getByRole('button', { name: 'Logg inn', exact: true }).click()
  await currentIsAnders(page)
  const initial = decodeAuthSession(await (await page.request.get('/api/auth/session')).json())
  const held = deferred<void>(), release = deferred<void>()
  let logins = 0
  await page.route('**/api/auth/login', async route => {
    logins++
    const response = await route.fetch()
    expect(response.status()).toBe(200)
    if (logins === 1) { held.resolve(); await release.promise }
    await route.fulfill({ response })
  })
  try {
    await driver(page, "const first=api.login('anders','golf-dev-2026');const second=api.login('anders','golf-dev-2026');await Promise.all([first,second])")
    await held.promise
    expect(logins).toBe(1)
    release.resolve()
    await expect(page.locator('html')).toHaveAttribute('data-identity-probe', 'done')
    expect(logins).toBe(2)
    const current = decodeAuthSession(await (await page.request.get('/api/auth/session')).json())
    expect(current.user_id === initial.user_id).toBe(true)
    expect(current.csrf_token !== initial.csrf_token).toBe(true)
    await page.getByRole('link', { name: 'Opprett ny turnering' }).click()
    await page.getByLabel('Turneringsnavn', { exact: true }).fill(`Fornyet sesjon ${unique()}`)
    await page.getByRole('button', { name: 'Neste', exact: true }).click()
    await page.getByRole('button', { name: 'Neste', exact: true }).click()
    const created = page.waitForResponse(response => new URL(response.url()).pathname === '/api/tournaments' && response.request().method() === 'POST')
    await page.getByRole('button', { name: 'Opprett turnering', exact: true }).click()
    const response = await created
    expect(response.status()).toBe(201)
    expect(response.request().headers()['x-csrf-token'] === current.csrf_token).toBe(true)
    await expect(page).toHaveURL(/\/manage\/tournaments\//)
    expect(events.errors).toEqual([])
    expect(events.failures).toEqual([])
  } finally { release.resolve() }
})

test('failed newer login stays reviewable after cookie reconciliation and can be retried deliberately', async ({ page }) => {
  const committed = deferred<void>(), release = deferred<void>()
  const creator = unique()
  await page.route('**/api/onboarding/tournaments', async route => {
    const response = await route.fetch()
    expect(response.status()).toBe(201)
    committed.resolve()
    await release.promise
    await route.fulfill({ response })
  })
  try {
    await reviewCreation(page, creator)
    await page.getByRole('button', { name: 'Opprett turnering', exact: true }).click()
    await committed.promise
    await enterLogin(page)
    await page.getByLabel('Passord', { exact: true }).fill('wrong password')
    await page.getByRole('button', { name: 'Logg inn', exact: true }).click()
    await expect(page.getByRole('button', { name: 'Logger inn...' })).toBeDisabled()
    release.resolve()
    await expect(page.getByRole('alert')).toBeVisible()
    await expect(page).toHaveURL(/\/login\?/)
    const actual = decodeAuthSession(await (await page.request.get('/api/auth/session')).json())
    expect(actual.username).toBe(creator)
    await expect(page.getByRole('alert')).toBeVisible()
    await page.getByLabel('Passord', { exact: true }).fill('golf-dev-2026')
    await page.getByRole('button', { name: 'Logg inn', exact: true }).click()
    await currentIsAnders(page)
  } finally { release.resolve() }
})
