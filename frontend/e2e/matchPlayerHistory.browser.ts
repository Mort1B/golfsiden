import { test, expect } from '@playwright/test'
import { decodeObject, decodeString } from '../src/api/decoder'
import { decodePlayerListing } from '../src/api/matchPlay/decoders'
import { matchFixture, noOverflow } from './matchSupport'

test.skip(process.env.GOLF_MATCH_BROWSER !== '1', 'Requires disposable match API')
for (const width of [320, 390, 1280]) test(`filtered real-API final history at ${width}px`, async ({ page, browser }) => {
  const f = await matchFixture(page, browser)
  for (let hole = 1; hole <= 18; hole++) await f.command({ type: 'report', event: { type: 'hole', hole_number: hole, outcome: 'halved', basis: { type: 'agreed_halve', play_begun: true, mutual_agreement: true } } })
  await f.command({ type: 'confirm', result_agreed_or_awarded: true })
  const context = await browser.newContext({ viewport: { width, height: width === 320 ? 600 : 900 } })
  try {
    const member = await context.newPage(), errors: string[] = []
    member.on('pageerror', error => errors.push(error.message))
    member.on('console', message => { if (message.type() === 'error') errors.push(message.text()) })
    member.on('response', response => { if (response.status() >= 400) errors.push(`HTTP ${response.status()}`) })
    member.on('requestfailed', request => { if (request.failure()?.errorText !== 'net::ERR_ABORTED') errors.push(request.failure()?.errorText ?? 'failed') })
    expect((await member.request.post('http://127.0.0.1:5173/api/auth/login', { data: { username: f.secondUsername, password: f.password } })).ok()).toBe(true)
    const path = `/api/rounds/${f.round.id}/match-play/matches`
    const response = member.waitForResponse(r => new URL(r.url()).pathname === path && new URL(r.url()).searchParams.get('player_id') === f.secondId)
    await member.goto(`http://127.0.0.1:5173/tournaments/${f.tournament.id}/results/players/${f.secondId}?metric=net`)
    const wire = await response
    expect(wire.headers()['cache-control']).toContain('no-store')
    const hidden = decodePlayerListing(await wire.json(), f.round.id, f.secondId)
    expect(hidden.matches).toHaveLength(1)
    expect(hidden.matches[0]?.confirmed).toBeNull()
    expect(hidden.matches[0]?.half_points).toBeNull()
    expect(hidden.matches[0]?.holes).toHaveLength(9)
    expect(hidden.matches[0]?.finish).toBeNull()
    await expect(member.getByText('Fullføring og poeng er skjult til finalen frigis.', { exact: true })).toBeVisible()
    await noOverflow(member)
    await member.screenshot({ path: `/tmp/filter-real-hidden-${width}.png`, fullPage: true })
    const releasedResponse = member.waitForResponse(r => new URL(r.url()).pathname === path && new URL(r.url()).searchParams.get('player_id') === f.secondId && r.request().method() === 'GET')
    const visibility = decodeObject(await (await page.request.get(`/api/tournaments/${f.tournament.id}/final-round-visibility`)).json(), 'visibility')
    await f.mutate(`/api/tournaments/${f.tournament.id}/final-round-visibility`, { back_nine_hidden: false, expected_visibility_updated_at: decodeString(visibility.visibility_updated_at, 'version') }, 'PATCH')
    // An initial stream-open refresh can precede the release event; assert the
    // eventual DOM and then read the same endpoint to verify authoritative data.
    await releasedResponse
    await expect(member.getByText('Bekreftet resultat', { exact: true })).toBeVisible()
    const released = decodePlayerListing(await (await member.request.get(`http://127.0.0.1:5173${path}?player_id=${f.secondId}`)).json(), f.round.id, f.secondId)
    expect(released.matches[0]?.confirmed).toBe(true)
    expect(released.matches[0]?.holes).toHaveLength(18)
    expect(released.matches[0]?.half_points).toEqual([1, 1])
    await noOverflow(member)
    await member.screenshot({ path: `/tmp/filter-real-released-${width}.png`, fullPage: true })
    const returned = member.waitForResponse(r => new URL(r.url()).pathname === path && new URL(r.url()).searchParams.get('player_id') === f.secondId)
    await member.evaluate(() => window.dispatchEvent(new PageTransitionEvent('pageshow', { persisted: true })))
    expect(decodePlayerListing(await (await returned).json(), f.round.id, f.secondId).matches[0]?.confirmed).toBe(true)
    await expect(member.getByText('Bekreftet resultat', { exact: true })).toBeVisible()
    expect(errors).toEqual([])
  } finally { await context.close() }
})
