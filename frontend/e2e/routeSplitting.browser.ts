import { test, expect } from '@playwright/test'
import { routeWorkspace, routeLayout, profileChunk, trip, scoreUrl, matchListUrl, matchScoreUrl, matchCard, shareId, shareSecret } from './routeSplittingSupport'

for (const width of [320, 390, 1280]) {
  test(`direct and in-app account, management, score and match routes at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: width === 320 ? 600 : 900 })
    const f = await routeWorkspace(page)
    f.state.emptyRounds = true
    let release: () => void = () => undefined
    const held = new Promise<void>(resolve => { release = resolve })
    try {
      await page.route(profileChunk, async route => { await held; await route.continue() })
      await page.goto('/profile')
      await expect(page.getByRole('region', { name: 'Laster side' })).toBeVisible()
      await expect(page.getByRole('navigation', { name: 'Hovedmeny' })).toBeVisible()
      expect(f.paths).not.toContain('/api/me/profile')
      await routeLayout(page, 'loading')
      release()
      await expect(page.getByRole('heading', { name: 'Min profil' })).toBeVisible()
      await expect(page.getByLabel('Navn', { exact: true })).toHaveValue(f.state.auth?.display_name ?? '')
      await page.getByRole('link', { name: 'Gå til turneringsoversikten' }).click()
      await page.getByRole('link').filter({ has: page.getByRole('heading', { name: trip.name }) }).click()
      await expect(page.getByRole('link', { name: 'Åpne administrasjon' })).toBeVisible()
      await page.getByRole('link', { name: 'Åpne administrasjon' }).click()
      await expect(page.getByRole('navigation', { name: 'Administrasjonsområder' })).toBeVisible()
      await expect(page.getByText('Ingen runder å følge opp ennå.')).toBeVisible()
      await routeLayout(page, 'management-empty')
      await page.goto(`/manage/tournaments/${trip.id}#entrants`)
      await expect(page.getByRole('heading', { name: 'Deltakere', exact: true })).toBeVisible()
      f.state.emptyRounds = false
      await page.goto(scoreUrl)
      await expect(page.locator('#current-hole-heading')).toHaveText('8')
      await page.getByRole('button', { name: 'Neste', exact: true }).click()
      await expect(page.locator('#current-hole-heading')).toHaveText('9')
      await page.getByRole('combobox', { name: 'Hull', exact: true }).selectOption('8')
      await expect(page.locator('#current-hole-heading')).toHaveText('8')
      await page.goBack()
      await expect(page.locator('#current-hole-heading')).toHaveText('9')
      await page.goto(matchListUrl)
      await page.getByRole('link', { name: 'Les match', exact: true }).click()
      await expect(page.getByRole('heading', { name: 'Aksepterte rapporter' })).toBeVisible()
      await page.getByRole('link', { name: 'Alle matcher', exact: true }).click()
      await page.getByRole('link', { name: 'Før match', exact: true }).click()
      await expect(page.getByLabel(`Notat · ${matchCard.opponents[0].display_name}`, { exact: true })).toBeVisible()
      await routeLayout(page, 'match')
    } finally { release(); await f.close() }
  })

  test(`chunk recovery preserves URL and durable score queue at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 })
    const f = await routeWorkspace(page)
    try {
      await page.goto(scoreUrl)
      await page.getByRole('button', { name: /Registrer par/ }).click()
      await expect(page.locator('.pending-scores summary')).toContainText('(1)')
      f.allowedChunks.push(profileChunk)
      await page.route(profileChunk, route => route.abort('failed'))
      await page.getByRole('navigation', { name: 'Hovedmeny' }).getByRole('link', { name: 'Profil', exact: true }).click()
      await expect(page.getByRole('heading', { name: 'Siden kunne ikke lastes' })).toBeVisible()
      await expect(page.getByRole('link', { name: 'Lokale scoreendringer (1)' })).toBeVisible()
      await routeLayout(page, 'chunk-error')
      const reload = page.getByRole('button', { name: 'Last siden på nytt' })
      expect((await reload.boundingBox())?.height).toBeGreaterThanOrEqual(44)
      await reload.focus()
      await page.unroute(profileChunk)
      await page.keyboard.press('Enter')
      await expect(page.getByRole('heading', { name: 'Min profil' })).toBeVisible()
      await expect(page.getByRole('link', { name: 'Lokale scoreendringer (1)' })).toBeVisible()
      await page.getByRole('link', { name: 'Lokale scoreendringer (1)' }).click()
      await page.locator('.pending-scores summary').click()
      await expect(page.getByText('Lokalt: 4 slag')).toBeVisible()
    } finally { await f.close() }
  })

  test(`dirty match notes block lazy navigation and logout at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 })
    const f = await routeWorkspace(page)
    try {
      await page.goto(matchScoreUrl)
      const note = page.getByLabel(`Notat · ${matchCard.opponents[0].display_name}`, { exact: true })
      await note.fill('7')
      await expect(page.getByRole('button', { name: 'Logg ut', exact: true })).toBeDisabled()
      await page.getByRole('navigation', { name: 'Hovedmeny' }).getByRole('link', { name: 'Profil', exact: true }).click()
      await expect(page).toHaveURL(new RegExp(`${matchScoreUrl}$`))
      await expect(note).toHaveValue('7')
      expect(f.paths.filter(path => profileChunk.test(path))).toEqual([])
      await page.getByRole('button', { name: 'Forkast ulagrede notater', exact: true }).click()
      await page.getByRole('navigation', { name: 'Hovedmeny' }).getByRole('link', { name: 'Profil', exact: true }).click()
      await expect(page.getByRole('heading', { name: 'Min profil' })).toBeVisible()
    } finally { await f.close() }
  })

  test(`public chunk failure reload retains fragment and metric at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 })
    const f = await routeWorkspace(page)
    const chunk = /\/assets\/SharedResultsPage-[^/]+\.js$/
    const path = `/results/shared/${shareId}?metric=net#token=${shareSecret}`
    try {
      f.state.auth = null; f.allowedChunks.push(chunk)
      await page.route(chunk, route => route.abort('failed'))
      await page.goto(path)
      await expect(page.getByRole('button', { name: 'Last siden på nytt' })).toBeVisible()
      expect(f.paths.filter(p => p.startsWith('/api/') && p !== '/api/auth/session')).toEqual([])
      await page.unroute(chunk)
      await page.getByRole('button', { name: 'Last siden på nytt' }).click()
      await expect(page.getByRole('heading', { name: 'En lang turneringstittel for offentlig resultatdeling' })).toBeVisible()
      expect(new URL(page.url()).pathname + new URL(page.url()).search + new URL(page.url()).hash).toBe(path)
      expect(f.paths.filter(p => p.startsWith('/api/') && p !== '/api/auth/session')).toEqual([`/api/public/results/${shareId}`])
      await routeLayout(page, 'shared')
    } finally { await f.close() }
  })
}

test('late private module resolution cannot restore a logged-out account', async ({ page }) => {
  const f = await routeWorkspace(page)
  let release: () => void = () => undefined
  const held = new Promise<void>(resolve => { release = resolve })
  try {
    await page.route(profileChunk, async route => { await held; await route.continue() })
    await page.goto('/profile?section=details#name')
    await expect(page.getByRole('region', { name: 'Laster side' })).toBeVisible()
    await page.getByRole('button', { name: 'Logg ut', exact: true }).click()
    await expect(page.getByRole('heading', { name: 'Logg inn', exact: true })).toBeVisible()
    release()
    await expect(page.getByRole('heading', { name: 'Min profil' })).toHaveCount(0)
    expect(f.paths).not.toContain('/api/me/profile')
    f.state.loginAs = { ...f.state.loginAs, user_id: '00000000-0000-0000-0000-000000000099', username: 'new_account', display_name: 'Ny konto' }
    await page.getByLabel('Brukernavn', { exact: true }).fill('new_account')
    await page.getByLabel('Passord', { exact: true }).fill('synthetic-unused-password')
    await page.getByRole('button', { name: 'Logg inn', exact: true }).click()
    await expect(page.getByLabel('Navn', { exact: true })).toHaveValue('Ny konto')
    expect(new URL(page.url()).search + new URL(page.url()).hash).toBe('?section=details#name')
  } finally { release(); await f.close() }
})

test('loaded management route still denies non-admin and profile errors recover', async ({ page }) => {
  const f = await routeWorkspace(page)
  try {
    f.state.role = 'viewer'
    await page.goto(`/manage/tournaments/${trip.id}`)
    await expect(page.getByRole('heading', { name: 'Ingen tilgang' })).toBeVisible()
    expect(f.paths).not.toContain(`/api/tournaments/${trip.id}/players`)
    f.state.profileError = true
    await page.getByRole('navigation', { name: 'Hovedmeny' }).getByRole('link', { name: 'Profil', exact: true }).click()
    await expect(page.getByRole('alert')).toContainText('Profilen er midlertidig utilgjengelig')
    f.state.profileError = false
    await page.getByRole('button', { name: 'Prøv igjen', exact: true }).click()
    await expect(page.getByLabel('Navn', { exact: true })).toHaveValue(f.state.auth?.display_name ?? '')
  } finally { await f.close() }
})

test('failed device persistence keeps stroke edits behind the navigation guard', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 })
  const f = await routeWorkspace(page)
  try {
    await page.goto(scoreUrl)
    await expect(page.getByRole('button', { name: /Registrer par/ })).toBeEnabled()
    await page.evaluate(() => { IDBObjectStore.prototype.put = () => { throw new DOMException('Device storage unavailable', 'QuotaExceededError') } })
    await page.getByRole('button', { name: /Registrer par/ }).click()
    await expect(page.getByText('Kunne ikke lagre på denne enheten. Endringen er ikke trygt lagret. Prøv igjen eller forkast.', { exact: true })).toBeVisible()
    await expect(page.getByRole('button', { name: 'Logg ut', exact: true })).toBeDisabled()
    await page.getByRole('navigation', { name: 'Hovedmeny' }).getByRole('link', { name: 'Profil', exact: true }).click()
    await expect(page).toHaveURL(/\/score\?/)
    expect(f.paths.filter(path => profileChunk.test(path))).toEqual([])
    await page.getByRole('button', { name: /Forkast/ }).click()
    await page.getByRole('navigation', { name: 'Hovedmeny' }).getByRole('link', { name: 'Profil', exact: true }).click()
    await expect(page.getByRole('heading', { name: 'Min profil' })).toBeVisible()
  } finally { await f.close() }
})

test('a failed route stylesheet has the same explicit recovery as failed JavaScript', async ({ page }) => {
  const f = await routeWorkspace(page)
  const css = /\/assets\/ProfilePage-[^/]+\.css$/
  try {
    f.allowedChunks.push(css)
    await page.route(css, route => route.abort('failed'))
    await page.goto('/profile?section=details#name')
    await expect(page.getByRole('heading', { name: 'Siden kunne ikke lastes' })).toBeVisible()
    expect(f.paths).not.toContain('/api/me/profile')
    await page.unroute(css)
    await page.getByRole('button', { name: 'Last siden på nytt' }).click()
    await expect(page.getByLabel('Navn', { exact: true })).toHaveValue(f.state.auth?.display_name ?? '')
    expect(new URL(page.url()).search + new URL(page.url()).hash).toBe('?section=details#name')
  } finally { await f.close() }
})
