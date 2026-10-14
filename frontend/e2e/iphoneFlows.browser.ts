import { expect, test } from '@playwright/test'
import { offlineFixture } from './offlineSupport'
import { stablefordFixture } from './stablefordSupport'
import { fourBallFixture } from './fourBallSupport'
import { at, releaseFixture } from './fantasyReleaseSupport'
import { layout, observe, reachable } from './iphoneSupport'

test.skip(process.env.GOLF_IPHONE_API !== '1', 'Requires disposable local API/PostgreSQL')

for (const format of ['stroke', 'team', 'four-ball', 'stableford'] as const) test(`${format} scorer and summary fit and retain the current hole`, async ({ page, browser }, info) => {
  const f = format === 'stableford' ? await stablefordFixture(page) : format === 'four-ball' ? await fourBallFixture(page, browser) : await offlineFixture(page, format === 'team' ? 'team_scramble' : 'individual_stroke_play')
  const events = observe(page)
  await page.goto(f.url(8))
  await expect(page.getByRole('button', { name: /Registrer par/ }).first()).toBeEnabled()
  await layout(page, info, `${format}-hole`, '.score-page')
  await expect(page).toHaveURL(/hole=8/)
  await page.goto(f.url(8, 'summary'))
  await expect(page.locator('.card-review')).toBeVisible()
  await layout(page, info, `${format}-summary`, '.score-page')
  await page.setViewportSize({ width: 393, height: 360 })
  await reachable(page, '.score-page button:visible:enabled')
  expect((await f.read()).holes_scored).toBe(0)
  events.check()
})

test('Fantasy retains draft through rotation and fits copy review, presets, boards and rules', async ({ page }, info) => {
  const f = await releaseFixture(page, ['team_scramble', 'team_scramble'], [0, 0, 0, 0])
  const events = observe(page)
  const writes: string[] = []
  page.on('request', request => { if (request.method() === 'PUT' && request.url().endsWith('/lineup')) writes.push(request.url()) })
  await page.goto(f.url)
  for (const name of f.names) await page.getByRole('checkbox', { name, exact: true }).check()
  const captain = page.getByLabel('Kaptein · doble poeng')
  await captain.selectOption(at(f.players, 1))
  await layout(page, info, 'fantasy-draft', '.fantasy-page')
  await expect(captain).toHaveValue(at(f.players, 1))
  for (const name of f.names) await expect(page.getByRole('checkbox', { name, exact: true })).toBeChecked()
  expect(writes).toEqual([])
  await page.getByRole('button', { name: 'Kopier fireren til andre åpne runder' }).click()
  await layout(page, info, 'fantasy-copy', '.fantasy-page')
  await page.getByRole('button', { name: 'Forbered valgte runder' }).click()
  await layout(page, info, 'fantasy-presets', '.fantasy-page')
  await page.getByRole('button', { name: 'Lagre gjennomgåtte utkast (2)' }).click()
  await expect(page.getByText(/Lagringen er satt på pause|Alle gjennomgåtte utkast er lagret/)).toBeVisible()
  const refresh = page.getByRole('button', { name: 'Prøv oppdatering igjen', exact: true })
  if (await refresh.isVisible()) {
    await layout(page, info, 'fantasy-refresh-recovery', '.fantasy-page')
    await refresh.click()
    await page.getByRole('button', { name: 'Lagre gjennomgåtte utkast (1)' }).click()
  }
  await expect(page.getByText(/Alle gjennomgåtte utkast er lagret/)).toBeVisible()
  expect(writes).toHaveLength(2)
  await page.getByRole('button', { name: 'Poengtavler', exact: true }).click()
  await layout(page, info, 'fantasy-boards', '.fantasy-page')
  await page.getByText('Regler og poeng', { exact: true }).click()
  await layout(page, info, 'fantasy-rules', '.fantasy-page')
  events.check()
})

test('completed round awards, standings and history fit; form drafts survive rotation', async ({ page }, info) => {
  const f = await offlineFixture(page)
  for (let hole = 1; hole <= 18; hole++) await f.save(hole, 4)
  await f.mutate(`/api/rounds/${f.round.id}/scorecards/player/${f.owner.id}/confirm`)
  await f.mutate(`/api/rounds/${f.round.id}/complete`)
  const events = observe(page)
  await page.goto(`/manage/tournaments/${f.tournament.id}?round=${f.round.id}#lifecycle`)
  await expect(page.getByLabel('Vinner av Closest to pin')).toBeVisible()
  await page.getByLabel('Vinner av Closest to pin').selectOption(f.owner.id)
  await page.getByLabel('Begrunnelse for Closest to pin').fill('Målt etter runden med lang begrunnelse for mobilvisning')
  await layout(page, info, 'management-awards', '.tournament-management-page')
  await expect(page.getByLabel('Vinner av Closest to pin')).toHaveValue(f.owner.id)
  await expect(page.getByLabel('Begrunnelse for Closest to pin')).toHaveValue('Målt etter runden med lang begrunnelse for mobilvisning')
  await page.setViewportSize({ width: 393, height: 360 })
  await reachable(page, '.round-awards fieldset:first-of-type textarea')
  await page.getByRole('button', { name: 'Lagre Closest to pin', exact: true }).click()
  await expect(page.getByText('Innsendingen er bekreftet. Gjeldende utmerkelser er oppdatert.')).toBeVisible()
  await page.goto(`/leaderboard?tournament=${f.tournament.id}&scope=round&round=${f.round.id}&metric=net`)
  await expect(page.locator('.standings-section')).toBeVisible()
  await layout(page, info, 'standings', '.standings-section')
  await page.goto(`/tournaments/${f.tournament.id}/results/players/${f.owner.id}?metric=net`)
  await expect(page.locator('.player-history')).toBeVisible()
  await layout(page, info, 'history', '.player-history')
  events.check()
})
