import { expect, test } from '@playwright/test'
import { routeWorkspace, shareId, shareSecret, matchScoreUrl } from './routeSplittingSupport'
import { layout, reachable } from './iphoneSupport'

test('public entry and onboarding forms fit and keep input across rotation', async ({ page }, info) => {
  const f = await routeWorkspace(page)
  f.state.auth = null
  async function visit(path: string) {
    const session = page.waitForResponse(response => new URL(response.url()).pathname === '/api/auth/session')
    await page.goto(path)
    await (await session).finished()
  }
  try {
    await visit('/')
    await expect(page.locator('.home-page')).toBeVisible()
    await layout(page, info, 'home', '.home-page')
    await visit('/login')
    await page.getByLabel('Brukernavn', { exact: true }).fill('mobil_spiller')
    await layout(page, info, 'login', '.sign-in-page')
    await expect(page.getByLabel('Brukernavn', { exact: true })).toHaveValue('mobil_spiller')
    await page.setViewportSize({ width: 393, height: 360 })
    await reachable(page, '.sign-in-panel input[type=password]')
    await reachable(page, '.sign-in-panel button')
    await visit('/create')
    await page.getByLabel('Turneringsnavn', { exact: true }).fill('Golfturen med et langt turneringsnavn for alle vennene')
    await layout(page, info, 'onboarding-tournament', '.onboarding-page')
    await expect(page.getByLabel('Turneringsnavn', { exact: true })).toHaveValue('Golfturen med et langt turneringsnavn for alle vennene')
    await page.getByRole('button', { name: 'Neste', exact: true }).click()
    await layout(page, info, 'onboarding-rounds', '.onboarding-page')
    await page.getByRole('button', { name: 'Neste', exact: true }).click()
    await layout(page, info, 'onboarding-creator', '.onboarding-page')
    await visit(`/join/${shareId}`)
    await expect(page.locator('.join-page')).toBeVisible()
    await layout(page, info, 'join-missing-link', '.join-page')
    await page.route(`**/api/invitations/${shareId}/preview`, route => route.fulfill({ json: {
      tournament: { id: shareId, name: 'Golfturen med et langt navn for venner fra hele Norge', start_date: '2099-06-01', end_date: '2099-06-04' },
      invitation: { expires_at: '2099-06-05T12:00:00Z' },
    } }))
    await visit('/login')
    await visit(`/join/${shareId}#token=${shareSecret}`)
    await expect(page.locator('.invitation-form').first()).toBeVisible()
    await layout(page, info, 'join-form', '.join-page')
  } finally { await f.close() }
})

test('profile loading, error, empty, long content and enlarged text stay usable', async ({ page }, info) => {
  const f = await routeWorkspace(page)
  let release = () => {}
  const gate = new Promise<void>(resolve => { release = resolve })
  try {
    await page.route('**/api/me/profile', async route => { await gate; await route.fallback() })
    await page.goto('/profile')
    await expect(page.getByText('Laster …')).toBeVisible()
    await layout(page, info, 'profile-loading', '.profile-page')
    f.state.profileError = true
    release()
    await expect(page.getByText('Profilen er midlertidig utilgjengelig')).toBeVisible()
    await layout(page, info, 'profile-error', '.profile-page')
    f.state.profileError = false
    f.state.membership = false
    await page.getByRole('button', { name: 'Prøv igjen' }).click()
    await expect(page.locator('.profile-page')).not.toContainText('Profilen er midlertidig utilgjengelig')
    await page.reload()
    await expect(page.locator('.profile-page .list-card')).toHaveCount(0)
    await layout(page, info, 'profile-empty', '.profile-page')
    await page.setViewportSize({ width: 393, height: 852 })
    await page.evaluate(() => { document.documentElement.style.fontSize = '200%' })
    await expect(page.getByRole('navigation', { name: 'Hovedmeny' })).toBeVisible()
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(393)
    for (const label of await page.locator('.nav-link span').all()) {
      expect(await label.evaluate(el => el.scrollWidth <= el.clientWidth)).toBe(true)
      expect(await label.evaluate(el => getComputedStyle(el).textOverflow)).not.toBe('ellipsis')
    }
    await reachable(page, '.profile-disclosure summary')
    await page.evaluate(() => scrollTo(0, 0))
    await page.screenshot({ path: `/tmp/iphone-${info.project.name}-profile-text-200.png` })
    await page.evaluate(() => { document.documentElement.style.fontSize = '' })
  } finally { release(); await f.close() }
})

test('public results and match scoring fit both orientations', async ({ page }, info) => {
  const f = await routeWorkspace(page)
  try {
    await page.goto(`/results/shared/${shareId}#token=${shareSecret}`)
    await expect(page.locator('.public-standings')).toBeVisible()
    await layout(page, info, 'public-results', '.public-results-page')
    await page.goto(matchScoreUrl)
    await expect(page.locator('.match-page')).toBeVisible()
    await layout(page, info, 'match-scoring', '.match-page')
  } finally { await f.close() }
})
