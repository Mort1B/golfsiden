import { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'
import { resolve } from 'node:path'
import { writeFileSync } from 'node:fs'
import { strict as assert } from 'node:assert'
import { startFixture } from './server.mjs'
import { scenarios, tournamentId } from './fixtures.mjs'
const root = fileURLToPath(new URL('../../../', import.meta.url))
const { chromium } = createRequire(resolve(root, 'frontend/package.json'))('playwright')
const fixture = await startFixture(root, ''), browser = await chromium.launch({ channel: 'chrome', headless: true })
const evidence = []
try {
  const scenario = scenarios.find(s => s.matches === 24 && s.rounds === 3)
  for (const width of [320, 390, 1280]) {
    const state = fixture.begin(scenario), context = await browser.newContext({ viewport: { width, height: 900 } }), page = await context.newPage(), errors = []
    page.on('pageerror', e => errors.push(e.message))
    page.on('console', m => { if (m.type() === 'error') errors.push(m.text()) })
    page.on('requestfailed', r => { if (r.failure()?.errorText !== 'net::ERR_ABORTED') errors.push(r.failure()?.errorText) })
    page.on('response', r => { if (r.status() >= 400) errors.push(`HTTP ${r.status()}`) })
    await page.goto(`${fixture.base}/tournaments/${tournamentId}/match-results`)
    await page.waitForFunction(() => document.querySelectorAll('.match-list article').length === 72)
    const deadline = Date.now() + 15000
    while (!state.openers.length) { if (Date.now() > deadline) throw new Error('No stream'); await new Promise(r => setTimeout(r, 20)) }
    fixture.open()
    await page.waitForFunction(() => [...document.querySelectorAll('.match-list article h3')].length === 72 && [...document.querySelectorAll('.match-list article h3')].every(n => n.textContent.includes('[epoch 1]')))
    const rows = [], lists = () => state.requests.filter(r => r.path.endsWith('/matches'))
    async function navigate(name, action, cards, expectedRequests) {
      const before = lists().length
      await action()
      await page.waitForFunction(cards => document.querySelectorAll('.match-list article').length === cards, cards)
      await page.evaluate(() => new Promise(done => requestAnimationFrame(() => requestAnimationFrame(done))))
      const requests = lists().slice(before)
      assert.equal(requests.length, expectedRequests, name)
      assert.ok(requests.every(r => r.finished && !r.aborted))
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false)
      rows.push({ name, requests, cards })
    }
    const otherUrl = await page.getByRole('link', { name: 'Matchhistorikk', exact: true }).nth(2).getAttribute('href')
    await navigate('all to first player', () => page.getByRole('link', { name: 'Matchhistorikk', exact: true }).first().click(), 3, 3)
    // There is no direct other-player link in a selected view. Drive a client
    // URL transition explicitly, without reloading the document/cache.
    await navigate('player to uncached different player via URL transition', () => page.evaluate(url => { history.pushState(null, '', url); dispatchEvent(new PopStateEvent('popstate')) }, otherUrl), 3, 3)
    await navigate('player to fresh all', () => page.getByRole('link', { name: 'Alle spillere', exact: true }).click(), 72, 0)
    await navigate('all to fresh different player', () => page.getByRole('link', { name: 'Matchhistorikk', exact: true }).nth(2).click(), 3, 0)
    await navigate('different player to fresh all', () => page.getByRole('link', { name: 'Alle spillere', exact: true }).click(), 72, 0)
    await navigate('all to fresh first player', () => page.getByRole('link', { name: 'Matchhistorikk', exact: true }).first().click(), 3, 0)
    assert.deepEqual(errors, []); assert.deepEqual(state.errors, [])
    evidence.push({ width, rows, errors }); await context.close()
    console.log(`navigation ${width}px passed`)
  }
} finally { await browser.close(); await fixture.close() }
writeFileSync(process.argv[2] ?? '/tmp/history-filter-navigation.json', JSON.stringify(evidence, null, 2) + '\n')
