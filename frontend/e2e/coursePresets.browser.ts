import { test, expect, type Page } from '@playwright/test'
import { suppliedCourses, presetResponse } from '../src/api/coursePresets.fixture'

test.skip(process.env.GOLF_COURSE_PRESETS_BROWSER !== '1', 'Requires disposable seed and GOLF_COURSE_PRESETS_BROWSER=1.')
const trip = '00000000-0000-0000-0000-000000002001'
const path = `/api/tournaments/${trip}/course-presets`
async function login(page: Page) {
  await page.goto('/login')
  await page.getByLabel('Brukernavn', { exact: true }).fill('admin')
  await page.getByLabel('Passord', { exact: true }).fill('golf-dev-2026')
  await page.getByRole('button', { name: 'Logg inn', exact: true }).click()
  await expect(page).not.toHaveURL(/\/login/)
}
async function open(page: Page, index = 0) {
  await page.goto('/tournaments')
  await page.goto(`/manage/tournaments/${trip}#courses`)
  const card = page.locator('.round-course-card').nth(index)
  await card.getByRole('button', { name: 'Endre', exact: true }).click()
  return card
}
async function layout(page: Page, state: string) {
  const picker = page.locator('.saved-course-picker:visible')
  for (const width of [320, 390, 1280]) {
    await page.setViewportSize({ width, height: 900 })
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width)
    for (const control of await picker.locator('button, select, summary').all()) {
      expect((await control.boundingBox())?.height).toBeGreaterThanOrEqual(44)
    }
    await picker.screenshot({ path: `/tmp/golf-course-presets-${state}-${width}.png` })
  }
}
test('exact supplied layouts can be inspected and saved on independent rounds', async ({ page }) => {
  await login(page)
  const errors: string[] = []
  const failed: string[] = []
  page.on('pageerror', (e) => errors.push(e.message))
  page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()) })
  page.on('response', (r) => { if (r.status() >= 400) failed.push(`${r.status()} ${new URL(r.url()).pathname}`) })
  expect(await (await page.request.get(path)).json()).toEqual(presetResponse)
  const savedIds: string[] = []
  for (const [i, course] of suppliedCourses.entries()) {
    const card = await open(page, i)
    await card.getByRole('combobox', { name: 'Lagret bane', exact: true }).selectOption(course.id)
    await expect(card.getByText('Red Tees · Herre · 18 hull · Par 72', { exact: true })).toBeVisible()
    await expect(card.getByText(`Baneverdi ${course.r.toLocaleString('nb-NO')} · Slope ${course.s}`, { exact: true })).toBeVisible()
    await card.getByText('Vis par og slagindeks for alle hull', { exact: true }).click()
    const rows = card.locator('tbody tr')
    await expect(rows).toHaveCount(18)
    for (let h = 0; h < 18; h++) expect(await rows.nth(h).locator('td').allTextContents()).toEqual([String(h + 1), String(course.p[h]), String(course.i[h])])
    if (i === 0) await layout(page, 'populated')
    const response = page.waitForResponse((r) => r.url().includes('/course-configuration') && r.request().method() === 'PUT')
    await card.getByRole('button', { name: 'Bruk lagret bane på runden' }).click()
    const saved = await response
    expect(saved.status()).toBe(200)
    const data = await saved.json()
    expect(data.course_name).toBe(course.n)
    expect(data.tee_name).toBe('Red Tees')
    expect(data.number_of_holes).toBe(18)
    expect(data.course_id).not.toBe(course.id)
    savedIds.push(data.course_id)
    await expect(card.locator('.course-receipt')).toContainText(`er lagret med ${course.n} · Red Tees.`)
    await expect(card.getByRole('button', { name: 'Endre', exact: true })).toBeFocused()
  }
  expect(new Set(savedIds).size).toBe(3)
  await page.reload()
  for (const [i, c] of suppliedCourses.entries()) await expect(page.locator('.round-course-card').nth(i).locator('.round-course-summary')).toContainText(c.n)
  expect(await (await page.request.get(path)).json()).toEqual(presetResponse)
  expect(errors).toEqual([])
  expect(failed).toEqual([])
})
test('loading, empty, unavailable, retry and long-content states remain usable', async ({ page }) => {
  await login(page)
  let mode = 'loading'
  let release!: () => void
  const pending = new Promise<void>((r) => { release = r })
  await page.route(`**${path}`, async (route) => {
    if (mode === 'loading') await pending
    if (mode === 'error') return route.fulfill({ status: 503, json: { error: { code: 'unavailable', message: 'Preset test unavailable' } } })
    const rows = mode === 'empty' ? [] : presetResponse.map((p) => ({ ...p, course_name: p.course_name.repeat(6) }))
    return route.fulfill({ json: rows })
  })
  await open(page)
  await expect(page.locator('.saved-course-picker:visible').getByRole('status')).toBeVisible()
  await layout(page, 'loading')
  mode = 'empty'; release()
  await expect(page.getByText('Ingen lagrede baner er tilgjengelige.').filter({ visible: true })).toBeVisible()
  await layout(page, 'empty')
  mode = 'error'
  await open(page)
  await expect(page.locator('.saved-course-picker:visible').getByRole('alert')).toBeVisible({ timeout: 20000 })
  await expect(page.locator('.saved-course-picker:visible').getByRole('button', { name: 'Bruk lagret bane på runden' })).toBeDisabled()
  await layout(page, 'error')
  mode = 'long'
  await page.locator('.saved-course-picker:visible').getByRole('button', { name: 'Prøv igjen' }).click()
  const first = suppliedCourses[0]
  if (!first) throw new Error('Missing expected preset')
  await page.locator('.saved-course-picker:visible').getByRole('combobox', { name: 'Lagret bane', exact: true }).selectOption(first.id)
  await layout(page, 'long')
})
