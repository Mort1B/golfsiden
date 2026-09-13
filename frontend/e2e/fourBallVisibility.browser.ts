import { expect, test } from '@playwright/test'
import { fourBallFixture } from './fourBallSupport'
import { offlineEvents, offlineLayout } from './offlineSupport'

test.skip(process.env.GOLF_FOUR_BALL_BROWSER !== '1', 'Requires disposable local API and Chrome')

test('four-ball setup retains manual teams and an eighteen-hole layout', async ({ page, browser }) => {
  const events = offlineEvents(page)
  const fixture = await fourBallFixture(page, browser, async (round, tournament) => {
    await page.goto(`/manage/tournaments/${tournament.id}?round=${round.id}#courses`)
    const course = page.locator(`#course-editor-${round.id}`)
    await expect(course.getByText('Four-ball krever et felles utslagssted med nøyaktig 18 hull.')).toBeVisible()
    await course.getByLabel('Registrer manuelt').check()
    await expect(course.getByLabel('Antall hull', { exact: true })).toHaveValue('18')
    await expect(course.getByLabel('Antall hull', { exact: true })).toHaveAttribute('readonly', '')
    await offlineLayout(page, 'four-ball-course-setup', '#courses')
    await page.goto(`/manage/tournaments/${tournament.id}?round=${round.id}#pairings`)
    await expect(page.getByText('Four-ball · lag og flighter', { exact: true })).toBeVisible()
    await expect(page.locator(`#pairing-editor-${round.id}`)).toBeVisible()
    await offlineLayout(page, 'four-ball-team-setup', '#pairings')
  })
  expect(fixture.round.handicap_allowance_percent).toBe(85)
  expect(fixture.card.partners).toHaveLength(2)
  expect(events.errors).toEqual([])
  expect(events.statuses.filter(status => ![401, 503].includes(status))).toEqual([])
})

test('member history keeps the hidden final nine private through live changes and tab return', async ({ page, browser }) => {
  const fixture = await fourBallFixture(page, browser)
  await fixture.save(1, fixture.firstId, { type: 'numeric', gross_strokes: 4 })
  await fixture.save(18, fixture.secondId, { type: 'numeric', gross_strokes: 3 })
  const memberContext = await browser.newContext()
  try {
    const member = await memberContext.newPage()
    const events = offlineEvents(member)
    const login = await member.request.post('http://127.0.0.1:5173/api/auth/login', {
      data: { username: fixture.partnerUsername, password: fixture.password },
    })
    expect(login.ok()).toBe(true)
    await member.goto(`/tournaments/${fixture.tournament.id}/rounds/${fixture.round.id}/scorecards/team/${fixture.teamId}?metric=net&view=summary`)
    const summary = member.getByRole('region', { name: 'Oppsummering av four-ball' })
    await expect(summary.getByText('Kun de første ni hullene vises. Bekreftelse og samlet fullføring er skjult.')).toBeVisible()
    await expect(summary.locator('li')).toHaveCount(9)
    const before = await summary.innerText()
    const refreshed = member.waitForResponse(response => new URL(response.url()).pathname === fixture.cardPath && response.ok())
    await fixture.save(18, fixture.secondId, { type: 'no_score' })
    await refreshed
    await expect(summary).toHaveText(before, { useInnerText: true })
    await expect(member.getByRole('button', { name: 'Hull 18 · par 4', exact: true })).toHaveCount(0)
    await offlineLayout(member, 'four-ball-hidden-history')
    const otherTab = await memberContext.newPage()
    await otherTab.goto('about:blank')
    await otherTab.bringToFront()
    await member.bringToFront()
    await member.evaluate(() => window.dispatchEvent(new PageTransitionEvent('pageshow', { persisted: true })))
    await expect(summary.locator('li')).toHaveCount(9)
    await expect(summary).toHaveText(before, { useInnerText: true })
    await member.getByRole('button', { name: 'Hull 9 · par 4', exact: true }).click()
    await expect(member.getByRole('button', { name: 'Neste hull', exact: true })).toBeDisabled()
    await expect(member.getByRole('heading', { name: 'Hull 9 · par 4 · indeks 9' })).toBeVisible()
    await otherTab.close()
    expect(events.errors).toEqual([])
    expect(events.statuses).toEqual([])
  } finally { await memberContext.close() }
})
