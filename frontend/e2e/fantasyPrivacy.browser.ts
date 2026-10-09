import { expect, test } from '@playwright/test'
import { decodeAuthSession } from '../src/api/auth'
import { decodeClaimReceipt } from '../src/api/playerClaimDecoders'
import { decodeArray, decodeObject, decodeUuid } from '../src/api/decoder'
import { fantasyFixture, fantasyLayouts } from './fantasySupport'

test.skip(process.env.GOLF_FANTASY_BROWSER !== '1', 'Requires disposable local API and Chrome.')
test('Fantasy protects other drafts and hidden results across score changes, release, re-hide and return', async ({ page, browser }) => {
  const f = await fantasyFixture(page), final = f.rounds[1], golfer = f.players[0], claimed = f.players[1]
  if (!final || !golfer || !claimed) throw new Error('Missing fixture identities')
  await f.mutate(`/api/tournaments/${f.tournament.id}/fantasy`, { enabled: true }, 'PUT')
  await f.mutate(`/api/tournaments/${f.tournament.id}/fantasy/entry`)
  const claim = decodeClaimReceipt(await (await f.mutate(`/api/tournaments/${f.tournament.id}/players/${claimed}/claim`)).json())
  const context = await browser.newContext({ baseURL: 'http://127.0.0.1:5173' })
  try {
    const member = await context.newPage(), errors: string[] = []
    member.on('pageerror', error => errors.push(error.message))
    const registration = await member.request.post(`/api/player-claims/${claim.claim_id}/register`, { data: {
      token: claim.token, account: { username: `fantasy_member_${Date.now()}`, password: 'fantasy-private-browser-fixture' },
    } })
    expect(registration.status()).toBe(201)
    const auth = decodeAuthSession(decodeObject(await registration.json(), 'registration').session)
    const entry = await member.request.post(`/api/tournaments/${f.tournament.id}/fantasy/entry`, { headers: { 'x-csrf-token': auth.csrf_token } })
    expect(entry.status()).toBe(204)
    for (const round of f.rounds) {
      const path = `/api/tournaments/${f.tournament.id}/fantasy/rounds/${round.id}/lineup`
      await f.mutate(path, { request_id: crypto.randomUUID(), expected_revision: 0, picks: f.players, captain: golfer }, 'PUT')
      const saved = await member.request.put(path, { headers: { 'x-csrf-token': auth.csrf_token }, data: {
        request_id: crypto.randomUUID(), expected_revision: 0, picks: f.players, captain: claimed,
      } })
      expect(saved.ok()).toBe(true)
    }
    await member.goto(f.url)
    await expect(member.getByRole('heading', { name: 'Fantasy', exact: true })).toBeVisible()
    await expect(member.getByRole('heading', { name: 'Fantasy-oppsett' })).toHaveCount(0)
    await member.getByRole('button', { name: 'Poengtavler', exact: true }).click()
    await member.locator('.fantasy-standings button').filter({ hasText: f.names[0] }).click()
    await expect(member.getByText('Andre deltakeres valg er private fram til fristen.')).toHaveCount(2)
    await expect(member.locator('.fantasy-detail .fantasy-contributions')).toHaveCount(0)
    await f.prepare()
    await f.mutate(`/api/rounds/${final.id}/open`)
    const card = decodeObject(await (await page.request.get(`/api/rounds/${final.id}/scorecards/player/${golfer}/scoring`)).json(), 'card')
    const holes = decodeArray(card.holes, 'holes', (v, p) => decodeUuid(decodeObject(v, p).hole_id, `${p}.id`))
    const firstHole = holes[0], lastHole = holes[17]
    if (!firstHole || !lastHole) throw new Error('Missing holes')
    const save = (hole_id: string, gross_strokes: number) => f.mutate(`/api/rounds/${final.id}/scores`, {
      owner: { type: 'player', id: golfer }, hole_id, gross_strokes,
    }, 'PUT')
    await save(firstHole, 4); await save(lastHole, 3)
    await member.getByRole('button', { name: 'Spillerpoeng', exact: true }).click()
    await member.locator('.fantasy-standings button').filter({ hasText: f.names[0] }).click()
    const last = member.locator('.fantasy-holes li').filter({ hasText: 'Hull 18 · par 4' })
    await expect(last).toContainText('Skjult')
    await expect(last).not.toContainText('netto')
    const previous = await last.innerText()
    const updated = member.waitForResponse(r => r.url().includes(`/fantasy/results/golfers/${golfer}`) && r.status() === 200)
    await save(lastHole, 10); await updated
    await expect(last).toHaveText(previous, { useInnerText: true })
    await fantasyLayouts(member, 'hidden')
    async function visibility(hidden: boolean) {
      const path = `/api/tournaments/${f.tournament.id}/final-round-visibility`
      const current = decodeObject(await (await page.request.get(path)).json(), 'visibility')
      await f.mutate(path, { back_nine_hidden: hidden, expected_visibility_updated_at: current.visibility_updated_at }, 'PATCH')
    }
    await visibility(false)
    // Visibility clearing intentionally removes all private Fantasy projections and detail state.
    await member.getByRole('button', { name: 'Spillerpoeng', exact: true }).click()
    await member.locator('.fantasy-standings button').filter({ hasText: f.names[0] }).click()
    await expect(last).toContainText('netto 10')
    await visibility(true)
    await member.getByRole('button', { name: 'Spillerpoeng', exact: true }).click()
    await member.locator('.fantasy-standings button').filter({ hasText: f.names[0] }).click()
    await expect(last).toContainText('Skjult')
    const other = await context.newPage(); await other.goto('about:blank'); await other.bringToFront(); await member.bringToFront()
    await member.evaluate(() => window.dispatchEvent(new PageTransitionEvent('pageshow', { persisted: true })))
    await expect(last).toContainText('Skjult'); await expect(last).not.toContainText('netto')
    await other.close()
    await member.getByRole('button', { name: 'Logg ut' }).click()
    await expect(member.locator('.fantasy-page')).toHaveCount(0)
    expect(errors).toEqual([])
  } finally { await context.close() }
})
