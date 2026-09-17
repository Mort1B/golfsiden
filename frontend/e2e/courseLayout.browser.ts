import { test, expect, type Page } from '@playwright/test'
import { decodeObject } from '../src/api/decoder'
import { decodeAuthSession } from '../src/api/auth'
import { decodeTournament, decodeTournamentRounds } from '../src/api/tournaments/decoders'
import { presetResponse } from '../src/api/coursePresets.fixture'

test.skip(process.env.GOLF_COURSE_LAYOUT_BROWSER !== '1', 'Requires a disposable API and GOLF_COURSE_LAYOUT_BROWSER=1.')

async function inspectLayout(page: Page, label: string) {
  const picker = page.locator('.saved-course-picker:visible')
  for (const [width, height] of [[320, 600], [390, 844], [1280, 900]] as const) {
    await page.setViewportSize({ width, height })
    const alert = picker.getByRole('alert')
    await alert.scrollIntoViewIfNeeded()
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width)
    const box = await alert.boundingBox()
    expect(box).not.toBeNull()
    if (!box) throw new Error('Missing visible format error')
    expect(box.x).toBeGreaterThanOrEqual(0)
    expect(box.x + box.width).toBeLessThanOrEqual(width)
    for (const control of await picker.locator('button, select, summary').all()) {
      expect((await control.boundingBox())?.height).toBeGreaterThanOrEqual(44)
      await control.scrollIntoViewIfNeeded()
      await control.click({ trial: true })
    }
    await picker.screenshot({ path: `/tmp/l2-course-${label}-${width}.png` })
  }
}

for (const [format, code, label] of [
  ['four_ball_stroke_play', 'four_ball_requires_18_holes', 'Four-ball'],
  ['individual_stableford', 'stableford_requires_18_holes', 'Stableford'],
  ['singles_match_play', 'singles_match_requires_18_holes', 'Matchspill (singel)'],
] as const) {
  test(`${format} explains nine-hole rejection and saves an eighteen-hole correction`, async ({ page }) => {
    const stamp = `${Date.now()}_${Math.floor(Math.random() * 10000)}`
    const day = new Date(Date.now() + 86400000).toISOString().slice(0, 10)
    const created = await page.request.post('/api/onboarding/tournaments', { data: {
      creator: { account: { username: `layout_${stamp}`, password: 'layout-browser-password' }, player: { display_name: 'Baneadministrator', handicap_index: 18 } },
      tournament: { name: `Banevalg ${stamp}`, description: '', start_date: day, end_date: day, counted_rounds: format === 'singles_match_play' ? null : 1, mandatory_round_number: null },
      rounds: [{ round_number: 1, name: label, round_date: day, scoring_format: format }],
    } })
    expect(created.status(), await created.text()).toBe(201)
    const body = decodeObject(await created.json(), 'created')
    const auth = decodeAuthSession(body.session), tournament = decodeTournament(body.tournament)
    const rounds = decodeTournamentRounds(await (await page.request.get(`/api/tournaments/${tournament.id}/rounds`)).json(), tournament.id)
    const round = rounds[0], preset = presetResponse[0]
    if (!round || !preset) throw new Error('Missing round or saved layout')
    const nine = { ...preset, course_name: 'Ni hull med et svært langt banenavn for kontroll av feilmeldinger', tee: { ...preset.tee, holes: preset.tee.holes.slice(0, 9).map((hole, i) => ({ ...hole, stroke_index: i + 1 })) } }
    // Only the saved-course read is a fixture: rejection and correction use the real API.
    await page.route(`**/api/tournaments/${tournament.id}/course-presets`, route => route.fulfill({ json: [nine, ...presetResponse.slice(1)] }))
    const invalid = await page.request.put(`/api/rounds/${round.id}/course-configuration`, { headers: { 'x-csrf-token': auth.csrf_token }, data: {
      expected_round_updated_at: round.updated_at,
      selection: { source: 'manual', course_name: nine.course_name, location: null, tee: { ...nine.tee, holes: nine.tee.holes.map(({ par, stroke_index, distance }) => ({ par, stroke_index, distance })) } },
    } })
    expect(invalid.status()).toBe(409)
    expect(decodeObject(decodeObject(await invalid.json(), 'response').error, 'error').code).toBe(code)
    expect(await (await page.request.get(`/api/rounds/${round.id}`)).json()).toEqual(round)
    const errors: string[] = [], failed: string[] = []
    page.on('pageerror', error => errors.push(error.message))
    page.on('console', message => { if (message.type() === 'error') errors.push(message.text()) })
    page.on('response', response => { if (response.status() >= 400) failed.push(`${response.status()} ${new URL(response.url()).pathname}`) })
    await page.goto(`/manage/tournaments/${tournament.id}#courses`)
    const card = page.locator('.round-course-card').first()
    await card.getByRole('button', { name: 'Konfigurer', exact: true }).click()
    await card.getByRole('combobox', { name: 'Lagret bane', exact: true }).selectOption(nine.id)
    let saves = 0
    page.on('request', request => { if (request.method() === 'PUT' && request.url().endsWith('/course-configuration')) saves++ })
    await card.getByRole('button', { name: 'Bruk lagret bane på runden' }).click()
    await expect(card.getByRole('alert')).toHaveText(`${label} krever nøyaktig 18 hull. Velg et utslagssted med 18 hull.`)
    await expect(card.getByRole('combobox', { name: 'Lagret bane', exact: true })).toHaveValue(nine.id)
    await inspectLayout(page, format)
    expect(saves).toBe(0)
    const valid = presetResponse[1]
    if (!valid) throw new Error('Missing valid saved layout')
    await card.getByRole('combobox', { name: 'Lagret bane', exact: true }).selectOption(valid.id)
    const saved = page.waitForResponse(response => response.url().endsWith('/course-configuration') && response.request().method() === 'PUT')
    await card.getByRole('button', { name: 'Bruk lagret bane på runden' }).click()
    expect((await saved).status()).toBe(200)
    await expect(card.locator('.course-receipt')).toContainText(`er lagret med ${valid.course_name}`)
    await expect(card.getByRole('button', { name: 'Endre', exact: true })).toBeFocused()
    await card.getByRole('button', { name: 'Endre', exact: true }).click()
    await card.getByLabel('Registrer manuelt').check()
    await expect(card.getByLabel('Antall hull', { exact: true })).toHaveValue('18')
    await expect(card.getByLabel('Antall hull', { exact: true })).toHaveAttribute('readonly', '')
    expect(errors).toEqual([])
    expect(failed).toEqual([])
    expect(saves).toBe(1)
  })
}
